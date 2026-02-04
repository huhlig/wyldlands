//
// Copyright 2025-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! State-driven input handler for telnet connections
//!
//! This module handles input based on the current session state,
//! providing a clean separation between connection handling and state logic.

use crate::context::ServerContext;
use crate::properties::{BANNER_LOGIN_DEFAULT, BANNER_WELCOME_DEFAULT};
use crate::server::{InputMode, ProtocolAdapter};
use crate::session::{AuthenticatedState, NewAccountState, SessionState, UnauthenticatedState};
use termionix_server::{terminal_word_unwrap, terminal_word_wrap};
use tracing::instrument;
use uuid::Uuid;
use wyldlands_common::gateway::GatewayProperty;

/// Editor input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Insert mode - new text is inserted at cursor position
    Insert,
    /// Overwrite mode - new text replaces existing text at cursor position
    Overwrite,
}

/// State-driven handler for telnet sessions
pub struct StateHandler {
    session_id: Uuid,
    context: ServerContext,
    address: String,

    // Cached session state to avoid repeated lookups
    cached_state: SessionState,

    // Temporary storage for multi-step flows
    temp_username: Option<String>,
    temp_password: Option<String>,
    temp_display_name: Option<String>,
    temp_email: Option<String>,
    temp_discord: Option<String>,
    temp_timezone: Option<String>,

    // Editor state
    editor_cursor_position: usize,
    editor_terminal_width: usize,
    editor_mode: EditorMode,
}

impl StateHandler {
    /// Create a new state handler
    pub fn new(session_id: Uuid, context: ServerContext, address: String) -> Self {
        Self {
            session_id,
            context,
            address,
            cached_state: SessionState::Unauthenticated(UnauthenticatedState::Welcome),
            temp_username: None,
            temp_password: None,
            temp_display_name: None,
            temp_email: None,
            temp_discord: None,
            temp_timezone: None,
            editor_cursor_position: 0,
            editor_terminal_width: 80,
            editor_mode: EditorMode::Insert,
        }
    }

    /// Check if the current state is an editing state
    pub fn is_editing(&self) -> bool {
        self.cached_state.is_editing()
    }

    /// Process input based on current session state
    #[instrument(skip(self, adapter))]
    pub async fn process_input(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        
        // Fast path for the most common case: authenticated playing state
        // This avoids match overhead and function call for the hot path
        if matches!(self.cached_state, SessionState::Authenticated(AuthenticatedState::Playing)) {
            let result = self.send_command_to_server(input).await;
            let duration = start.elapsed();
            tracing::debug!(
                session_id = %self.session_id,
                state = "playing",
                duration_ms = %duration.as_millis(),
                "process_input completed (fast path)"
            );
            return result;
        }

        // Use cached state instead of looking up session every time
        let result = match &self.cached_state {
            SessionState::Unauthenticated(substate) => {
                self.handle_unauthenticated(adapter, *substate, input).await
            }
            SessionState::Authenticated(substate) => {
                self.handle_authenticated(adapter, substate.clone(), input).await
            }
            SessionState::Disconnected => Err("Session is disconnected".to_string()),
        };
        
        let duration = start.elapsed();
        tracing::debug!(
            session_id = %self.session_id,
            state = %self.cached_state.to_metric_str(),
            duration_ms = %duration.as_millis(),
            success = %result.is_ok(),
            "process_input completed"
        );
        
        result
    }

    /// Fast path: Send command directly to server (hot path optimization)
    #[inline]
    async fn send_command_to_server(&self, input: String) -> Result<(), String> {
        // Skip blank inputs - server sends output directly to client now
        if input.trim().is_empty() {
            return Ok(());
        }

        let rpc_client = self.context.rpc_client();
        if let Some(mut client) = rpc_client.session_client().await {
            let request = wyldlands_common::proto::SendInputRequest {
                session_id: self.session_id.to_string(),
                command: input,
            };

            client.send_input(request).await
                .map(|_| ())
                .map_err(|e| format!("Failed to send command: {}", e))
        } else {
            Err("Server not connected".to_string())
        }
    }

    /// Send the appropriate prompt based on current state
    pub async fn send_prompt(&mut self, adapter: &mut dyn ProtocolAdapter) -> Result<(), String> {
        // Use cached state instead of looking up session
        // Update input mode based on session state
        if self.cached_state.is_editing() {
            adapter.set_input_mode(InputMode::Character);
        } else {
            adapter.set_input_mode(InputMode::Line);
        }

        match &self.cached_state {
            SessionState::Unauthenticated(substate) => {
                self.send_unauthenticated_prompt(adapter, *substate).await
            }
            SessionState::Authenticated(AuthenticatedState::Playing) => {
                adapter.send_text("> ").await.map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }
            SessionState::Authenticated(AuthenticatedState::Editing { title, .. }) => {
                let mode_indicator = match self.editor_mode {
                    EditorMode::Insert => "INS",
                    EditorMode::Overwrite => "OVR",
                };
                adapter
                    .send_text(&format!("[Editing: {} - {}] ", title, mode_indicator))
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }
            SessionState::Disconnected => {
                Ok(()) // No prompt for disconnected state
            }
        }
    }

    /// Handle input in unauthenticated state
    #[instrument(skip(self, adapter))]
    async fn handle_unauthenticated(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        substate: UnauthenticatedState,
        input: String,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        
        let result = match substate {
            UnauthenticatedState::Welcome => {
                // Welcome state auto-advances, shouldn't receive input
                self.transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::Username,
                ))
                .await
            }

            UnauthenticatedState::Username => self.handle_username_input(adapter, input).await,

            UnauthenticatedState::Password => self.handle_password_input(adapter, input).await,

            UnauthenticatedState::NewAccount(new_state) => {
                self.handle_new_account(adapter, new_state, input).await
            }
        };
        
        let duration = start.elapsed();
        tracing::info!(
            session_id = %self.session_id,
            substate = ?substate,
            duration_ms = %duration.as_millis(),
            success = %result.is_ok(),
            "handle_unauthenticated completed"
        );
        
        result
    }

    /// Handle username input
    #[instrument(skip(self, adapter))]
    async fn handle_username_input(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        let username = input.trim();

        if username.is_empty() {
            adapter
                .send_line("Username cannot be empty.\r\n")
                .await
                .map_err(|e| e.to_string())?;
            tracing::debug!(
                session_id = %self.session_id,
                duration_ms = %start.elapsed().as_millis(),
                "handle_username_input: empty username rejected"
            );
            return Ok(());
        }

        // Check if user wants to create new account
        if username.eq_ignore_ascii_case("n") || username.eq_ignore_ascii_case("new") {
            adapter.send_line("\r\n").await.map_err(|e| e.to_string())?;
            let result = self
                .transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::NewAccount(NewAccountState::Banner),
                ))
                .await;
            
            tracing::info!(
                session_id = %self.session_id,
                duration_ms = %start.elapsed().as_millis(),
                "handle_username_input: new account flow initiated"
            );
            return result;
        }

        // Store username and move to password
        self.temp_username = Some(username.to_string());
        let result = self.transition_to(SessionState::Unauthenticated(
            UnauthenticatedState::Password,
        ))
        .await;
        
        tracing::info!(
            session_id = %self.session_id,
            duration_ms = %start.elapsed().as_millis(),
            username_len = %username.len(),
            "handle_username_input: username accepted, transitioning to password"
        );
        
        result
    }

    /// Handle password input
    #[instrument(skip(self, adapter, input))]
    async fn handle_password_input(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        let password = input.trim();

        if password.is_empty() {
            adapter
                .send_line("Password cannot be empty.\r\n")
                .await
                .map_err(|e| e.to_string())?;
            tracing::debug!(
                session_id = %self.session_id,
                duration_ms = %start.elapsed().as_millis(),
                "handle_password_input: empty password rejected"
            );
            return Ok(());
        }

        let username = self
            .temp_username
            .clone()
            .ok_or_else(|| "No username stored".to_string())?;

        // Authenticate with server
        let send_start = std::time::Instant::now();
        adapter
            .send_line("Authenticating...\r\n")
            .await
            .map_err(|e| e.to_string())?;
        tracing::debug!(
            session_id = %self.session_id,
            duration_ms = %send_start.elapsed().as_millis(),
            "handle_password_input: sent authenticating message"
        );

        let rpc_start = std::time::Instant::now();
        let rpc_client = self.context.rpc_client();
        let auth_result = rpc_client
            .authenticate_session(
                self.session_id.to_string(),
                username.clone(),
                password.to_string(),
                self.address.clone(),
            )
            .await;
        
        tracing::info!(
            session_id = %self.session_id,
            duration_ms = %rpc_start.elapsed().as_millis(),
            success = %auth_result.is_ok(),
            "handle_password_input: RPC authenticate_session call completed"
        );
        
        match auth_result {
            Ok(account_info) => {
                tracing::info!(
                    "Session {} authenticated successfully for user {}",
                    self.session_id,
                    username
                );

                let welcome_start = std::time::Instant::now();
                adapter
                    .send_line(&format!("Welcome back, {}!\r\n", account_info.login))
                    .await
                    .map_err(|e| e.to_string())?;
                tracing::debug!(
                    session_id = %self.session_id,
                    duration_ms = %welcome_start.elapsed().as_millis(),
                    "handle_password_input: sent welcome message"
                );

                // Display MOTD
                let motd_start = std::time::Instant::now();
                let motd = self
                    .context
                    .properties_manager()
                    .get_property(GatewayProperty::BannerMotd)
                    .await
                    .unwrap_or_else(|_| String::from("\r\n=== MOTD Banner Fetch Error ===\r\n"));
                
                tracing::debug!(
                    session_id = %self.session_id,
                    duration_ms = %motd_start.elapsed().as_millis(),
                    motd_len = %motd.len(),
                    "handle_password_input: fetched MOTD"
                );

                if !motd.is_empty() {
                    let motd_send_start = std::time::Instant::now();
                    adapter.send_line(&motd).await.map_err(|e| e.to_string())?;
                    tracing::debug!(
                        session_id = %self.session_id,
                        duration_ms = %motd_send_start.elapsed().as_millis(),
                        "handle_password_input: sent MOTD"
                    );
                }

                // Clear temp data
                self.temp_username = None;
                self.temp_password = None;

                // Transition to authenticated state
                let transition_start = std::time::Instant::now();
                let result = self.transition_to(SessionState::Authenticated(AuthenticatedState::Playing))
                    .await;
                
                tracing::info!(
                    session_id = %self.session_id,
                    transition_duration_ms = %transition_start.elapsed().as_millis(),
                    total_duration_ms = %start.elapsed().as_millis(),
                    "handle_password_input: authentication flow completed successfully"
                );

                // Server now sends output (like character list) directly to client
                // No need to fetch and forward queued events manually

                result
            }
            Err(e) => {
                tracing::warn!(
                    "Authentication failed for session {}: {}",
                    self.session_id,
                    e
                );

                adapter
                    .send_line(&format!("Authentication failed: {}\r\n", e))
                    .await
                    .map_err(|e| e.to_string())?;

                // Clear temp data and return to username
                self.temp_username = None;
                let result = self.transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::Username,
                ))
                .await;
                
                tracing::info!(
                    session_id = %self.session_id,
                    total_duration_ms = %start.elapsed().as_millis(),
                    "handle_password_input: authentication failed, returned to username"
                );
                
                result
            }
        }
    }

    /// Handle new account creation flow
    async fn handle_new_account(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        substate: NewAccountState,
        input: String,
    ) -> Result<(), String> {
        match substate {
            NewAccountState::Banner => {
                // Banner state auto-advances
                self.transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::NewAccount(NewAccountState::Username),
                ))
                .await
            }

            NewAccountState::Username => self.handle_new_username(adapter, input).await,

            NewAccountState::Password => self.handle_new_password(adapter, input).await,

            NewAccountState::PasswordConfirm => self.handle_password_confirm(adapter, input).await,

            NewAccountState::Email => self.handle_email(adapter, input).await,

            NewAccountState::Discord => self.handle_discord(adapter, input).await,

            NewAccountState::Timezone => self.handle_timezone(adapter, input).await,

            NewAccountState::Creating => {
                // Creating state shouldn't receive input
                Ok(())
            }
        }
    }

    /// Handle new username input
    async fn handle_new_username(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let username = input.trim();

        // Validate username format
        if let Err(e) = self.validate_username(username) {
            adapter
                .send_line(&format!("Invalid username: {}\r\n", e))
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }

        // Check availability
        adapter
            .send_line("Checking username availability...\r\n")
            .await
            .map_err(|e| e.to_string())?;

        let rpc_client = self.context.rpc_client();
        match rpc_client.check_username(username.to_string()).await {
            Ok(available) => {
                if available {
                    adapter
                        .send_line("Username is available!\r\n")
                        .await
                        .map_err(|e| e.to_string())?;

                    self.temp_username = Some(username.to_string());
                    self.transition_to(SessionState::Unauthenticated(
                        UnauthenticatedState::NewAccount(NewAccountState::Password),
                    ))
                    .await
                } else {
                    adapter
                        .send_line("Username is already taken. Please choose another.\r\n")
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(())
                }
            }
            Err(e) => {
                adapter
                    .send_line(&format!("Error checking username: {}\r\n", e))
                    .await
                    .map_err(|e| e.to_string())?;
                Err(format!("Username check failed: {}", e))
            }
        }
    }

    /// Handle new password input
    async fn handle_new_password(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let password = input.trim();

        if let Err(e) = self.validate_password(password) {
            adapter
                .send_line(&format!("Invalid password: {}\r\n", e))
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }

        self.temp_password = Some(password.to_string());
        self.transition_to(SessionState::Unauthenticated(
            UnauthenticatedState::NewAccount(NewAccountState::PasswordConfirm),
        ))
        .await
    }

    /// Handle password confirmation
    async fn handle_password_confirm(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let password_confirm = input.trim();
        let password = self
            .temp_password
            .clone()
            .ok_or_else(|| "No password stored".to_string())?;

        if password != password_confirm {
            adapter
                .send_line("Passwords do not match. Please try again.\r\n")
                .await
                .map_err(|e| e.to_string())?;

            self.temp_password = None;
            return self
                .transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::NewAccount(NewAccountState::Password),
                ))
                .await;
        }

        adapter
            .send_line("\r\n--- Optional Information ---\r\n")
            .await
            .map_err(|e| e.to_string())?;

        self.transition_to(SessionState::Unauthenticated(
            UnauthenticatedState::NewAccount(NewAccountState::Email),
        ))
        .await
    }

    /// Handle email input
    async fn handle_email(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let email = input.trim();

        if !email.is_empty() {
            if let Err(e) = self.validate_email(email) {
                adapter
                    .send_line(&format!("Invalid email: {}\r\n", e))
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
            self.temp_email = Some(email.to_string());
        }

        self.transition_to(SessionState::Unauthenticated(
            UnauthenticatedState::NewAccount(NewAccountState::Discord),
        ))
        .await
    }

    /// Handle Discord input
    async fn handle_discord(
        &mut self,
        _adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let discord = input.trim();

        if !discord.is_empty() {
            self.temp_discord = Some(discord.to_string());
        }

        self.transition_to(SessionState::Unauthenticated(
            UnauthenticatedState::NewAccount(NewAccountState::Timezone),
        ))
        .await
    }

    /// Handle timezone input
    async fn handle_timezone(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        input: String,
    ) -> Result<(), String> {
        let timezone = input.trim();

        if !timezone.is_empty() {
            self.temp_timezone = Some(timezone.to_string());
        }

        // Create the account
        self.create_account(adapter).await
    }

    /// Create the account on the server
    async fn create_account(&mut self, adapter: &mut dyn ProtocolAdapter) -> Result<(), String> {
        adapter
            .send_line("\r\nCreating account...\r\n")
            .await
            .map_err(|e| e.to_string())?;

        let username = self
            .temp_username
            .clone()
            .ok_or_else(|| "No username stored".to_string())?;
        let password = self
            .temp_password
            .clone()
            .ok_or_else(|| "No password stored".to_string())?;

        let rpc_client = self.context.rpc_client();
        match rpc_client
            .create_account(
                self.address.clone(),
                username.clone(),
                password,
                self.temp_email.clone(),
                self.temp_display_name.clone(),
                self.temp_discord.clone(),
                self.temp_timezone.clone(),
            )
            .await
        {
            Ok(account_info) => {
                tracing::info!(
                    "Account created successfully for user {} (session {})",
                    username,
                    self.session_id
                );

                adapter
                    .send_line(&format!(
                        "\r\nAccount created successfully!\r\nWelcome, {}!\r\n",
                        account_info.login
                    ))
                    .await
                    .map_err(|e| e.to_string())?;

                // Clear temp data
                self.clear_temp_data();

                // Transition to authenticated state
                self.transition_to(SessionState::Authenticated(AuthenticatedState::Playing))
                    .await
            }
            Err(e) => {
                tracing::warn!(
                    "Account creation failed for session {}: {}",
                    self.session_id,
                    e
                );

                adapter
                    .send_line(&format!("Account creation failed: {}\r\n", e))
                    .await
                    .map_err(|e| e.to_string())?;

                // Clear temp data and return to username
                self.clear_temp_data();
                self.transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::NewAccount(NewAccountState::Username),
                ))
                .await
            }
        }
    }

    /// Handle input in authenticated state
    #[instrument(skip(self, _adapter))]
    async fn handle_authenticated(
        &mut self,
        _adapter: &mut dyn ProtocolAdapter,
        substate: AuthenticatedState,
        input: String,
    ) -> Result<(), String> {
        match substate {
            AuthenticatedState::Playing => {
                // Send command to server via RPC
                let rpc_client = self.context.rpc_client();
                use wyldlands_common::proto::SessionToWorldClient;

                if let Some(client) = rpc_client.session_client().await {
                    let mut client: SessionToWorldClient = client;
                    let request = wyldlands_common::proto::SendInputRequest {
                        session_id: self.session_id.to_string(),
                        command: input,
                    };

                    match client.send_input(request).await {
                        Ok(_response) => {
                            // Response is handled by the RPC callback
                            Ok(())
                        }
                        Err(e) => Err(format!("Failed to send command: {}", e)),
                    }
                } else {
                    Err("Server not connected".to_string())
                }
            }

            AuthenticatedState::Editing { title, content } => {
                self.handle_editing_input(_adapter, title, content, input)
                    .await
            }
        }
    }

    /// Send prompt for unauthenticated state
    async fn send_unauthenticated_prompt(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        substate: UnauthenticatedState,
    ) -> Result<(), String> {
        match substate {
            UnauthenticatedState::Welcome => {
                // Display banners and username prompt concurrently to speed up connection
                let properties_manager = self.context.properties_manager().clone();
                
                // Fetch both properties in a single RPC call if not in cache
                let _ = properties_manager.refresh_cached_properties(&[
                    GatewayProperty::BannerWelcome,
                    GatewayProperty::BannerLogin,
                ]).await;
                
                let welcome_banner_task = properties_manager.get_property(GatewayProperty::BannerWelcome);
                let login_banner_task = properties_manager.get_property(GatewayProperty::BannerLogin);
                
                let (welcome_banner, login_banner) = tokio::join!(welcome_banner_task, login_banner_task);
                
                let welcome_banner = welcome_banner.unwrap_or_else(|_| BANNER_WELCOME_DEFAULT.to_string());
                let login_banner = login_banner.unwrap_or_else(|_| BANNER_LOGIN_DEFAULT.to_string());

                adapter
                    .send_line(&welcome_banner)
                    .await
                    .map_err(|e| e.to_string())?;

                adapter
                    .send_line(&login_banner)
                    .await
                    .map_err(|e| e.to_string())?;

                // Auto-advance to username
                self.transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::Username,
                ))
                .await?;

                // Send username prompt directly to avoid recursion
                adapter
                    .send_text("Username (or 'n' for new account): ")
                    .await
                    .map_err(|e| e.to_string())?;
                
                // Flush to ensure prompt is sent immediately
                adapter.flush().await.map_err(|e| e.to_string())
            }

            UnauthenticatedState::Username => {
                adapter
                    .send_text("Username (or 'n' for new account): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            UnauthenticatedState::Password => {
                adapter
                    .send_text("Password: ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            UnauthenticatedState::NewAccount(new_state) => {
                self.send_new_account_prompt(adapter, new_state).await
            }
        }
    }

    /// Send prompt for new account state
    async fn send_new_account_prompt(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        substate: NewAccountState,
    ) -> Result<(), String> {
        match substate {
            NewAccountState::Banner => {
                adapter
                    .send_line("\r\n=== Create New Account ===\r\n")
                    .await
                    .map_err(|e| e.to_string())?;

                // Auto-advance to username
                self.transition_to(SessionState::Unauthenticated(
                    UnauthenticatedState::NewAccount(NewAccountState::Username),
                ))
                .await?;

                // Send username prompt directly to avoid recursion
                adapter
                    .send_text("Choose a username (3-20 characters, letters/numbers/_): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::Username => {
                adapter
                    .send_text("Choose a username (3-20 characters, letters/numbers/_): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::Password => {
                adapter
                    .send_text("Choose a password (minimum 6 characters): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::PasswordConfirm => {
                adapter
                    .send_text("Confirm password: ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::Email => {
                adapter
                    .send_text("Email address (press Enter to skip): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::Discord => {
                adapter
                    .send_text("Discord username (press Enter to skip): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::Timezone => {
                adapter
                    .send_text("Timezone (e.g., America/Los_Angeles, press Enter to skip): ")
                    .await
                    .map_err(|e| e.to_string())?;
                adapter.flush().await.map_err(|e| e.to_string())
            }

            NewAccountState::Creating => {
                Ok(()) // No prompt while creating
            }
        }
    }

    /// Transition to a new state
    async fn transition_to(&mut self, new_state: SessionState) -> Result<(), String> {
        let start = std::time::Instant::now();
        let old_state = self.cached_state.to_metric_str();
        let new_state_str = new_state.to_metric_str();
        
        // Update cached state
        self.cached_state = new_state.clone();
        
        // Update session manager
        let manager_start = std::time::Instant::now();
        let result = self.context
            .session_manager()
            .transition_session(self.session_id, new_state)
            .await;
        
        tracing::info!(
            session_id = %self.session_id,
            old_state = %old_state,
            new_state = %new_state_str,
            manager_duration_ms = %manager_start.elapsed().as_millis(),
            total_duration_ms = %start.elapsed().as_millis(),
            success = %result.is_ok(),
            "transition_to completed"
        );
        
        result
    }

    /// Validate username format
    fn validate_username(&self, username: &str) -> Result<(), String> {
        if username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }
        if username.len() > 20 {
            return Err("Username must be at most 20 characters".to_string());
        }
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Username can only contain letters, numbers, and underscores".to_string());
        }
        Ok(())
    }

    /// Validate password format
    fn validate_password(&self, password: &str) -> Result<(), String> {
        if password.is_empty() {
            return Err("Password cannot be empty".to_string());
        }
        if password.len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }
        if password.len() > 100 {
            return Err("Password must be at most 100 characters".to_string());
        }
        Ok(())
    }

    /// Validate email format (basic check)
    fn validate_email(&self, email: &str) -> Result<(), String> {
        if !email.is_empty() && !email.contains('@') {
            return Err("Invalid email address".to_string());
        }
        Ok(())
    }

    /// Clear temporary data
    fn clear_temp_data(&mut self) {
        self.temp_username = None;
        self.temp_password = None;
        self.temp_display_name = None;
        self.temp_email = None;
        self.temp_discord = None;
        self.temp_timezone = None;
    }

    /// Handle input in editing mode with cursor control
    async fn handle_editing_input(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        title: String,
        content: String,
        input: String,
    ) -> Result<(), String> {
        // Parse special editor commands
        let trimmed = input.trim();

        match trimmed {
            // Save and exit
            ".s" | ".save" => self.save_editor_content(adapter, content).await,

            // Cancel and discard changes
            ".q" | ".quit" => {
                adapter
                    .send_line("\r\nChanges discarded.\r\n")
                    .await
                    .map_err(|e| e.to_string())?;
                self.transition_to(SessionState::Authenticated(AuthenticatedState::Playing))
                    .await
            }

            // Show help
            ".h" | ".help" => self.show_editor_help(adapter).await,

            // Word wrap current content
            ".w" | ".wrap" => self.wrap_editor_content(adapter, title, content).await,

            // Unwrap current content
            ".u" | ".unwrap" => self.unwrap_editor_content(adapter, title, content).await,

            // Show current content
            ".p" | ".print" => self.print_editor_content(adapter, &content).await,

            // Clear all content
            ".c" | ".clear" => {
                adapter
                    .send_line("\r\nContent cleared.\r\n")
                    .await
                    .map_err(|e| e.to_string())?;
                self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
                    title,
                    content: String::new(),
                }))
                .await
            }

            // Toggle between insert and overwrite modes
            ".i" | ".insert" => self.toggle_editor_mode(adapter).await,

            // Set foreground color
            cmd if cmd.starts_with(".fg ") => {
                let color = cmd.strip_prefix(".fg ").unwrap().trim();
                self.set_foreground_color(adapter, title, content, color)
                    .await
            }

            // Set background color
            cmd if cmd.starts_with(".bg ") => {
                let color = cmd.strip_prefix(".bg ").unwrap().trim();
                self.set_background_color(adapter, title, content, color)
                    .await
            }

            // Regular text input - handle based on current mode
            _ => {
                let new_content = match self.editor_mode {
                    EditorMode::Insert => {
                        // Insert mode: append new line
                        let mut new_content = content;
                        if !new_content.is_empty() {
                            new_content.push('\n');
                        }
                        new_content.push_str(&input);
                        new_content
                    }
                    EditorMode::Overwrite => {
                        // Overwrite mode: replace content at cursor position
                        self.overwrite_at_cursor(&content, &input)
                    }
                };

                // Update cursor position
                self.editor_cursor_position = new_content.len();

                self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
                    title,
                    content: new_content,
                }))
                .await
            }
        }
    }

    /// Save editor content and exit editing mode
    async fn save_editor_content(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        content: String,
    ) -> Result<(), String> {
        adapter
            .send_line("\r\nSaving content...\r\n")
            .await
            .map_err(|e| e.to_string())?;

        // Send the edited content to the server
        let rpc_client = self.context.rpc_client();
        use wyldlands_common::proto::SessionToWorldClient;

        if let Some(client) = rpc_client.session_client().await {
            let mut client: SessionToWorldClient = client;
            let request = wyldlands_common::proto::SendInputRequest {
                session_id: self.session_id.to_string(),
                command: format!(".editor_save {}", content),
            };

            match client.send_input(request).await {
                Ok(_response) => {
                    adapter
                        .send_line("Content saved successfully.\r\n")
                        .await
                        .map_err(|e| e.to_string())?;

                    // Reset editor state
                    self.editor_cursor_position = 0;

                    self.transition_to(SessionState::Authenticated(AuthenticatedState::Playing))
                        .await
                }
                Err(e) => {
                    adapter
                        .send_line(&format!("Failed to save: {}\r\n", e))
                        .await
                        .map_err(|e| e.to_string())?;
                    Err(format!("Failed to save content: {}", e))
                }
            }
        } else {
            Err("Server not connected".to_string())
        }
    }

    /// Show editor help
    async fn show_editor_help(&self, adapter: &mut dyn ProtocolAdapter) -> Result<(), String> {
        let mode_name = match self.editor_mode {
            EditorMode::Insert => "Insert",
            EditorMode::Overwrite => "Overwrite",
        };

        let help_text = format!(
            r#"
=== Editor Commands ===
.s, .save       - Save and exit
.q, .quit       - Quit without saving
.h, .help       - Show this help
.w, .wrap       - Word wrap content to terminal width
.u, .unwrap     - Remove soft line breaks
.p, .print      - Display current content
.c, .clear      - Clear all content
.i, .insert     - Toggle between Insert/Overwrite mode
.fg <color>     - Set foreground color (e.g., .fg red, .fg #FF0000, .fg reset)
.bg <color>     - Set background color (e.g., .bg blue, .bg #0000FF, .bg reset)

Current Mode: {}

Colors: black, red, green, yellow, blue, magenta, cyan, white, reset
        bright_black, bright_red, bright_green, bright_yellow,
        bright_blue, bright_magenta, bright_cyan, bright_white
        Or use hex codes: #RRGGBB (e.g., #FF5500)

In Insert mode, new lines are appended to the content.
In Overwrite mode, new lines replace existing content at cursor position.
Type text normally to add/replace lines based on current mode.
"#,
            mode_name
        );

        adapter
            .send_line(&help_text)
            .await
            .map_err(|e| e.to_string())
    }

    /// Toggle between insert and overwrite modes
    async fn toggle_editor_mode(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
    ) -> Result<(), String> {
        self.editor_mode = match self.editor_mode {
            EditorMode::Insert => EditorMode::Overwrite,
            EditorMode::Overwrite => EditorMode::Insert,
        };

        let mode_name = match self.editor_mode {
            EditorMode::Insert => "Insert",
            EditorMode::Overwrite => "Overwrite",
        };

        adapter
            .send_line(&format!("\r\nEditor mode: {}\r\n", mode_name))
            .await
            .map_err(|e| e.to_string())
    }

    /// Overwrite content at cursor position
    fn overwrite_at_cursor(&self, content: &str, new_line: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        // Calculate which line the cursor is on
        let mut char_count = 0;
        let mut target_line = 0;

        for (i, line) in lines.iter().enumerate() {
            char_count += line.len() + 1; // +1 for newline
            if char_count > self.editor_cursor_position {
                target_line = i;
                break;
            }
        }

        // If cursor is beyond all lines, append
        if target_line >= lines.len() {
            result.extend_from_slice(&lines);
            result.push(new_line);
        } else {
            // Replace the line at cursor position
            for (i, line) in lines.iter().enumerate() {
                if i == target_line {
                    result.push(new_line);
                } else {
                    result.push(line);
                }
            }
        }

        result.join("\n")
    }

    /// Word wrap editor content using terminal_word_wrap
    async fn wrap_editor_content(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        title: String,
        content: String,
    ) -> Result<(), String> {
        // Use terminal_word_wrap to wrap the content
        let wrapped = terminal_word_wrap(&content, self.editor_terminal_width);
        let wrapped_string = wrapped.to_string();

        adapter
            .send_line(&format!(
                "\r\nContent wrapped to {} columns.\r\n",
                self.editor_terminal_width
            ))
            .await
            .map_err(|e| e.to_string())?;

        // Update cursor position to end of wrapped content
        self.editor_cursor_position = wrapped_string.len();

        self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
            title,
            content: wrapped_string,
        }))
        .await
    }

    /// Unwrap editor content using terminal_word_unwrap
    async fn unwrap_editor_content(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        title: String,
        content: String,
    ) -> Result<(), String> {
        // Use terminal_word_unwrap to remove soft line breaks
        let unwrapped = terminal_word_unwrap(&content);
        let unwrapped_string = unwrapped.to_string();

        adapter
            .send_line("\r\nSoft line breaks removed.\r\n")
            .await
            .map_err(|e| e.to_string())?;

        // Update cursor position to end of unwrapped content
        self.editor_cursor_position = unwrapped_string.len();

        self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
            title,
            content: unwrapped_string,
        }))
        .await
    }

    /// Print current editor content
    async fn print_editor_content(
        &self,
        adapter: &mut dyn ProtocolAdapter,
        content: &str,
    ) -> Result<(), String> {
        adapter
            .send_line("\r\n=== Current Content ===\r\n")
            .await
            .map_err(|e| e.to_string())?;

        if content.is_empty() {
            adapter
                .send_line("(empty)\r\n")
                .await
                .map_err(|e| e.to_string())?;
        } else {
            adapter
                .send_line(content)
                .await
                .map_err(|e| e.to_string())?;
            adapter.send_line("\r\n").await.map_err(|e| e.to_string())?;
        }

        adapter
            .send_line("=== End of Content ===\r\n")
            .await
            .map_err(|e| e.to_string())
    }

    /// Set terminal width for word wrapping
    pub fn set_terminal_width(&mut self, width: usize) {
        self.editor_terminal_width = width.max(20).min(200); // Clamp between 20 and 200
    }

    /// Get current cursor position in editor
    pub fn get_cursor_position(&self) -> usize {
        self.editor_cursor_position
    }

    /// Set foreground color for subsequent text
    async fn set_foreground_color(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        title: String,
        content: String,
        color: &str,
    ) -> Result<(), String> {
        let ansi_code = self.color_to_ansi_fg(color)?;

        // Add the ANSI code to the content
        let new_content = format!("{}{}", content, ansi_code);

        adapter
            .send_line(&format!("\r\nForeground color set to: {}\r\n", color))
            .await
            .map_err(|e| e.to_string())?;

        // Update cursor position
        self.editor_cursor_position = new_content.len();

        self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
            title,
            content: new_content,
        }))
        .await
    }

    /// Set background color for subsequent text
    async fn set_background_color(
        &mut self,
        adapter: &mut dyn ProtocolAdapter,
        title: String,
        content: String,
        color: &str,
    ) -> Result<(), String> {
        let ansi_code = self.color_to_ansi_bg(color)?;

        // Add the ANSI code to the content
        let new_content = format!("{}{}", content, ansi_code);

        adapter
            .send_line(&format!("\r\nBackground color set to: {}\r\n", color))
            .await
            .map_err(|e| e.to_string())?;

        // Update cursor position
        self.editor_cursor_position = new_content.len();

        self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
            title,
            content: new_content,
        }))
        .await
    }

    /// Convert color name or hex code to ANSI foreground escape sequence
    fn color_to_ansi_fg(&self, color: &str) -> Result<String, String> {
        let color_lower = color.to_lowercase();

        // Handle hex colors
        if color_lower.starts_with('#') {
            return self.hex_to_ansi_fg(&color_lower);
        }

        // Handle named colors
        let code = match color_lower.as_str() {
            "black" => "30",
            "red" => "31",
            "green" => "32",
            "yellow" => "33",
            "blue" => "34",
            "magenta" => "35",
            "cyan" => "36",
            "white" => "37",
            "bright_black" | "gray" | "grey" => "90",
            "bright_red" => "91",
            "bright_green" => "92",
            "bright_yellow" => "93",
            "bright_blue" => "94",
            "bright_magenta" => "95",
            "bright_cyan" => "96",
            "bright_white" => "97",
            "reset" | "default" => "39",
            _ => {
                return Err(format!(
                    "Unknown color: {}. Use a color name, hex code (#RRGGBB), or 'reset'",
                    color
                ));
            }
        };

        Ok(format!("\x1b[{}m", code))
    }

    /// Convert color name or hex code to ANSI background escape sequence
    fn color_to_ansi_bg(&self, color: &str) -> Result<String, String> {
        let color_lower = color.to_lowercase();

        // Handle hex colors
        if color_lower.starts_with('#') {
            return self.hex_to_ansi_bg(&color_lower);
        }

        // Handle named colors
        let code = match color_lower.as_str() {
            "black" => "40",
            "red" => "41",
            "green" => "42",
            "yellow" => "43",
            "blue" => "44",
            "magenta" => "45",
            "cyan" => "46",
            "white" => "47",
            "bright_black" | "gray" | "grey" => "100",
            "bright_red" => "101",
            "bright_green" => "102",
            "bright_yellow" => "103",
            "bright_blue" => "104",
            "bright_magenta" => "105",
            "bright_cyan" => "106",
            "bright_white" => "107",
            "reset" | "default" => "49",
            _ => {
                return Err(format!(
                    "Unknown color: {}. Use a color name, hex code (#RRGGBB), or 'reset'",
                    color
                ));
            }
        };

        Ok(format!("\x1b[{}m", code))
    }

    /// Convert hex color to ANSI foreground 24-bit color escape sequence
    fn hex_to_ansi_fg(&self, hex: &str) -> Result<String, String> {
        let (r, g, b) = self.parse_hex_color(hex)?;
        Ok(format!("\x1b[38;2;{};{};{}m", r, g, b))
    }

    /// Convert hex color to ANSI background 24-bit color escape sequence
    fn hex_to_ansi_bg(&self, hex: &str) -> Result<String, String> {
        let (r, g, b) = self.parse_hex_color(hex)?;
        Ok(format!("\x1b[48;2;{};{};{}m", r, g, b))
    }

    /// Parse hex color string to RGB values
    fn parse_hex_color(&self, hex: &str) -> Result<(u8, u8, u8), String> {
        let hex = hex.trim_start_matches('#');

        if hex.len() != 6 {
            return Err(format!(
                "Invalid hex color: #{}. Must be 6 characters (RRGGBB)",
                hex
            ));
        }

        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| format!("Invalid hex color: #{}", hex))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| format!("Invalid hex color: #{}", hex))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| format!("Invalid hex color: #{}", hex))?;

        Ok((r, g, b))
    }
}



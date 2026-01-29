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

//! Login state machine for telnet connections

use crate::avatar::Avatar;
use crate::context::ServerContext;
use crate::session::SessionState;
use crate::shell::AccountCreationFlow;
use crate::telnet::character_sheet;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;
use wyldlands_common::account::Account;
use wyldlands_common::character::{AttributeType, CharacterBuilder};

/// Login state for a telnet connection
#[derive(Debug, Clone)]
pub enum LoginState {
    /// Showing the welcome banner
    Welcome,
    /// Prompting for username or new account
    UsernameOrNew,
    /// Prompting for password
    Password { username: String },
    /// Creating a new account
    CreateAccount,
    /// Showing MOTD
    Motd { account: Account },
    /// Selecting or creating avatar
    AvatarSelection { account: Account },
    /// Creating a new avatar - name prompt
    AvatarCreationName { account: Account },
    /// Interactive character builder
    AvatarCreationBuilder {
        account: Account,
        builder: CharacterBuilder,
    },
    /// Selecting starting location
    AvatarCreationLocation {
        account: Account,
        builder: CharacterBuilder,
    },
    /// Login complete
    Complete { account: Account, avatar: Avatar },
}

/// Login handler for telnet connections
pub struct LoginHandler {
    session_id: Uuid,
    context: ServerContext,
    state: LoginState,
    input_buffer: String,
    account_flow: Option<AccountCreationFlow>,
    prompt_shown: bool,
}

impl LoginHandler {
    /// Create a new login handler
    pub fn new(session_id: Uuid, context: ServerContext) -> Self {
        Self {
            session_id,
            context,
            state: LoginState::Welcome,
            input_buffer: String::new(),
            account_flow: None,
            prompt_shown: false,
        }
    }

    /// Process the login flow
    pub async fn process(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<Option<(Account, Avatar)>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if session is already in CharacterSelection state (returning from exit)
        if let Some(session) = self
            .context
            .session_manager()
            .get_session(self.session_id)
            .await
        {
            if session.state == SessionState::CharacterSelection {
                // Try to retrieve the account from session metadata
                if let Some(account_id_str) = session.metadata.custom.get("account_id") {
                    if let Ok(account_id) = Uuid::parse_str(account_id_str) {
                        match self
                            .context
                            .auth_manager()
                            .get_account_by_id(account_id)
                            .await
                        {
                            Ok(account) => {
                                // Successfully retrieved account, go directly to character selection
                                tracing::info!(
                                    "Returning to character selection for account {}",
                                    account.login
                                );
                                self.state = LoginState::AvatarSelection { account };
                                self.prompt_shown = false;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to retrieve account from session: {}", e);
                                // Fall back to username prompt
                                self.state = LoginState::UsernameOrNew;
                                self.prompt_shown = false;
                            }
                        }
                    }
                }
            }
        }

        loop {
            // Clone the state to avoid borrow checker issues
            let current_state = self.state.clone();

            match current_state {
                LoginState::Welcome => {
                    self.show_welcome(stream).await?;
                    self.state = LoginState::UsernameOrNew;
                    self.prompt_shown = false;
                }
                LoginState::UsernameOrNew => {
                    // Only prompt if we haven't shown it yet for this state
                    if !self.prompt_shown {
                        stream
                            .write_all(b"\r\nUsername (or 'new' to create account): ")
                            .await?;
                        stream.flush().await?;
                        self.prompt_shown = true;
                    }

                    if let Some(input) =
                        read_line_from_stream(stream, &mut self.input_buffer).await?
                    {
                        tracing::debug!("Received username input: {}", input);

                        if input.is_empty() {
                            tracing::debug!("Empty input, re-prompting");
                            self.prompt_shown = false;
                            continue;
                        }

                        if input.to_lowercase() == "new" {
                            // Log the account creation attempt
                            tracing::info!(
                                "New account creation initiated for session {}",
                                self.session_id
                            );

                            // Notify user that account creation is starting
                            stream
                                .write_all(b"\r\n=== New Account Creation ===\r\n\r\n")
                                .await?;
                            stream.flush().await?;

                            // Initialize account creation flow
                            self.account_flow =
                                Some(AccountCreationFlow::new(self.context.clone()));
                            self.state = LoginState::CreateAccount;
                            self.prompt_shown = false;
                        } else {
                            tracing::debug!("Username provided: {}", input);
                            self.state = LoginState::Password { username: input };
                            self.prompt_shown = false;
                        }
                    } else {
                        tracing::debug!("No complete line received yet, continuing");
                    }
                }
                LoginState::CreateAccount => {
                    let has_flow = self.account_flow.is_some();
                    if !has_flow {
                        self.state = LoginState::UsernameOrNew;
                        self.prompt_shown = false;
                        continue;
                    }

                    // Only show prompt if we haven't shown it yet for this step
                    if !self.prompt_shown {
                        let prompt = self.account_flow.as_ref().unwrap().get_prompt();
                        stream.write_all(prompt.as_bytes()).await?;
                        stream.flush().await?;
                        self.prompt_shown = true;
                    }

                    let input_opt = read_line_from_stream(stream, &mut self.input_buffer).await?;

                    if let Some(input) = input_opt {
                        let result = if let Some(flow) = &mut self.account_flow {
                            flow.process_input(&input).await
                        } else {
                            self.state = LoginState::UsernameOrNew;
                            self.prompt_shown = false;
                            continue;
                        };

                        match result {
                            Ok(Some(request)) => {
                                match self
                                    .context
                                    .auth_manager()
                                    .create_account(request.clone())
                                    .await
                                {
                                    Ok(account) => {
                                        stream
                                            .write_all(b"\r\n\r\nAccount created successfully!\r\n")
                                            .await?;

                                        // Authenticate with RPC server using the new account credentials
                                        if let Some(mut client) =
                                            self.context.rpc_client().client().await
                                        {
                                            let auth_request =
                                                wyldlands_common::proto::AuthenticateRequest {
                                                    session_id: self.session_id.to_string(),
                                                    username: request.username.clone(),
                                                    password: request.password.clone(),
                                                };

                                            match client.authenticate(auth_request).await {
                                                Ok(response) => {
                                                    let auth_result = response.into_inner();
                                                    if auth_result.success {
                                                        tracing::info!(
                                                            "RPC server authentication successful for new account, session {}",
                                                            self.session_id
                                                        );
                                                        stream
                                                            .write_all(
                                                                b"Server authentication successful!\r\n\r\n",
                                                            )
                                                            .await?;
                                                    } else {
                                                        let error_msg =
                                                            auth_result.error.unwrap_or_else(
                                                                || "Unknown error".to_string(),
                                                            );
                                                        tracing::warn!(
                                                            "RPC server authentication failed for new account: {}",
                                                            error_msg
                                                        );
                                                        stream
                                                            .write_all(
                                                                format!(
                                                                    "Warning: Server authentication failed: {}\r\n\r\n",
                                                                    error_msg
                                                                )
                                                                .as_bytes(),
                                                            )
                                                            .await?;
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "RPC call failed for new account: {}",
                                                        e
                                                    );
                                                    stream
                                                        .write_all(
                                                            b"Warning: Server connection error.\r\n\r\n",
                                                        )
                                                        .await?;
                                                }
                                            }
                                        } else {
                                            tracing::error!(
                                                "No RPC client available for new account authentication"
                                            );
                                            stream
                                                .write_all(b"Warning: Server unavailable.\r\n\r\n")
                                                .await?;
                                        }

                                        self.account_flow = None;
                                        self.state = LoginState::Motd { account };
                                        self.prompt_shown = false;
                                    }
                                    Err(e) => {
                                        stream
                                            .write_all(
                                                format!(
                                                    "\r\nError creating account: {}\r\n\r\n",
                                                    e
                                                )
                                                .as_bytes(),
                                            )
                                            .await?;
                                        self.account_flow = None;
                                        self.state = LoginState::UsernameOrNew;
                                        self.prompt_shown = false;
                                    }
                                }
                            }
                            Ok(None) => {
                                // Flow continues to next step, reset prompt flag
                                self.prompt_shown = false;
                                continue;
                            }
                            Err(e) => {
                                stream
                                    .write_all(format!("\r\nError: {}\r\n", e).as_bytes())
                                    .await?;
                                // Keep prompt_shown true so we don't re-prompt on error
                                self.prompt_shown = false;
                                continue;
                            }
                        }
                    }
                }
                LoginState::Password { username } => {
                    self.prompt_password(stream).await?;
                    if let Some(password) =
                        read_line_from_stream(stream, &mut self.input_buffer).await?
                    {
                        match self.authenticate(&username, &password).await {
                            Ok(account) => {
                                // Authenticate with RPC server
                                if let Some(mut client) = self.context.rpc_client().client().await {
                                    let auth_request =
                                        wyldlands_common::proto::AuthenticateRequest {
                                            session_id: self.session_id.to_string(),
                                            username: username.clone(),
                                            password: password.clone(),
                                        };

                                    match client.authenticate(auth_request).await {
                                        Ok(response) => {
                                            let auth_result = response.into_inner();
                                            if auth_result.success {
                                                tracing::info!(
                                                    "RPC server authentication successful for session {}",
                                                    self.session_id
                                                );
                                            } else {
                                                let error_msg = auth_result
                                                    .error
                                                    .unwrap_or_else(|| "Unknown error".to_string());
                                                tracing::warn!(
                                                    "RPC server authentication failed: {}",
                                                    error_msg
                                                );
                                                stream
                                                    .write_all(
                                                        format!(
                                                            "\r\nServer authentication failed: {}\r\n",
                                                            error_msg
                                                        )
                                                        .as_bytes(),
                                                    )
                                                    .await?;
                                                self.state = LoginState::UsernameOrNew;
                                                self.prompt_shown = false;
                                                continue;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("RPC call failed: {}", e);
                                            stream
                                                .write_all(
                                                    b"\r\nServer connection error. Please try again.\r\n",
                                                )
                                                .await?;
                                            self.state = LoginState::UsernameOrNew;
                                            self.prompt_shown = false;
                                            continue;
                                        }
                                    }
                                } else {
                                    tracing::error!("No RPC client available for authentication");
                                    stream
                                        .write_all(
                                            b"\r\nServer unavailable. Please try again later.\r\n",
                                        )
                                        .await?;
                                    self.state = LoginState::UsernameOrNew;
                                    self.prompt_shown = false;
                                    continue;
                                }

                                self.state = LoginState::Motd { account };
                            }
                            Err(e) => {
                                stream
                                    .write_all(format!("\r\n{}\r\n\r\n", e).as_bytes())
                                    .await?;
                                self.state = LoginState::UsernameOrNew;
                            }
                        }
                    }
                }
                LoginState::Motd { account } => {
                    self.show_motd(stream).await?;

                    // Transition session to CharacterSelection state if not already there
                    if let Some(session) = self
                        .context
                        .session_manager()
                        .get_session(self.session_id)
                        .await
                    {
                        if session.state != SessionState::CharacterSelection {
                            self.context
                                .session_manager()
                                .transition_session(
                                    self.session_id,
                                    SessionState::CharacterSelection,
                                )
                                .await
                                .map_err(|e| format!("Failed to transition session: {}", e))?;
                        }
                    }

                    // Store account ID in session metadata for return to character selection
                    if let Some(mut session) = self
                        .context
                        .session_manager()
                        .get_session(self.session_id)
                        .await
                    {
                        session
                            .metadata
                            .custom
                            .insert("account_id".to_string(), account.id.to_string());
                        self.context
                            .session_manager()
                            .update_session(session)
                            .await
                            .map_err(|e| format!("Failed to update session: {}", e))?;
                    }

                    self.state = LoginState::AvatarSelection { account };
                }
                LoginState::AvatarSelection { account } => {
                    match self.show_avatar_selection(stream, &account).await? {
                        AvatarSelectionResult::Selected(avatar) => {
                            self.state = LoginState::Complete { account, avatar };
                        }
                        AvatarSelectionResult::CreateNew => {
                            self.state = LoginState::AvatarCreationName { account };
                        }
                        AvatarSelectionResult::Continue => {
                            // Stay in this state
                        }
                    }
                }
                LoginState::AvatarCreationName { account } => {
                    stream.write_all(b"\r\nEnter character name: ").await?;
                    if let Some(name) =
                        read_line_from_stream(stream, &mut self.input_buffer).await?
                    {
                        if name.is_empty() {
                            self.state = LoginState::AvatarSelection { account };
                            continue;
                        }
                        if !is_valid_name(&name) {
                            stream
                                .write_all(b"\r\nInvalid name. Use only letters and spaces.\r\n")
                                .await?;
                            continue;
                        }

                        // Create character builder and show initial sheet
                        let builder = CharacterBuilder::new(name);
                        self.state = LoginState::AvatarCreationBuilder { account, builder };
                        self.prompt_shown = false;
                    }
                }
                LoginState::AvatarCreationBuilder {
                    account,
                    mut builder,
                } => {
                    // Show character sheet if not shown yet
                    if !self.prompt_shown {
                        let sheet = character_sheet::format_character_sheet(&builder);
                        stream.write_all(sheet.as_bytes()).await?;
                        stream.flush().await?;
                        self.prompt_shown = true;
                    }

                    if let Some(input) =
                        read_line_from_stream(stream, &mut self.input_buffer).await?
                    {
                        let input = input.trim().to_lowercase();

                        if input.is_empty() {
                            self.prompt_shown = false;
                            self.state = LoginState::AvatarCreationBuilder { account, builder };
                            continue;
                        }

                        match input.as_str() {
                            "done" => {
                                // Transition to location selection
                                stream.write_all(b"\r\nCharacter build complete! Now choose your starting location.\r\n").await?;
                                self.state =
                                    LoginState::AvatarCreationLocation { account, builder };
                                self.prompt_shown = false;
                            }
                            "cancel" => {
                                stream
                                    .write_all(b"\r\nCharacter creation cancelled.\r\n")
                                    .await?;
                                self.state = LoginState::AvatarSelection { account };
                                self.prompt_shown = false;
                            }
                            "sheet" => {
                                // Redisplay sheet
                                self.prompt_shown = false;
                                self.state = LoginState::AvatarCreationBuilder { account, builder };
                            }
                            "talents" => {
                                // Show talents list
                                let talents = character_sheet::format_talents_list();
                                stream.write_all(talents.as_bytes()).await?;
                                stream.flush().await?;
                                self.state = LoginState::AvatarCreationBuilder { account, builder };
                            }
                            "skills" => {
                                // Show skills list
                                let skills = character_sheet::format_skills_list();
                                stream.write_all(skills.as_bytes()).await?;
                                stream.flush().await?;
                                self.state = LoginState::AvatarCreationBuilder { account, builder };
                            }
                            _ => {
                                // Try to parse as command
                                let result =
                                    self.process_builder_command(&mut builder, &input).await;
                                match result {
                                    Ok(msg) => {
                                        if !msg.is_empty() {
                                            stream
                                                .write_all(format!("\r\n{}\r\n", msg).as_bytes())
                                                .await?;
                                        }
                                        self.prompt_shown = false;
                                        self.state =
                                            LoginState::AvatarCreationBuilder { account, builder };
                                    }
                                    Err(e) => {
                                        stream
                                            .write_all(format!("\r\nError: {}\r\n", e).as_bytes())
                                            .await?;
                                        self.state =
                                            LoginState::AvatarCreationBuilder { account, builder };
                                    }
                                }
                            }
                        }
                    }
                }
                LoginState::AvatarCreationLocation {
                    account,
                    mut builder,
                } => {
                    // Show starting locations if not shown yet
                    if !self.prompt_shown {
                        match self.context.auth_manager().get_starting_locations().await {
                            Ok(locations) => {
                                stream
                                    .write_all(b"\r\n=== Choose Your Starting Location ===\r\n\r\n")
                                    .await?;

                                for (i, location) in locations.iter().enumerate() {
                                    stream
                                        .write_all(
                                            format!(
                                                "  {}. {}\r\n     {}\r\n\r\n",
                                                i + 1,
                                                location.name,
                                                location.description
                                            )
                                            .as_bytes(),
                                        )
                                        .await?;
                                }

                                stream
                                    .write_all(b"Select a location (enter number): ")
                                    .await?;
                                stream.flush().await?;
                                self.prompt_shown = true;
                            }
                            Err(e) => {
                                stream
                                    .write_all(
                                        format!("\r\nError loading locations: {}\r\n", e)
                                            .as_bytes(),
                                    )
                                    .await?;
                                self.state = LoginState::AvatarSelection { account };
                                self.prompt_shown = false;
                                continue;
                            }
                        }
                    }

                    if let Some(input) =
                        read_line_from_stream(stream, &mut self.input_buffer).await?
                    {
                        let input = input.trim();

                        if input.is_empty() {
                            self.prompt_shown = false;
                            self.state = LoginState::AvatarCreationLocation { account, builder };
                            continue;
                        }

                        // Get locations again to process selection
                        match self.context.auth_manager().get_starting_locations().await {
                            Ok(locations) => {
                                if let Ok(index) = input.parse::<usize>() {
                                    if index > 0 && index <= locations.len() {
                                        let selected_location = &locations[index - 1];
                                        builder.set_starting_location(selected_location.id.clone());

                                        stream
                                            .write_all(
                                                format!(
                                                    "\r\nYou have chosen: {}\r\n",
                                                    selected_location.name
                                                )
                                                .as_bytes(),
                                            )
                                            .await?;

                                        // Now validate and create character
                                        match builder.is_valid() {
                                            Ok(()) => {
                                                match self
                                                    .create_avatar_from_builder(&account, &builder)
                                                    .await
                                                {
                                                    Ok(avatar) => {
                                                        stream
                                                            .write_all(b"\r\nCharacter created successfully!\r\n")
                                                            .await?;
                                                        self.state = LoginState::Complete {
                                                            account,
                                                            avatar,
                                                        };
                                                    }
                                                    Err(e) => {
                                                        stream
                                                            .write_all(
                                                                format!("\r\nError: {}\r\n", e)
                                                                    .as_bytes(),
                                                            )
                                                            .await?;
                                                        self.prompt_shown = false;
                                                        self.state =
                                                            LoginState::AvatarCreationLocation {
                                                                account,
                                                                builder,
                                                            };
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                stream
                                                    .write_all(
                                                        format!(
                                                            "\r\nCannot create character: {}\r\n",
                                                            e
                                                        )
                                                        .as_bytes(),
                                                    )
                                                    .await?;
                                                self.prompt_shown = false;
                                                self.state = LoginState::AvatarCreationLocation {
                                                    account,
                                                    builder,
                                                };
                                            }
                                        }
                                    } else {
                                        stream
                                            .write_all(
                                                b"\r\nInvalid selection. Please try again.\r\n",
                                            )
                                            .await?;
                                        self.prompt_shown = false;
                                        self.state =
                                            LoginState::AvatarCreationLocation { account, builder };
                                    }
                                } else {
                                    stream.write_all(b"\r\nPlease enter a number.\r\n").await?;
                                    self.prompt_shown = false;
                                    self.state =
                                        LoginState::AvatarCreationLocation { account, builder };
                                }
                            }
                            Err(e) => {
                                stream
                                    .write_all(format!("\r\nError: {}\r\n", e).as_bytes())
                                    .await?;
                                self.prompt_shown = false;
                                self.state =
                                    LoginState::AvatarCreationLocation { account, builder };
                            }
                        }
                    }
                }
                LoginState::Complete { account, avatar } => {
                    self.context
                        .session_manager()
                        .transition_session(self.session_id, SessionState::Playing)
                        .await
                        .map_err(|e| format!("Failed to transition session: {}", e))?;

                    self.context
                        .auth_manager()
                        .update_last_played(avatar.entity_id)
                        .await
                        .map_err(|e| format!("Failed to update last played: {}", e))?;

                    return Ok(Some((account, avatar)));
                }
            }
        }
    }

    async fn show_welcome(
        &self,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let banner = self
            .context
            .banner_manager()
            .get_welcome_banner()
            .await
            .unwrap_or_else(|_: String| "Welcome to Wyldlands!\r\n".to_string());

        stream.write_all(banner.as_bytes()).await?;
        Ok(())
    }

    async fn show_motd(
        &self,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let motd = self
            .context
            .banner_manager()
            .get_motd()
            .await
            .unwrap_or_else(|_: String| "Message of the Day\r\n".to_string());

        stream.write_all(motd.as_bytes()).await?;
        stream.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn prompt_password(
        &self,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        stream.write_all(b"Password: ").await?;
        Ok(())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<Account, String> {
        self.context
            .auth_manager()
            .authenticate(username, password)
            .await
    }

    async fn show_avatar_selection(
        &mut self,
        stream: &mut TcpStream,
        account: &Account,
    ) -> Result<AvatarSelectionResult, Box<dyn std::error::Error + Send + Sync>> {
        let avatar_infos = self
            .context
            .auth_manager()
            .get_avatars(account.id)
            .await
            .map_err(|e| format!("Failed to get avatars: {}", e))?;

        stream
            .write_all(b"\r\n=== Character Selection ===\r\n\r\n")
            .await?;

        if avatar_infos.is_empty() {
            stream
                .write_all(b"You have no characters yet.\r\n\r\n")
                .await?;
        } else {
            for (i, avatar_info) in avatar_infos.iter().enumerate() {
                stream
                    .write_all(format!("  {}. {}\r\n", i + 1, avatar_info.name,).as_bytes())
                    .await?;
            }
            stream.write_all(b"\r\n").await?;
        }

        stream.write_all(b"  N. Create new character\r\n").await?;
        stream.write_all(b"  Q. Quit\r\n\r\n").await?;
        stream.write_all(b"Select: ").await?;

        if let Some(choice) = read_line_from_stream(stream, &mut self.input_buffer).await? {
            let choice = choice.trim().to_lowercase();

            if choice == "n" || choice == "new" {
                return Ok(AvatarSelectionResult::CreateNew);
            } else if choice == "q" || choice == "quit" {
                return Err("Goodbye!".into());
            } else if let Ok(index) = choice.parse::<usize>() {
                if index > 0 && index <= avatar_infos.len() {
                    // Fetch the actual Avatar record for the selected character
                    let avatar = self
                        .context
                        .auth_manager()
                        .get_avatar(avatar_infos[index - 1].entity_id)
                        .await
                        .map_err(|e| format!("Failed to get avatar: {}", e))?;
                    return Ok(AvatarSelectionResult::Selected(avatar));
                }
            }

            stream.write_all(b"\r\nInvalid selection.\r\n").await?;
        }

        Ok(AvatarSelectionResult::Continue)
    }

    /// Process a character builder command
    async fn process_builder_command(
        &self,
        builder: &mut CharacterBuilder,
        input: &str,
    ) -> Result<String, String> {
        // Try attribute command
        if let Some((attr, increase)) = character_sheet::parse_attribute_command(input) {
            return if increase {
                builder.increase_attribute(attr)?;
                Ok(format!(
                    "{} increased to {}",
                    attr.name(),
                    builder.get_attribute(attr)
                ))
            } else {
                builder.decrease_attribute(attr)?;
                Ok(format!(
                    "{} decreased to {}",
                    attr.name(),
                    builder.get_attribute(attr)
                ))
            };
        }

        // Try skill command
        if let Some((skill_name, increase)) = character_sheet::parse_skill_command(input) {
            return if increase {
                builder.increase_skill(skill_name.clone())?;
                Ok(format!(
                    "{} increased to {}",
                    skill_name,
                    builder.get_skill(&skill_name)
                ))
            } else {
                builder.decrease_skill(&skill_name)?;
                Ok(format!(
                    "{} decreased to {}",
                    skill_name,
                    builder.get_skill(&skill_name)
                ))
            };
        }

        // Try talent command
        if let Some((add, talent_name)) = character_sheet::parse_talent_command(input) {
            if let Some(talent) = character_sheet::find_talent_by_name(&talent_name) {
                return if add {
                    builder.add_talent(talent)?;
                    Ok(format!("Added talent: {}", talent.name()))
                } else {
                    builder.remove_talent(talent)?;
                    Ok(format!("Removed talent: {}", talent.name()))
                };
            } else {
                return Err(format!("Unknown talent: {}", talent_name));
            }
        }

        Err("Unknown command. Type 'sheet' to see available commands.".to_string())
    }

    /// Create avatar from character builder
    async fn create_avatar_from_builder(
        &self,
        account: &Account,
        builder: &CharacterBuilder,
    ) -> Result<Avatar, String> {
        // Get the starting location to retrieve the room_id
        let starting_location_id = builder
            .starting_location_id
            .as_ref()
            .ok_or_else(|| "No starting location selected".to_string())?;

        let starting_location = self
            .context
            .auth_manager()
            .get_starting_location(starting_location_id)
            .await?;

        // Convert builder attributes to character data
        let mut attributes = std::collections::HashMap::new();

        // Map our attribute system to the old system for now
        // AttributeType is already imported at the top
        attributes.insert(
            "body_offence".to_string(),
            builder.get_attribute(AttributeType::BodyOffence),
        );
        attributes.insert(
            "body_finesse".to_string(),
            builder.get_attribute(AttributeType::BodyFinesse),
        );
        attributes.insert(
            "body_defence".to_string(),
            builder.get_attribute(AttributeType::BodyDefence),
        );
        attributes.insert(
            "mind_offence".to_string(),
            builder.get_attribute(AttributeType::MindOffence),
        );
        attributes.insert(
            "mind_finesse".to_string(),
            builder.get_attribute(AttributeType::MindFinesse),
        );
        attributes.insert(
            "mind_defence".to_string(),
            builder.get_attribute(AttributeType::MindDefence),
        );
        attributes.insert(
            "soul_offence".to_string(),
            builder.get_attribute(AttributeType::SoulOffence),
        );
        attributes.insert(
            "soul_finesse".to_string(),
            builder.get_attribute(AttributeType::SoulFinesse),
        );
        attributes.insert(
            "soul_defence".to_string(),
            builder.get_attribute(AttributeType::SoulDefence),
        );

        // Create metadata with talents, skills, and starting location
        let mut metadata = std::collections::HashMap::new();

        // Store talents
        let talents_json = serde_json::to_string(&builder.talents)
            .map_err(|e| format!("Failed to serialize talents: {}", e))?;
        metadata.insert("talents".to_string(), talents_json);

        // Store skills
        let skills_json = serde_json::to_string(&builder.skills)
            .map_err(|e| format!("Failed to serialize skills: {}", e))?;
        metadata.insert("skills".to_string(), skills_json);

        // Store starting location info
        metadata.insert(
            "starting_location_id".to_string(),
            starting_location.id.clone(),
        );
        metadata.insert(
            "starting_location_name".to_string(),
            starting_location.name.clone(),
        );
        metadata.insert(
            "starting_room_id".to_string(),
            starting_location.room_id.to_string(),
        );

        // Character creation is now handled through the unified send_command interface
        // The server's state machine will route this to character creation logic
        // For now, we'll use the RPC client's command queuing if not connected
        let rpc_client = self.context.rpc_client();

        // Queue the finalize command - it will be processed when connected
        let finalize_cmd = "create finalize".to_string();
        rpc_client
            .send_command_or_queue(self.session_id.to_string(), finalize_cmd)
            .await
            .map_err(|e| format!("Failed to queue character creation: {}", e))?;

        // For now, return a placeholder entity ID
        // In a real implementation, we'd wait for the server response
        let entity_id = uuid::Uuid::new_v4().to_string();

        // TODO: This needs to be refactored to properly wait for server response
        // and get the actual entity ID. For now, we'll just proceed with character creation
        tracing::warn!("Character creation using placeholder entity ID - needs refactoring");

        // Entity ID is already set above
        let entity_id = entity_id;

        // Parse entity_id string to UUID
        let entity_uuid =
            uuid::Uuid::parse_str(&entity_id).map_err(|e| format!("Invalid entity ID: {}", e))?;

        // Link the avatar to the account
        let avatar = self
            .context
            .auth_manager()
            .link_avatar(account.id, entity_uuid)
            .await?;

        tracing::info!(
            "Created character {} (entity_id: {}) for account {} with {} talents and {} skills",
            builder.name,
            entity_id,
            account.id,
            builder.talents.len(),
            builder.skills.len()
        );

        Ok(avatar)
    }
}

enum AvatarSelectionResult {
    Selected(Avatar),
    CreateNew,
    Continue,
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 50
        && name.chars().all(|c| c.is_alphabetic() || c.is_whitespace())
}

/// Read a line from a TCP stream with buffering
async fn read_line_from_stream(
    stream: &mut TcpStream,
    input_buffer: &mut String,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = [0u8; 1024];

    match stream.read(&mut buffer).await {
        Ok(0) => Err("Connection closed".into()),
        Ok(n) => {
            let data = &buffer[..n];
            let mut i = 0;

            while i < data.len() {
                let byte = data[i];

                // Handle telnet IAC (Interpret As Command) sequences
                if byte == 255 {
                    // IAC
                    // Skip IAC and the next 1-2 bytes (command and possibly option)
                    if i + 1 < data.len() {
                        let cmd = data[i + 1];
                        // Commands like WILL, WONT, DO, DONT take an option byte
                        if cmd >= 251 && cmd <= 254 && i + 2 < data.len() {
                            i += 3; // Skip IAC, command, and option
                        } else {
                            i += 2; // Skip IAC and command
                        }
                    } else {
                        i += 1;
                    }
                    continue;
                }

                // Handle line endings
                if byte == b'\r' || byte == b'\n' {
                    // Return the line (even if empty) when we hit a line ending
                    let line = input_buffer.clone();
                    input_buffer.clear();
                    return Ok(Some(line.trim().to_string()));
                } else if byte >= 32 && byte < 127 {
                    // Only add printable ASCII characters
                    input_buffer.push(byte as char);
                }

                i += 1;
            }

            Ok(None)
        }
        Err(e) => Err(format!("Read error: {}", e).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_name() {
        assert!(is_valid_name("Gandalf"));
        assert!(is_valid_name("Frodo Baggins"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("Test123"));
        assert!(!is_valid_name("Test@User"));
    }
}

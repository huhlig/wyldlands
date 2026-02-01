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

//! Server RPC handler for gateway-to-server communication

use crate::ecs::components::{AttributeType, CharacterBuilder, EntityId, Skill, Talent};
use crate::ecs::context::WorldContext;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use wyldlands_common::gateway::{PersistentEntityId, SessionId};
use wyldlands_common::proto::game_output::OutputType;
use wyldlands_common::proto::{
    AuthenticateGatewayRequest, AuthenticateGatewayResponse, AuthenticateSessionRequest,
    AuthenticateSessionResponse, CheckUsernameRequest, CheckUsernameResponse, CreateAccountRequest,
    CreateAccountResponse, DataValue, EditResponse, Empty, GameOutput, GatewayHeartbeatRequest,
    GatewayHeartbeatResponse, GatewayManagement, GatewayPropertiesRequest,
    GatewayPropertiesResponse, SendInputRequest, SendInputResponse, ServerStatisticsRequest,
    ServerStatisticsResponse, SessionDisconnectedRequest, SessionHeartbeatRequest,
    SessionHeartbeatResponse, SessionReconnectedRequest, SessionReconnectedResponse,
    SessionToWorld, StructuredOutput, TextOutput, game_output,
};

/// Server RPC handler
///
/// Implements the GatewayServer trait to receive calls from the gateway.
#[derive(Clone)]
pub struct ServerRpcHandler {
    /// Session state storage
    sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,

    /// Active entity mapping (SessionId -> EntityId)
    active_entities: Arc<RwLock<HashMap<SessionId, EntityId>>>,

    /// Character builders for sessions in character creation
    character_builders: Arc<RwLock<HashMap<SessionId, CharacterBuilder>>>,

    /// World engine context (contains world, registry, and persistence)
    world_context: Arc<WorldContext>,

    /// Expected authentication key
    auth_key: String,

    /// Whether this connection is authenticated
    authenticated: Arc<RwLock<bool>>,

    /// Start time of the server handler
    start_time: std::time::Instant,
}

/// Session state type for routing commands
#[derive(Debug, Clone, PartialEq)]
enum SessionStateType {
    Unauthenticated,
    Authenticated,
    CharacterCreation,
    Playing,
    Editing,
}

/// Session state tracked by the server
#[derive(Debug, Clone)]
struct SessionState {
    /// Current state of the session
    state: SessionStateType,

    /// Whether the session is authenticated
    authenticated: bool,

    /// The authenticated account ID (if any)
    account_id: Option<uuid::Uuid>,

    /// The authenticated entity ID (if any)
    entity_id: Option<PersistentEntityId>,

    /// Client IP address from gateway
    client_addr: Option<String>,

    /// Queued events during disconnection
    queued_events: Vec<GameOutput>,
}

impl ServerRpcHandler {
    /// Create a new server RPC handler
    pub fn new(auth_key: &str, world_context: Arc<WorldContext>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_entities: Arc::new(RwLock::new(HashMap::new())),
            character_builders: Arc::new(RwLock::new(HashMap::new())),
            world_context,
            auth_key: auth_key.to_string(),
            authenticated: Arc::new(RwLock::new(false)),
            start_time: std::time::Instant::now(),
        }
    }

    /// Check if the connection is authenticated
    async fn is_authenticated(&self) -> bool {
        *self.authenticated.read().await
    }
}

// ============================================================================
// gRPC Server Implementation
// ============================================================================

#[tonic::async_trait]
impl GatewayManagement for ServerRpcHandler {
    /// Authenticate the gateway connection
    async fn authenticate_gateway(
        &self,
        request: Request<AuthenticateGatewayRequest>,
    ) -> Result<Response<AuthenticateGatewayResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("gRPC Gateway authentication attempt");

        // Check if the provided auth key matches
        if req.auth_key != self.auth_key {
            tracing::warn!("gRPC Gateway authentication failed: invalid auth key");
            return Ok(Response::new(AuthenticateGatewayResponse {
                success: false,
                error: Some("Invalid authentication key".to_string()),
            }));
        }

        // Mark as authenticated
        let mut authenticated = self.authenticated.write().await;
        *authenticated = true;

        tracing::info!("gRPC Gateway authenticated successfully");
        Ok(Response::new(AuthenticateGatewayResponse {
            success: true,
            error: None,
        }))
    }

    async fn fetch_gateway_properties(
        &self,
        request: Request<GatewayPropertiesRequest>,
    ) -> Result<Response<GatewayPropertiesResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let keys = request.into_inner().properties;
        let mut properties = HashMap::new();
        // Load requested properties from database settings table in one go
        match sqlx::query_scalar::<_, (String, String)>(
            "SELECT key, value FROM wyldlands.settings WHERE key = ANY($1)",
        )
        .bind(&keys)
        .fetch_all(self.world_context.persistence().database())
        .await
        {
            Ok(rows) => {
                for (key, value) in rows {
                    properties.insert(key, value);
                }
                let missing_keys: Vec<&String> = keys
                    .iter()
                    .filter(|a| !properties.contains_key(*a))
                    .collect();
                if !missing_keys.is_empty() {
                    tracing::warn!("Requested properties missing: {:?}", missing_keys);
                }
            }
            Err(e) => {
                tracing::error!("Failed to load properties from database: {}", e);
            }
        }

        Ok(Response::new(GatewayPropertiesResponse { properties }))
    }

    async fn fetch_server_statistics(
        &self,
        _request: Request<ServerStatisticsRequest>,
    ) -> Result<Response<ServerStatisticsResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let mut statistics = HashMap::new();

        // Server Uptime
        let uptime = self.start_time.elapsed();
        statistics.insert("uptime_seconds".to_string(), uptime.as_secs().to_string());

        // Session Statistics
        let sessions = self.sessions.read().await;
        statistics.insert("active_sessions".to_string(), sessions.len().to_string());

        let mut authenticated_sessions = 0;
        let mut playing_sessions = 0;
        for session in sessions.values() {
            if session.authenticated {
                authenticated_sessions += 1;
            }
            if session.state == SessionStateType::Playing {
                playing_sessions += 1;
            }
        }
        statistics.insert(
            "authenticated_sessions".to_string(),
            authenticated_sessions.to_string(),
        );
        statistics.insert("playing_sessions".to_string(), playing_sessions.to_string());

        // Entity Statistics
        let active_entities = self.active_entities.read().await;
        statistics.insert("active_entities".to_string(), active_entities.len().to_string());

        statistics.insert(
            "world_entities".to_string(),
            self.world_context.len().await.to_string(),
        );
        statistics.insert(
            "dirty_entities".to_string(),
            self.world_context.dirty_count().await.to_string(),
        );

        // Character Creation Statistics
        let builders = self.character_builders.read().await;
        statistics.insert(
            "characters_in_creation".to_string(),
            builders.len().to_string(),
        );

        Ok(Response::new(ServerStatisticsResponse { statistics }))
    }

    async fn check_username(
        &self,
        request: Request<CheckUsernameRequest>,
    ) -> Result<Response<CheckUsernameResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::debug!("Checking username availability: {}", req.username);

        // Check if username exists in database
        match self
            .world_context
            .persistence()
            .username_exists(&req.username)
            .await
        {
            Ok(exists) => Ok(Response::new(CheckUsernameResponse {
                available: !exists,
                error: None,
            })),
            Err(e) => {
                tracing::error!("Database error checking username: {}", e);
                Ok(Response::new(CheckUsernameResponse {
                    available: false,
                    error: Some(format!("Database error: {}", e)),
                }))
            }
        }
    }

    async fn create_account(
        &self,
        request: Request<CreateAccountRequest>,
    ) -> Result<Response<CreateAccountResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::info!("Creating account for username: {}", req.username);

        // Validate input
        if req.username.is_empty() || req.password.is_empty() {
            return Ok(Response::new(CreateAccountResponse {
                success: false,
                account: None,
                error: Some("Username and password are required".to_string()),
            }));
        }

        // Check if username already exists
        match self
            .world_context
            .persistence()
            .username_exists(&req.username)
            .await
        {
            Ok(true) => {
                return Ok(Response::new(CreateAccountResponse {
                    success: false,
                    account: None,
                    error: Some("Username already exists".to_string()),
                }));
            }
            Ok(false) => {
                // Username is available, proceed with creation
            }
            Err(e) => {
                tracing::error!("Database error checking username: {}", e);
                return Ok(Response::new(CreateAccountResponse {
                    success: false,
                    account: None,
                    error: Some("Database error".to_string()),
                }));
            }
        }

        // Create the account with hashed password
        // Use username as display name if not provided in properties
        let display_name = req
            .properties
            .get("display_name")
            .cloned()
            .unwrap_or_else(|| req.username.clone());
        match self
            .world_context
            .persistence()
            .create_account(&req.username, &display_name, &req.password)
            .await
        {
            Ok(Some(account)) => {
                tracing::info!(
                    "Account created successfully: {} ({})",
                    account.login,
                    account.id
                );

                // Fetch the created account to return
                match self
                    .world_context
                    .persistence()
                    .get_account_by_id(account.id)
                    .await
                {
                    Ok(account) => {
                        let account_info = wyldlands_common::proto::AccountInfo {
                            id: account.id.to_string(),
                            login: account.login.clone(),
                            active: account.active,
                            role: account.role.to_string(),
                            properties: std::collections::HashMap::new(),
                        };

                        Ok(Response::new(CreateAccountResponse {
                            success: true,
                            account: Some(account_info),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch created account: {}", e);
                        Ok(Response::new(CreateAccountResponse {
                            success: false,
                            account: None,
                            error: Some("Account created but failed to fetch details".to_string()),
                        }))
                    }
                }
            }
            Ok(None) => {
                tracing::error!("Failed to create account without error");
                Ok(Response::new(CreateAccountResponse {
                    success: false,
                    account: None,
                    error: Some(format!("Failed to create account without error")),
                }))
            }
            Err(e) => {
                tracing::error!("Failed to create account: {}", e);
                Ok(Response::new(CreateAccountResponse {
                    success: false,
                    account: None,
                    error: Some(format!("Failed to create account: {}", e)),
                }))
            }
        }
    }

    /// Gateway-level heartbeat for connection health monitoring
    async fn gateway_heartbeat(
        &self,
        request: Request<GatewayHeartbeatRequest>,
    ) -> Result<Response<GatewayHeartbeatResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::debug!("gRPC: Gateway heartbeat from {}", req.gateway_id);

        Ok(Response::new(GatewayHeartbeatResponse {
            success: true,
            error: None,
        }))
    }
}

#[tonic::async_trait]
impl SessionToWorld for ServerRpcHandler {
    /// Authenticate a user session
    async fn authenticate_session(
        &self,
        request: Request<AuthenticateSessionRequest>,
    ) -> Result<Response<AuthenticateSessionResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::info!(
            "gRPC: Authenticating session {} - user: {}",
            req.session_id,
            req.username
        );

        // Validate input
        if req.username.is_empty() || req.password.is_empty() {
            return Ok(Response::new(AuthenticateSessionResponse {
                success: false,
                account: None,
                error: Some("Username and password are required".to_string()),
            }));
        }

        let account = match self
            .world_context
            .persistence()
            .get_account_by_authentication(&req.username, &req.password)
            .await
        {
            Ok(Some(account)) => {
                if account.active {
                    account
                } else {
                    tracing::warn!("Authentication failed: account {} disabled", req.username);
                    return Ok(Response::new(AuthenticateSessionResponse {
                        success: false,
                        account: None,
                        error: Some("Invalid username or password".to_string()),
                    }));
                }
            }
            Ok(None) => {
                tracing::warn!(
                    "Authentication failed: account not found for {}",
                    req.username
                );
                return Ok(Response::new(AuthenticateSessionResponse {
                    success: false,
                    account: None,
                    error: Some("Invalid username or password".to_string()),
                }));
            }
            Err(e) => {
                tracing::error!("Database error during authentication: {}", e);
                return Ok(Response::new(AuthenticateSessionResponse {
                    success: false,
                    account: None,
                    error: Some("Authentication service error".to_string()),
                }));
            }
        };

        // Update last_login timestamp
        if let Err(e) = self
            .world_context
            .persistence()
            .update_last_login(account.id)
            .await
        {
            tracing::warn!(
                "Failed to update last_login for account {}: {}",
                account.id,
                e
            );
        }

        // Create/update session state
        let mut sessions = self.sessions.write().await;
        sessions.insert(
            req.session_id.clone(),
            SessionState {
                state: SessionStateType::Authenticated,
                authenticated: true,
                account_id: Some(account.id),
                entity_id: None,
                client_addr: if req.client_addr.is_empty() {
                    None
                } else {
                    Some(req.client_addr.clone())
                },
                queued_events: Vec::new(),
            },
        );

        // Convert account to protobuf AccountInfo
        let account_info = wyldlands_common::proto::AccountInfo {
            id: account.id.to_string(),
            login: account.login.clone(),
            active: account.active,
            role: account.role.to_string(),
            properties: HashMap::new(), // Account properties not yet implemented
        };

        tracing::info!(
            "Session {} authenticated successfully as {} from {}",
            req.session_id,
            account.login,
            req.client_addr
        );

        Ok(Response::new(AuthenticateSessionResponse {
            success: true,
            account: Some(account_info),
            error: None,
        }))
    }

    /// Send unified input (state machine based)
    async fn send_input(
        &self,
        request: Request<SendInputRequest>,
    ) -> Result<Response<SendInputResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::debug!(
            "gRPC command from session {}: {}",
            req.session_id,
            req.command
        );

        // Get session state
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&req.session_id)
            .ok_or_else(|| Status::not_found("Session not found"))?;

        let current_state = session.state.clone();
        drop(sessions);

        // Convert SessionStateType to protobuf SessionState enum
        let proto_state = match current_state {
            SessionStateType::Unauthenticated => 1,   // UNAUTHENTICATED
            SessionStateType::Authenticated => 2,     // AUTHENTICATED
            SessionStateType::CharacterCreation => 3, // CHARACTER_CREATION
            SessionStateType::Playing => 4,           // PLAYING
            SessionStateType::Editing => 5,           // EDITING
        };

        // Route based on state
        match current_state {
            SessionStateType::Authenticated => {
                // Handle authenticated state commands (character selection, creation initiation)
                self.handle_authenticated_command(req.session_id, req.command)
                    .await
            }
            SessionStateType::CharacterCreation => {
                // Handle character creation commands
                self.handle_character_creation_command(req.session_id, req.command)
                    .await
            }
            SessionStateType::Playing => {
                // Handle gameplay commands
                self.handle_playing_command(req.session_id, req.command)
                    .await
            }
            _ => {
                // Other states not yet implemented
                Ok(Response::new(SendInputResponse {
                    success: false,
                    output: vec![],
                    error: Some(format!(
                        "Commands not implemented for state: {:?}",
                        current_state
                    )),
                }))
            }
        }
    }

    async fn finish_editing(
        &self,
        request: Request<EditResponse>,
    ) -> Result<Response<Empty>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();

        // Get the edited content
        if let Some(content) = req.content {
            tracing::info!("Received edited content ({} bytes)", content.len());

            // TODO: In a full implementation, we would:
            // 1. Look up the editing context from the session (what object/field was being edited)
            // 2. Validate the content
            // 3. Update the appropriate database record
            // 4. Notify the session that editing is complete
            // 5. Transition the session back to Playing state

            // For now, we just log that we received the content
            tracing::info!("Editing system: Content received and acknowledged");
        } else {
            tracing::info!("Editing cancelled (no content provided)");
        }

        Ok(Response::new(Empty {}))
    }

    /// Notify server of session disconnection
    async fn session_disconnected(
        &self,
        request: Request<SessionDisconnectedRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::info!("gRPC: Session {} disconnected", req.session_id);

        // Remove active entity mapping
        let mut active_entities = self.active_entities.write().await;
        active_entities.remove(&req.session_id);

        Ok(Response::new(Empty {}))
    }

    /// Handle session reconnection
    async fn session_reconnected(
        &self,
        request: Request<SessionReconnectedRequest>,
    ) -> Result<Response<SessionReconnectedResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::info!(
            "gRPC: Session {} reconnecting (old: {})",
            req.session_id,
            req.old_session_id
        );

        // Transfer session state from old session to new session
        let mut sessions = self.sessions.write().await;

        if let Some(old_state) = sessions.remove(&req.old_session_id) {
            // Transfer state to new session ID
            sessions.insert(req.session_id.clone(), old_state.clone());

            // Transfer active entity mapping if exists
            let mut active_entities = self.active_entities.write().await;
            if let Some(entity_id) = active_entities.remove(&req.old_session_id) {
                active_entities.insert(req.session_id.clone(), entity_id);
            }

            // Transfer character builder if exists
            let mut builders = self.character_builders.write().await;
            if let Some(builder) = builders.remove(&req.old_session_id) {
                builders.insert(req.session_id.clone(), builder);
            }

            tracing::info!("Session state transferred successfully");
            Ok(Response::new(SessionReconnectedResponse {
                success: true,
                error: None,
            }))
        } else {
            tracing::warn!(
                "Old session {} not found for reconnection",
                req.old_session_id
            );
            Ok(Response::new(SessionReconnectedResponse {
                success: false,
                error: Some("Old session not found".to_string()),
            }))
        }
    }

    /// Heartbeat to keep session alive
    async fn session_heartbeat(
        &self,
        request: Request<SessionHeartbeatRequest>,
    ) -> Result<Response<SessionHeartbeatResponse>, Status> {
        let req = request.into_inner();
        tracing::debug!("gRPC: Heartbeat from session {}", req.session_id);

        // Verify session exists
        let sessions = self.sessions.read().await;
        if !sessions.contains_key(&req.session_id) {
            return Err(Status::not_found("Session not found"));
        }

        Ok(Response::new(SessionHeartbeatResponse {
            success: true,
            error: None,
        }))
    }
}

// ============================================================================
// ServerRpcHandler Helper Methods
// ============================================================================

impl ServerRpcHandler {
    // ========================================================================
    // Authenticated State Command Handlers
    // ========================================================================

    /// Handle authenticated state commands (character selection, creation initiation)
    async fn handle_authenticated_command(
        &self,
        session_id: String,
        command: String,
    ) -> Result<Response<SendInputResponse>, Status> {
        let command = command.trim().to_lowercase();
        let mut output = Vec::new();

        if command == "create character"
            || command == "create"
            || command == "new character"
            || command == "new"
        {
            // Initiate character creation
            tracing::info!("Session {} initiating character creation", session_id);

            // Prompt for character name
            output.push(GameOutput {
                output_type: Some(game_output::OutputType::Text(TextOutput {
                    content: "\r\n=== Character Creation ===\r\n\r\nEnter your character's name: "
                        .to_string(),
                })),
            });

            // Transition to CharacterCreation state (will be done after name is provided)
            // For now, just acknowledge the command
            Ok(Response::new(SendInputResponse {
                success: true,
                output,
                error: None,
            }))
        } else if command.starts_with("select ") || command.starts_with("play ") {
            // Handle character selection
            let char_name = command
                .strip_prefix("select ")
                .or_else(|| command.strip_prefix("play "))
                .unwrap_or("")
                .trim();

            if char_name.is_empty() {
                output.push(GameOutput {
                    output_type: Some(game_output::OutputType::Text(TextOutput {
                        content: "Usage: select <character_name> or play <character_name>"
                            .to_string(),
                    })),
                });
                return Ok(Response::new(SendInputResponse {
                    success: false,
                    output,
                    error: Some("Character name required".to_string()),
                }));
            }

            // Get account_id from session
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| Status::not_found("Session not found"))?;
            let account_id = match session.account_id {
                Some(id) => id,
                None => {
                    drop(sessions);
                    output.push(GameOutput {
                        output_type: Some(game_output::OutputType::Text(TextOutput {
                            content: "Error: Not authenticated.".to_string(),
                        })),
                    });
                    return Ok(Response::new(SendInputResponse {
                        success: false,
                        output,
                        error: Some("Not authenticated".to_string()),
                    }));
                }
            };
            drop(sessions);

            // Load character list and find matching character
            match self
                .world_context
                .persistence()
                .list_characters_for_account(account_id)
                .await
            {
                Ok(avatars) => {
                    // Find character by name (case-insensitive)
                    let char_name_lower = char_name.to_lowercase();
                    if let Some(avatar) = avatars
                        .iter()
                        .find(|c| c.display.to_lowercase() == char_name_lower)
                    {
                        // Load the character entity into the world
                        match self.world_context.load_character(avatar.id).await {
                            Ok(entity) => {
                                // Get EntityId for the loaded character
                                let entity_id =
                                    self.world_context.get_entity_id(entity).await.ok_or_else(
                                        || Status::internal("Failed to get entity ID"),
                                    )?;

                                // Update session state to Playing
                                let mut sessions = self.sessions.write().await;
                                if let Some(session) = sessions.get_mut(&session_id) {
                                    session.state = SessionStateType::Playing;
                                    session.entity_id = Some(entity_id.uuid().to_string());
                                }

                                // Map session to active entity
                                let mut active_entities = self.active_entities.write().await;
                                active_entities.insert(session_id.clone(), entity_id);

                                output.push(GameOutput {
                                    output_type: Some(game_output::OutputType::Text(TextOutput {
                                        content: format!("Welcome back, {}!\r\n", avatar.display),
                                    })),
                                });

                                tracing::info!(
                                    "Session {} selected character {}",
                                    session_id,
                                    avatar.display
                                );

                                Ok(Response::new(SendInputResponse {
                                    success: true,
                                    output,
                                    error: None,
                                }))
                            }
                            Err(e) => {
                                tracing::error!("Failed to load character: {}", e);
                                output.push(GameOutput {
                                    output_type: Some(game_output::OutputType::Text(TextOutput {
                                        content: format!("Error loading character: {}", e),
                                    })),
                                });

                                Ok(Response::new(SendInputResponse {
                                    success: false,
                                    output,
                                    error: Some(format!("Failed to load character: {}", e)),
                                }))
                            }
                        }
                    } else {
                        output.push(GameOutput {
                            output_type: Some(OutputType::Text(TextOutput {
                                content: format!(
                                    "Character '{}' not found. Use 'list' to see your characters.",
                                    char_name
                                ),
                            })),
                        });

                        Ok(Response::new(SendInputResponse {
                            success: false,
                            output,
                            error: Some("Character not found".to_string()),
                        }))
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load character list: {}", e);
                    output.push(GameOutput {
                        output_type: Some(game_output::OutputType::Text(TextOutput {
                            content: format!("Error loading character list: {}", e),
                        })),
                    });

                    Ok(Response::new(SendInputResponse {
                        success: false,
                        output,
                        error: Some(format!("Database error: {}", e)),
                    }))
                }
            }
        } else if command == "list" || command == "avatars" || command == "characters" {
            // List available characters from database
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| Status::not_found("Session not found"))?;

            let account_id = match session.account_id {
                Some(id) => id,
                None => {
                    drop(sessions);
                    output.push(GameOutput {
                        output_type: Some(game_output::OutputType::Text(TextOutput {
                            content: "Error: Not authenticated. Please log in first.".to_string(),
                        })),
                    });
                    return Ok(Response::new(SendInputResponse {
                        success: false,
                        output,
                        error: Some("Not authenticated".to_string()),
                    }));
                }
            };
            drop(sessions);

            // Load characters from database
            match self
                .world_context
                .persistence()
                .list_characters_for_account(account_id)
                .await
            {
                Ok(avatars) => {
                    let mut content = String::from("=== Your Characters ===\r\n\r\n");

                    if avatars.is_empty() {
                        content.push_str("You have no characters yet.\r\n");
                    } else {
                        for avatar in avatars {
                            content.push_str(&format!("  {}\r\n", avatar.display,));
                        }
                    }

                    content.push_str("\r\nCommands:\r\n");
                    content.push_str("  create - Create a new character\r\n");
                    content.push_str("  select <name> - Select a character to play\r\n");

                    output.push(GameOutput {
                        output_type: Some(OutputType::Text(TextOutput { content })),
                    });

                    Ok(Response::new(SendInputResponse {
                        success: true,
                        output,
                        error: None,
                    }))
                }
                Err(e) => {
                    tracing::error!("Failed to load character list: {}", e);
                    output.push(GameOutput {
                        output_type: Some(OutputType::Text(TextOutput {
                            content: format!("Error loading character list: {}", e),
                        })),
                    });

                    Ok(Response::new(SendInputResponse {
                        success: false,
                        output,
                        error: Some(format!("Database error: {}", e)),
                    }))
                }
            }
        } else {
            output.push(GameOutput {
                output_type: Some(OutputType::Text(TextOutput {
                    content: format!("Unknown command: {}\r\n\r\nAvailable commands:\r\n  create - Create a new character\r\n  select <name> - Select a character\r\n  list - List your characters", command),
                })),
            });

            Ok(Response::new(SendInputResponse {
                success: false,
                output,
                error: Some("Unknown command".to_string()),
            }))
        }
    }

    // ========================================================================
    // Character Creation Command Handlers
    // ========================================================================

    /// Handle character creation commands
    async fn handle_character_creation_command(
        &self,
        session_id: String,
        command: String,
    ) -> Result<Response<SendInputResponse>, Status> {
        let command = command.trim();
        let mut output = Vec::new();

        // Handle finalize separately since it needs to drop the lock
        if command == "create finalize" || command == "finalize" || command == "done" {
            // Get and validate the builder
            let builders = self.character_builders.read().await;
            let builder = builders
                .get(&session_id)
                .ok_or_else(|| Status::not_found("Character builder not found for session"))?;

            let validation_result = builder.validate();
            let builder_clone = builder.clone();
            drop(builders);

            match validation_result {
                Ok(()) => {
                    // Character is valid, create it in the database
                    // Get account_id from session
                    let sessions = self.sessions.read().await;
                    let session = sessions
                        .get(&session_id)
                        .ok_or_else(|| Status::not_found("Session not found"))?;
                    let account_id = session
                        .account_id
                        .ok_or_else(|| Status::unauthenticated("Not authenticated"))?;
                    drop(sessions);

                    // Create character with full attributes, talents, and skills
                    match self
                        .world_context
                        .persistence()
                        .create_character_with_builder(account_id, &builder_clone)
                        .await
                    {
                        Ok(character_uuid) => {
                            tracing::info!(
                                "Character {} created successfully with UUID {}",
                                builder_clone.name,
                                character_uuid
                            );

                            // Remove builder from session
                            let mut builders = self.character_builders.write().await;
                            builders.remove(&session_id);
                            drop(builders);

                            // Transition session back to Authenticated state
                            let mut sessions = self.sessions.write().await;
                            if let Some(session) = sessions.get_mut(&session_id) {
                                session.state = SessionStateType::Authenticated;
                            }
                            drop(sessions);

                            output.push(GameOutput {
                                output_type: Some(game_output::OutputType::Text(TextOutput {
                                    content: format!(
                                        "Character {} created successfully!\r\n\r\nYou can now select this character with: select {}\r\n",
                                        builder_clone.name, builder_clone.name
                                    ),
                                })),
                            });

                            return Ok(Response::new(SendInputResponse {
                                success: true,
                                output,
                                error: None,
                            }));
                        }
                        Err(e) => {
                            tracing::error!("Failed to create character: {}", e);
                            output.push(GameOutput {
                                output_type: Some(game_output::OutputType::Text(TextOutput {
                                    content: format!("Error: Failed to create character: {}", e),
                                })),
                            });

                            return Ok(Response::new(SendInputResponse {
                                success: false,
                                output,
                                error: Some(format!("Failed to create character: {}", e)),
                            }));
                        }
                    }
                }
                Err(errors) => {
                    output.push(GameOutput {
                        output_type: Some(game_output::OutputType::Text(TextOutput {
                            content: format!(
                                "Error: Cannot finalize character:\n{}",
                                errors.join("\n")
                            ),
                        })),
                    });

                    return Ok(Response::new(SendInputResponse {
                        success: false,
                        output,
                        error: Some(format!("Cannot finalize character:\n{}", errors.join("\n"))),
                    }));
                }
            }
        }

        // Handle other commands with write lock
        let mut builders = self.character_builders.write().await;
        let builder = builders
            .get_mut(&session_id)
            .ok_or_else(|| Status::not_found("Character builder not found for session"))?;

        // Parse and execute command
        let result = if command.starts_with("attr ") {
            let arg = command.strip_prefix("attr ").unwrap().trim();
            self.parse_attr_command(builder, arg)
        } else if command.starts_with("talent ") {
            let arg = command.strip_prefix("talent ").unwrap().trim();
            self.parse_talent_command(builder, arg)
        } else if command.starts_with("skill ") {
            let arg = command.strip_prefix("skill ").unwrap().trim();
            self.parse_skill_command(builder, arg)
        } else if command == "sheet" {
            let sheet = self.format_character_sheet(builder);
            output.push(GameOutput {
                output_type: Some(game_output::OutputType::Text(TextOutput { content: sheet })),
            });
            Ok("Character sheet displayed".to_string())
        } else {
            Err(format!("Unknown character creation command: {}", command))
        };

        // Add result message to output
        match &result {
            Ok(msg) => {
                output.push(GameOutput {
                    output_type: Some(game_output::OutputType::Text(TextOutput {
                        content: msg.clone(),
                    })),
                });
            }
            Err(err) => {
                output.push(GameOutput {
                    output_type: Some(game_output::OutputType::Text(TextOutput {
                        content: format!("Error: {}", err),
                    })),
                });
            }
        }

        Ok(Response::new(SendInputResponse {
            success: result.is_ok(),
            output,
            error: result.err(),
        }))
    }

    /// Parse attribute modification command (e.g., "+BodyOffence", "-MindDefence")
    fn parse_attr_command(
        &self,
        builder: &mut CharacterBuilder,
        arg: &str,
    ) -> Result<String, String> {
        if arg.is_empty() {
            return Err("Usage: attr +<AttributeName> or attr -<AttributeName>".to_string());
        }

        let (delta, attr_name) = if arg.starts_with('+') {
            (1, &arg[1..])
        } else if arg.starts_with('-') {
            (-1, &arg[1..])
        } else {
            return Err("Attribute command must start with + or -".to_string());
        };

        // Parse attribute type (case-insensitive)
        let attr = AttributeType::from_str(attr_name)?;

        builder.modify_attribute(attr, delta)?;
        let new_value = builder.get_attribute(attr);
        Ok(format!(
            "{} {} to {}. Points remaining: {}",
            attr.name(),
            if delta > 0 { "increased" } else { "decreased" },
            new_value,
            builder.attribute_talent_points
        ))
    }

    /// Parse talent modification command (e.g., "+WeaponMaster", "-Berserker")
    fn parse_talent_command(
        &self,
        builder: &mut CharacterBuilder,
        arg: &str,
    ) -> Result<String, String> {
        if arg.is_empty() {
            return Err("Usage: talent +<TalentName> or talent -<TalentName>".to_string());
        }

        let (add, talent_name) = if arg.starts_with('+') {
            (true, &arg[1..])
        } else if arg.starts_with('-') {
            (false, &arg[1..])
        } else {
            return Err("Talent command must start with + or -".to_string());
        };

        // Parse talent (case-insensitive, remove spaces)
        let talent_key = talent_name.to_lowercase().replace(" ", "");
        let talent = Talent::from_str(talent_key.as_str())?;

        builder.modify_talent(talent, add)?;
        Ok(format!(
            "Talent {} {}. Points remaining: {}",
            talent.name(),
            if add { "added" } else { "removed" },
            builder.attribute_talent_points
        ))
    }

    /// Parse skill modification command (e.g., "+Swords", "-Archery")
    fn parse_skill_command(
        &self,
        builder: &mut CharacterBuilder,
        arg: &str,
    ) -> Result<String, String> {
        if arg.is_empty() {
            return Err("Usage: skill +<SkillName> or skill -<SkillName>".to_string());
        }

        let (delta, skill_name) = if arg.starts_with('+') {
            (1, &arg[1..])
        } else if arg.starts_with('-') {
            (-1, &arg[1..])
        } else {
            return Err("Skill command must start with + or -".to_string());
        };

        let skill_name = skill_name.trim().to_string();
        if skill_name.is_empty() {
            return Err("Skill name cannot be empty".to_string());
        }
        let skill = Skill::from_str(skill_name.as_str())?;

        builder.modify_skill_points(skill, delta)?;
        let new_value = builder.get_skill_level(skill);
        Ok(format!(
            "{} {} to {}. Skill points remaining: {}",
            skill_name,
            if delta > 0 { "increased" } else { "decreased" },
            new_value,
            builder.skill_points
        ))
    }

    /// Format character sheet for display
    fn format_character_sheet(&self, builder: &CharacterBuilder) -> String {
        let mut sheet = String::new();
        sheet.push_str(&format!("=== Character Sheet: {} ===\n\n", builder.name));

        // Attributes
        sheet.push_str("Attributes:\n");
        for attr in AttributeType::all() {
            let value = builder.get_attribute(attr);
            sheet.push_str(&format!("  {}: {}\n", attr.name(), value));
        }
        sheet.push_str(&format!(
            "  Points remaining: {}/{}\n\n",
            builder.attribute_talent_points, builder.max_attribute_talent_points
        ));

        // Talents
        sheet.push_str("Talents:\n");
        if builder.talents.is_empty() {
            sheet.push_str("  None\n");
        } else {
            for (talent, rank, exp) in builder.talents.iter() {
                sheet.push_str(&format!(
                    "  {} ({}pts)\n",
                    talent.name(),
                    talent.cost().unwrap_or(0)
                ));
            }
        }
        sheet.push_str("\n");

        // Skills
        sheet.push_str("Skills:\n");
        if builder.skills.is_empty() {
            sheet.push_str("  None\n");
        } else {
            let mut skills: Vec<_> = builder.skills.iter().collect();
            for (name, level, _experience, _knowledge) in skills {
                sheet.push_str(&format!("  {}: {}\n", name, level));
            }
        }
        sheet.push_str(&format!(
            "  Points remaining: {}/{}\n\n",
            builder.skill_points, builder.max_skill_points
        ));

        // Starting location
        sheet.push_str("Starting Location: ");
        if let Some(loc) = &builder.starting_location_id {
            sheet.push_str(loc);
        } else {
            sheet.push_str("Not selected");
        }
        sheet.push_str("\n\n");

        // Validation
        let errors = builder.validation_errors();
        if errors.is_empty() {
            sheet.push_str("Status: Ready to finalize!\n");
        } else {
            sheet.push_str("Validation Errors:\n");
            for error in errors {
                sheet.push_str(&format!("  - {}\n", error));
            }
        }

        sheet
    }

    /// Render ServerCharacterBuilder to GameOutput
    fn convert_builder_to_proto(&self, builder: &CharacterBuilder) -> Vec<GameOutput> {
        // Convert attributes to map<string, int32>
        let mut attributes = HashMap::new();
        for attr in AttributeType::all() {
            attributes.insert(attr.name().to_string(), builder.get_attribute(attr));
        }

        // Convert talents to Vec<String>
        let talents: Vec<String> = builder
            .talents
            .iter()
            .map(|(t, rank, exp)| t.name().to_string())
            .collect();

        // Skills are already HashMap<String, i32>
        let skills = builder.skills.clone();
        let mut output = Vec::new();

        // Text output placeholder for future character sheet rendering
        output.push(GameOutput {
            output_type: Some(OutputType::Text(TextOutput {
                content: "".to_string(),
            })),
        });

        // Structured output placeholder for future character sheet data
        output.push(GameOutput {
            output_type: Some(OutputType::Structured(StructuredOutput {
                output_type: "charsheet".to_string(),
                data: Some(DataValue { data_value: None }),
            })),
        });

        output
    }

    // ========================================================================
    // Playing State Command Handlers
    // ========================================================================

    /// Handle playing state commands (actual gameplay)
    async fn handle_playing_command(
        &self,
        session_id: String,
        command: String,
    ) -> Result<Response<SendInputResponse>, Status> {
        // Get the active entity for this session
        let active_entities = self.active_entities.read().await;
        let entity_id = active_entities
            .get(&session_id)
            .ok_or_else(|| Status::not_found("No active character for session"))?;

        let entity_id = entity_id.clone();
        drop(active_entities);

        // Process command through ECS command system
        // Parse command into command name and arguments
        let parts: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
        let (cmd_name, args) = if parts.is_empty() {
            ("".to_string(), vec![])
        } else {
            (parts[0].clone(), parts[1..].to_vec())
        };

        // Execute command through command system
        let mut command_system = self.world_context.command_system().write().await;
        let result = command_system
            .execute(
                self.world_context.clone(),
                entity_id.entity(),
                &cmd_name,
                &args,
            )
            .await;
        drop(command_system);

        // Convert CommandResult to response
        let mut output = Vec::new();
        match result {
            crate::ecs::systems::CommandResult::Success(msg) => {
                output.push(GameOutput {
                    output_type: Some(game_output::OutputType::Text(TextOutput { content: msg })),
                });

                Ok(Response::new(SendInputResponse {
                    success: true,
                    output,
                    error: None,
                }))
            }
            crate::ecs::systems::CommandResult::Failure(msg) => {
                output.push(GameOutput {
                    output_type: Some(game_output::OutputType::Text(TextOutput {
                        content: msg.clone(),
                    })),
                });

                Ok(Response::new(SendInputResponse {
                    success: false,
                    output,
                    error: Some(msg),
                }))
            }
            crate::ecs::systems::CommandResult::Invalid(msg) => {
                output.push(GameOutput {
                    output_type: Some(game_output::OutputType::Text(TextOutput {
                        content: msg.clone(),
                    })),
                });

                Ok(Response::new(SendInputResponse {
                    success: false,
                    output,
                    error: Some(msg),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::context::WorldContext;
    use crate::persistence::PersistenceManager;
    use std::sync::Arc;
    use tonic::Request;
    use wyldlands_common::proto::{ServerStatisticsRequest, GatewayManagement};

    #[tokio::test]
    async fn test_fetch_server_statistics() {
        let persistence = Arc::new(PersistenceManager::new_mock());
        let world_context = Arc::new(WorldContext::new(persistence));
        let handler = ServerRpcHandler::new("test_key", world_context);

        // Authenticate first
        let auth_request = Request::new(AuthenticateGatewayRequest {
            auth_key: "test_key".to_string(),
        });
        handler.authenticate_gateway(auth_request).await.unwrap();

        let request = Request::new(ServerStatisticsRequest {
            statistics: vec![],
        });

        let response = handler.fetch_server_statistics(request).await.unwrap();
        let stats = response.into_inner().statistics;

        assert!(stats.contains_key("uptime_seconds"));
        assert!(stats.contains_key("active_sessions"));
        assert!(stats.contains_key("active_entities"));
        assert!(stats.contains_key("world_entities"));
        assert!(stats.contains_key("dirty_entities"));
        assert!(stats.contains_key("characters_in_creation"));
        
        assert_eq!(stats.get("active_sessions").unwrap(), "0");
        assert_eq!(stats.get("world_entities").unwrap(), "0");
    }
}

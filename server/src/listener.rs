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

use crate::ecs::components::EntityId;
use crate::ecs::context::WorldContext;
use crate::ecs::ServerCharacterBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use wyldlands_common::gateway::{
    PersistentEntityId, SessionId,
};
use wyldlands_common::proto::{
    gateway_server_server::GatewayServer as GrpcGatewayServer,
    *,
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
    character_builders: Arc<RwLock<HashMap<SessionId, ServerCharacterBuilder>>>,

    /// World engine context (contains world, registry, and persistence)
    world_context: Arc<WorldContext>,

    /// Expected authentication key
    auth_key: String,

    /// Whether this connection is authenticated
    authenticated: Arc<RwLock<bool>>,
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

    /// The authenticated entity ID (if any)
    entity_id: Option<PersistentEntityId>,

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
impl GrpcGatewayServer for ServerRpcHandler {
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

    /// Authenticate a user session
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::info!("gRPC: Authenticating session {} - user: {}", req.session_id, req.username);

        // TODO: Implement real authentication
        // For now, accept any non-empty credentials
        if req.username.is_empty() || req.password.is_empty() {
            return Ok(Response::new(AuthenticateResponse {
                success: false,
                account_id: None,
                characters: vec![],
                error: Some("Invalid credentials".to_string()),
            }));
        }

        // Create/update session state
        let mut sessions = self.sessions.write().await;
        sessions.insert(
            req.session_id.clone(),
            SessionState {
                state: SessionStateType::Authenticated,
                authenticated: true,
                entity_id: None,
                queued_events: Vec::new(),
            },
        );

        // TODO: Load real characters from database
        let mock_characters = vec![
            wyldlands_common::proto::CharacterSummary {
                entity_id: uuid::Uuid::new_v4().to_string(),
                name: "Test Warrior".to_string(),
                level: 5,
                location: "Developer Hub".to_string(),
            },
        ];

        Ok(Response::new(AuthenticateResponse {
            success: true,
            account_id: Some(uuid::Uuid::new_v4().to_string()),
            characters: mock_characters,
            error: None,
        }))
    }

    /// Select a character for play
    async fn select_character(
        &self,
        request: Request<SelectCharacterRequest>,
    ) -> Result<Response<SelectCharacterResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::info!("gRPC: Selecting character {} for session {}", req.entity_id, req.session_id);

        // TODO: Load character and create CharacterInfo
        Ok(Response::new(SelectCharacterResponse {
            success: true,
            character: None, // TODO: Populate with real data
            error: None,
        }))
    }

    /// Send unified command (state machine based)
    async fn send_command(
        &self,
        request: Request<SendCommandRequest>,
    ) -> Result<Response<SendCommandResponse>, Status> {
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        tracing::debug!("gRPC command from session {}: {}", req.session_id, req.command);

        // Get session state
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&req.session_id)
            .ok_or_else(|| Status::not_found("Session not found"))?;

        let current_state = session.state.clone();
        drop(sessions);

        // Convert SessionStateType to protobuf SessionState enum
        let proto_state = match current_state {
            SessionStateType::Unauthenticated => 1, // UNAUTHENTICATED
            SessionStateType::Authenticated => 2,    // AUTHENTICATED
            SessionStateType::CharacterCreation => 3, // CHARACTER_CREATION
            SessionStateType::Playing => 4,          // PLAYING
            SessionStateType::Editing => 5,          // EDITING
        };

        // Route based on state
        match current_state {
            SessionStateType::Authenticated => {
                // Handle authenticated state commands (character selection, creation initiation)
                self.handle_authenticated_command(req.session_id, req.command).await
            }
            SessionStateType::CharacterCreation => {
                // Handle character creation commands
                self.handle_character_creation_command(req.session_id, req.command).await
            }
            SessionStateType::Playing => {
                // Handle gameplay commands
                self.handle_playing_command(req.session_id, req.command).await
            }
            _ => {
                // Other states not yet implemented
                Ok(Response::new(SendCommandResponse {
                    success: false,
                    output: vec![],
                    error: Some(format!("Commands not implemented for state: {:?}", current_state)),
                    session_state: proto_state,
                    character_builder_state: None,
                }))
            }
        }
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
        let req = request.into_inner();
        tracing::info!("gRPC: Session {} reconnecting (old: {})", req.session_id, req.old_session_id);

        // TODO: Implement reconnection logic
        Ok(Response::new(SessionReconnectedResponse {
            success: true,
            error: None,
        }))
    }

    /// Heartbeat to keep session alive
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        tracing::debug!("gRPC: Heartbeat from session {}", req.session_id);

        // Verify session exists
        let sessions = self.sessions.read().await;
        if !sessions.contains_key(&req.session_id) {
            return Err(Status::not_found("Session not found"));
        }

        Ok(Response::new(Empty {}))
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
    ) -> Result<Response<SendCommandResponse>, Status> {
        let command = command.trim().to_lowercase();
        let mut output = Vec::new();

        if command == "create character" || command == "create" || command == "new character" || command == "new" {
            // Initiate character creation
            tracing::info!("Session {} initiating character creation", session_id);

            // Prompt for character name
            output.push(GameOutput {
                output_type: Some(game_output::OutputType::Text(TextOutput {
                    content: "\r\n=== Character Creation ===\r\n\r\nEnter your character's name: ".to_string(),
                })),
            });

            // Transition to CharacterCreation state (will be done after name is provided)
            // For now, just acknowledge the command
            Ok(Response::new(SendCommandResponse {
                success: true,
                output,
                error: None,
                session_state: 2, // Still AUTHENTICATED, will transition after name
                character_builder_state: None,
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
                        content: "Usage: select <character_name> or play <character_name>".to_string(),
                    })),
                });
                return Ok(Response::new(SendCommandResponse {
                    success: false,
                    output,
                    error: Some("Character name required".to_string()),
                    session_state: 2, // AUTHENTICATED
                    character_builder_state: None,
                }));
            }

            // TODO: Implement character selection
            output.push(GameOutput {
                output_type: Some(game_output::OutputType::Text(TextOutput {
                    content: format!("Character selection not yet implemented. Requested: {}", char_name),
                })),
            });

            Ok(Response::new(SendCommandResponse {
                success: false,
                output,
                error: Some("Not implemented".to_string()),
                session_state: 2, // AUTHENTICATED
                character_builder_state: None,
            }))
        } else if command == "list" || command == "characters" {
            // List available characters
            // TODO: Load from database
            output.push(GameOutput {
                output_type: Some(game_output::OutputType::Text(TextOutput {
                    content: "Available characters:\r\n  (Character list not yet implemented)\r\n\r\nCommands:\r\n  create - Create a new character\r\n  select <name> - Select a character to play".to_string(),
                })),
            });

            Ok(Response::new(SendCommandResponse {
                success: true,
                output,
                error: None,
                session_state: 2, // AUTHENTICATED
                character_builder_state: None,
            }))
        } else {
            output.push(GameOutput {
                output_type: Some(game_output::OutputType::Text(TextOutput {
                    content: format!("Unknown command: {}\r\n\r\nAvailable commands:\r\n  create - Create a new character\r\n  select <name> - Select a character\r\n  list - List your characters", command),
                })),
            });

            Ok(Response::new(SendCommandResponse {
                success: false,
                output,
                error: Some("Unknown command".to_string()),
                session_state: 2, // AUTHENTICATED
                character_builder_state: None,
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
    ) -> Result<Response<SendCommandResponse>, Status> {
        let mut builders = self.character_builders.write().await;
        let builder = builders
            .get_mut(&session_id)
            .ok_or_else(|| Status::not_found("Character builder not found for session"))?;

        let command = command.trim();
        let mut output = Vec::new();

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
                output_type: Some(game_output::OutputType::Text(TextOutput {
                    content: sheet,
                })),
            });
            Ok("Character sheet displayed".to_string())
        } else if command == "create finalize" {
            match builder.validate() {
                Ok(()) => {
                    // TODO: Actually create the character entity
                    Ok("Character creation finalized! (TODO: Create entity)".to_string())
                }
                Err(errors) => {
                    Err(format!("Cannot finalize character:\n{}", errors.join("\n")))
                }
            }
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

        // Convert builder state to protobuf
        let builder_state = self.convert_builder_to_proto(builder);

        Ok(Response::new(SendCommandResponse {
            success: result.is_ok(),
            output,
            error: result.err(),
            session_state: 3, // CHARACTER_CREATION
            character_builder_state: Some(builder_state),
        }))
    }

    /// Parse attribute modification command (e.g., "+BodyOffence", "-MindDefence")
    fn parse_attr_command(
        &self,
        builder: &mut ServerCharacterBuilder,
        arg: &str,
    ) -> Result<String, String> {
        use wyldlands_common::character::AttributeType;

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
        let attr = match attr_name.to_lowercase().as_str() {
            "bodyoffence" | "body_offence" | "bo" => AttributeType::BodyOffence,
            "bodyfinesse" | "body_finesse" | "bf" => AttributeType::BodyFinesse,
            "bodydefence" | "body_defence" | "bd" => AttributeType::BodyDefence,
            "mindoffence" | "mind_offence" | "mo" => AttributeType::MindOffence,
            "mindfinesse" | "mind_finesse" | "mf" => AttributeType::MindFinesse,
            "minddefence" | "mind_defence" | "md" => AttributeType::MindDefence,
            "souloffence" | "soul_offence" | "so" => AttributeType::SoulOffence,
            "soulfinesse" | "soul_finesse" | "sf" => AttributeType::SoulFinesse,
            "souldefence" | "soul_defence" | "sd" => AttributeType::SoulDefence,
            _ => return Err(format!("Unknown attribute: {}", attr_name)),
        };

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
        builder: &mut ServerCharacterBuilder,
        arg: &str,
    ) -> Result<String, String> {
        use wyldlands_common::character::Talent;

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
        let talent = match talent_key.as_str() {
            "weaponmaster" => Talent::WeaponMaster,
            "shieldexpert" => Talent::ShieldExpert,
            "dualwielder" => Talent::DualWielder,
            "berserker" => Talent::Berserker,
            "tactician" => Talent::Tactician,
            "spellweaver" => Talent::Spellweaver,
            "elementalaffinity" => Talent::ElementalAffinity,
            "arcanescholar" => Talent::ArcaneScholar,
            "ritualist" => Talent::Ritualist,
            "channeler" => Talent::Channeler,
            "astralprojection" => Talent::AstralProjection,
            "mastercraftsman" => Talent::MasterCraftsman,
            "artificer" => Talent::Artificer,
            "alchemist" => Talent::Alchemist,
            "enchanter" => Talent::Enchanter,
            "diplomat" => Talent::Diplomat,
            "merchant" => Talent::Merchant,
            "leader" => Talent::Leader,
            "performer" => Talent::Performer,
            "tracker" => Talent::Tracker,
            "forager" => Talent::Forager,
            "beastmaster" => Talent::BeastMaster,
            "survivalist" => Talent::Survivalist,
            "prodigy" => Talent::Prodigy,
            "lucky" => Talent::Lucky,
            "fastlearner" => Talent::FastLearner,
            "resilient" => Talent::Resilient,
            _ => return Err(format!("Unknown talent: {}", talent_name)),
        };

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
        builder: &mut ServerCharacterBuilder,
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

        builder.modify_skill(skill_name.clone(), delta)?;
        let new_value = builder.get_skill(&skill_name);
        Ok(format!(
            "{} {} to {}. Skill points remaining: {}",
            skill_name,
            if delta > 0 { "increased" } else { "decreased" },
            new_value,
            builder.skill_points
        ))
    }

    /// Format character sheet for display
    fn format_character_sheet(&self, builder: &ServerCharacterBuilder) -> String {
        use wyldlands_common::character::AttributeType;

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
            for talent in &builder.talents {
                sheet.push_str(&format!("  {} ({}pts)\n", talent.name(), talent.cost()));
            }
        }
        sheet.push_str("\n");

        // Skills
        sheet.push_str("Skills:\n");
        if builder.skills.is_empty() {
            sheet.push_str("  None\n");
        } else {
            let mut skills: Vec<_> = builder.skills.iter().collect();
            skills.sort_by_key(|(name, _)| *name);
            for (name, rank) in skills {
                sheet.push_str(&format!("  {}: {}\n", name, rank));
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

    /// Convert ServerCharacterBuilder to protobuf CharacterBuilderState
    fn convert_builder_to_proto(&self, builder: &ServerCharacterBuilder) -> CharacterBuilderState {
        use wyldlands_common::character::AttributeType;

        // Convert attributes to map<string, int32>
        let mut attributes = HashMap::new();
        for attr in AttributeType::all() {
            attributes.insert(attr.name().to_string(), builder.get_attribute(attr));
        }

        // Convert talents to Vec<String>
        let talents: Vec<String> = builder
            .talents
            .iter()
            .map(|t| t.name().to_string())
            .collect();

        // Skills are already HashMap<String, i32>
        let skills = builder.skills.clone();

        CharacterBuilderState {
            name: builder.name.clone(),
            attributes,
            talents,
            skills,
            attribute_talent_points: builder.attribute_talent_points,
            skill_points: builder.skill_points,
            max_attribute_talent_points: builder.max_attribute_talent_points,
            max_skill_points: builder.max_skill_points,
            starting_location_id: builder.starting_location_id.clone(),
            validation_errors: builder.validation_errors(),
            is_valid: builder.is_valid(),
        }
    }

    // ========================================================================
    // Playing State Command Handlers
    // ========================================================================

    /// Handle playing state commands (actual gameplay)
    async fn handle_playing_command(
        &self,
        session_id: String,
        command: String,
    ) -> Result<Response<SendCommandResponse>, Status> {
        // Get the active entity for this session
        let active_entities = self.active_entities.read().await;
        let entity_id = active_entities
            .get(&session_id)
            .ok_or_else(|| Status::not_found("No active character for session"))?;

        let entity_id = entity_id.clone();
        drop(active_entities);

        // TODO: Process command through ECS command system
        // For now, return a placeholder response
        let mut output = Vec::new();
        output.push(GameOutput {
            output_type: Some(game_output::OutputType::Text(TextOutput {
                content: format!("Command received: {}\r\n(Command processing not yet implemented)", command),
            })),
        });

        Ok(Response::new(SendCommandResponse {
            success: true,
            output,
            error: None,
            session_state: 4, // PLAYING
            character_builder_state: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Write Tests
}



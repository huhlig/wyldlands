//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wyldlands_common::gateway::{
    AuthError, AuthResult, CharacterCreationData, CharacterError, CharacterInfo, CharacterSummary,
    CommandError, CommandResult, DisconnectReason, GameOutput, GatewayServer, PersistentEntityId,
    ReconnectError, ReconnectResult, SessionError, SessionId,
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

    /// World engine context (contains world, registry, and persistence)
    world_context: Arc<WorldContext>,

    /// Expected authentication key
    auth_key: String,

    /// Whether this connection is authenticated
    authenticated: Arc<RwLock<bool>>,
}

/// Session state tracked by the server
#[derive(Debug, Clone)]
struct SessionState {
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

impl GatewayServer for ServerRpcHandler {
    /// Authenticate the gateway connection with an auth key
    async fn authenticate_gateway(
        self,
        _context: tarpc::context::Context,
        auth_key: String,
    ) -> Result<(), String> {
        tracing::info!("Gateway authentication attempt");

        // Check if the provided auth key matches
        if auth_key != self.auth_key {
            tracing::warn!(
                "Gateway authentication failed: invalid auth key {} != {}",
                auth_key,
                self.auth_key
            );
            return Err("Invalid authentication key".to_string());
        }

        // Mark as authenticated
        let mut authenticated = self.authenticated.write().await;
        *authenticated = true;

        tracing::info!("Gateway authenticated successfully");
        Ok(())
    }

    /// Authenticate a session with credentials
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn authenticate(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
        username: String,
        password: String,
    ) -> Result<AuthResult, AuthError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to authenticate session without gateway authentication");
            return Err(AuthError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        tracing::info!(
            "Authentication attempt for session {} - user: {}",
            session_id,
            username
        );

        // TODO: Implement real authentication against database
        // For now, accept any non-empty username/password
        if username.is_empty() || password.is_empty() {
            return Ok(AuthResult {
                success: false,
                entity_id: None,
                message: "Invalid credentials".to_string(),
            });
        }

        // Create a mock entity ID
        let entity_id = uuid::Uuid::new_v4().to_string();

        // Store session state
        let mut sessions = self.sessions.write().await;
        sessions.insert(
            session_id.clone(),
            SessionState {
                authenticated: true,
                entity_id: Some(entity_id.clone()),
                queued_events: Vec::new(),
            },
        );

        Ok(AuthResult {
            success: true,
            entity_id: Some(entity_id),
            message: format!("Welcome, {}!", username),
        })
    }

    /// Create a new character for an authenticated session
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn create_character(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
        character_name: String,
        character_data: CharacterCreationData,
    ) -> Result<PersistentEntityId, CharacterError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to create character without gateway authentication");
            return Err(CharacterError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        tracing::info!(
            "Creating character '{}' for session {}",
            character_name,
            session_id
        );

        // Check if session is authenticated
        let sessions = self.sessions.read().await;
        if !sessions.get(&session_id).map_or(false, |s| s.authenticated) {
            return Err(CharacterError::NotAuthenticated);
        }
        drop(sessions);

        // Create ECS entity with components if world context is available
        {
            use crate::ecs::components::*;

            // Create entity UUID
            let entity_uuid = EntityUuid::new();
            let entity_id = entity_uuid.0.to_string();

            // Parse attributes from character data - using new BodyAttributes
            let mut body_attrs = BodyAttributes::new();

            // Map old attribute names to new score system
            if let Some(&str_val) = character_data.attributes.get("strength") {
                body_attrs.score_offence = str_val;
            }
            if let Some(&dex_val) = character_data.attributes.get("dexterity") {
                body_attrs.score_finesse = dex_val;
            }
            if let Some(&con_val) = character_data.attributes.get("constitution") {
                body_attrs.score_defence = con_val;
            }

            // Calculate initial health and energy based on attributes
            let health_max = 100.0 + (body_attrs.score_defence - 10) as f32 * 5.0;
            let energy_max = 50.0 + (body_attrs.score_finesse - 10) as f32 * 3.0;

            body_attrs.health_maximum = health_max;
            body_attrs.health_current = health_max;
            body_attrs.energy_maximum = energy_max;
            body_attrs.energy_current = energy_max;

            // Extract starting location from metadata, fallback to Developer Hub if not specified
            let (area_id, room_id) = character_data
                .metadata
                .get("starting_room_id")
                .and_then(|room_str| uuid::Uuid::parse_str(room_str).ok())
                .map(|room_uuid| {
                    // Query the database to get the area_id for this room
                    // For now, we'll use hardcoded area mapping based on known rooms
                    let area_uuid = match room_uuid.to_string().as_str() {
                        "00000000-0000-0000-0000-000000000001" => {
                            // The Void Room -> The Void area
                            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
                        }
                        "10000000-0000-0000-0000-000000000001" => {
                            // Developer Hub -> Developer Testing Grounds
                            uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap()
                        }
                        "10000000-0000-0000-0000-000000000002" => {
                            // Combat Testing Arena -> Developer Testing Grounds
                            uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap()
                        }
                        _ => {
                            // Unknown room, default to Developer Testing Grounds
                            tracing::warn!(
                                "Unknown starting room {}, defaulting to Developer Testing Grounds",
                                room_uuid
                            );
                            uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap()
                        }
                    };
                    (
                        EntityId::from_uuid(area_uuid),
                        EntityId::from_uuid(room_uuid),
                    )
                })
                .unwrap_or_else(|| {
                    // Default to Developer Hub if no starting location specified
                    tracing::info!("No starting location in metadata, defaulting to Developer Hub");
                    (
                        EntityId::from_uuid(
                            uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                        ),
                        EntityId::from_uuid(
                            uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                        ),
                    )
                });

            tracing::info!(
                "Spawning character '{}' at room {} in area {}",
                character_name,
                room_id.uuid(),
                area_id.uuid()
            );

            // Create the entity with all necessary components
            let mut world = self.world_context.entities().write().await;
            let ecs_entity = world.spawn((
                entity_uuid,
                Name::new(&character_name),
                Description::new(
                    format!("A {} {}", character_data.race, character_data.class),
                    character_data.description.clone(),
                ),
                EntityType::Player,
                Location::new(area_id, room_id),
                body_attrs,
                Skills::new(),
                Combatant::new(),
                Equipment::new(),
                Persistent,
            ));

            // Save the entity to database
            self.world_context
                .persistence_manager()
                .save_character(&world, ecs_entity)
                .await
                .map_err(|e| {
                    CharacterError::ServerError(format!("Failed to save character: {}", e))
                })?;

            // Store the EntityId in the active entities map and registry
            drop(world);
            let mut registry = self.world_context.registry().write().await;
            registry.register(ecs_entity, entity_uuid.0)
                .map_err(|e| CharacterError::ServerError(format!("Failed to register entity: {}", e)))?;

            let entity_full_id = EntityId::new(ecs_entity, entity_uuid.0);
            let mut active_entities = self.active_entities.write().await;
            active_entities.insert(session_id.clone(), entity_full_id);
            drop(active_entities);
            drop(registry);

            tracing::info!(
                "Created character '{}' with entity ID {} (ECS: {:?}) - race: {}, class: {}",
                character_name,
                entity_id,
                ecs_entity,
                character_data.race,
                character_data.class
            );

            Ok(entity_uuid.0.to_string())
        }
    }

    /// Select a character for a session
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn select_character(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
        entity_id: PersistentEntityId,
    ) -> Result<CharacterInfo, CharacterError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to select character without gateway authentication");
            return Err(CharacterError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        tracing::info!(
            "Selecting character {} for session {}",
            entity_id,
            session_id
        );

        // Check if session is authenticated
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or(CharacterError::NotAuthenticated)?;

        if !session.authenticated {
            return Err(CharacterError::NotAuthenticated);
        }

        // Update session with selected character
        session.entity_id = Some(entity_id.clone());

        // Load character from database if persistence manager is available
        {
            use crate::ecs::components::EntityUuid;

            // Parse avatar_id from entity_id string
            let avatar_id =
                uuid::Uuid::parse_str(&entity_id).map_err(|_| CharacterError::NotFound)?;

            // Check if character is already loaded in the registry
            let registry = self.world_context.registry().read().await;
            let ecs_entity = if let Some(existing_entity) = registry.get_entity(avatar_id) {
                tracing::info!(
                    "Character with UUID {} already loaded in ECS world as entity {:?}, reusing",
                    avatar_id,
                    existing_entity
                );
                drop(registry);
                existing_entity
            } else {
                drop(registry);

                // Load character from database
                tracing::info!("Loading character with UUID {} into ECS world", avatar_id);
                let mut world = self.world_context.entities().write().await;
                let registry = self.world_context.registry().read().await;
                let ecs_entity = self
                    .world_context
                    .persistence_manager()
                    .load_character(&mut world, &registry, avatar_id)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to load character: {}", e);
                        CharacterError::NotFound
                    })?;

                // Verify the entity was loaded
                if let Ok(uuid_comp) = world.get::<&EntityUuid>(ecs_entity) {
                    tracing::info!(
                        "Successfully loaded character entity {:?} with UUID {} into ECS world",
                        ecs_entity,
                        uuid_comp.0
                    );
                } else {
                    tracing::warn!(
                        "Character entity {:?} loaded but has no EntityUuid component",
                        ecs_entity
                    );
                }

                // Count total entities
                let entity_count = world.query::<&EntityUuid>().iter().count();
                tracing::info!("Total entities with EntityUuid in world: {}", entity_count);

                drop(world);
                drop(registry);

                // Register the newly loaded entity
                let mut registry = self.world_context.registry().write().await;
                registry.register(ecs_entity, avatar_id)
                    .map_err(|e| CharacterError::ServerError(format!("Failed to register entity: {}", e)))?;
                drop(registry);
                tracing::info!("Registered entity {:?} with UUID {}", ecs_entity, avatar_id);

                ecs_entity
            };

            // Store the EntityId in the active entities map
            let entity_full_id = EntityId::new(ecs_entity, avatar_id);
            let mut active_entities = self.active_entities.write().await;
            active_entities.insert(session_id.clone(), entity_full_id);
            drop(active_entities);

            tracing::info!(
                "Stored entity mapping for session {} -> EntityId (ECS: {:?}, UUID: {})",
                session_id,
                ecs_entity,
                avatar_id
            );
        }

        // TODO: Extract real character data from ECS components
        // For now, return mock data
        Ok(CharacterInfo {
            entity_id: entity_id.clone(),
            name: "Test Character".to_string(),
            level: 1,
            race: "Human".to_string(),
            class: "Warrior".to_string(),
            location: "Starting Area".to_string(),
            attributes: HashMap::from([
                ("strength".to_string(), 10),
                ("dexterity".to_string(), 10),
                ("constitution".to_string(), 10),
            ]),
            stats: HashMap::from([("hp".to_string(), 100), ("mp".to_string(), 50)]),
        })
    }

    /// Send a command from a session to the world server
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn send_command(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
        command: String,
    ) -> Result<CommandResult, CommandError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to send command without gateway authentication");
            return Err(CommandError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        tracing::debug!("Command from session {}: {}", session_id, command);

        // Check if session has a character selected and get the EntityId from the mapping
        let entity_id = {
            let active_entities = self.active_entities.read().await;
            active_entities
                .get(&session_id)
                .copied()
                .ok_or_else(|| {
                    tracing::error!(
                        "No active entity found for session {}. Session may need to select a character.",
                        session_id
                    );
                    CommandError::NoCharacterSelected
                })?
        };

        // Process command through the ECS command system if world context is available
        {
            use crate::ecs::events::EventBus;
            use crate::ecs::systems::{CommandResult as EcsCommandResult, CommandSystem};

            let mut world = self.world_context.entities().write().await;

            tracing::debug!(
                "Using entity {:?} (UUID: {}) for command from session {}",
                entity_id.entity(),
                entity_id.uuid(),
                session_id
            );

            // Verify the entity still exists in the world
            if !world.contains(entity_id.entity()) {
                tracing::error!(
                    "Entity {:?} (UUID: {}) for session {} no longer exists in ECS world",
                    entity_id.entity(),
                    entity_id.uuid(),
                    session_id
                );
                return Err(CommandError::ServerError(
                    "Character entity no longer exists".to_string(),
                ));
            }

            // Parse command and arguments
            let parts: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();

            if parts.is_empty() {
                return Ok(CommandResult {
                    success: false,
                    output: vec![],
                    error: Some("Empty command".to_string()),
                });
            }

            let cmd = &parts[0];
            let args = &parts[1..];

            // Create command system and execute
            let event_bus = EventBus::new();
            let mut cmd_system = CommandSystem::new(event_bus);
            drop(world); // Release write lock before calling execute
            let result = cmd_system.execute(self.world_context.clone(), entity_id.entity(), cmd, args).await;
            let mut world = self.world_context.entities().write().await; // Re-acquire for later use

            // Convert ECS command result to RPC command result
            match result {
                EcsCommandResult::Success(text) => {
                    // Check if this is an exit command
                    if text.contains("[EXIT_TO_CHARACTER_SELECTION]") {
                        // Save the character before unloading
                        if let Err(e) = self
                            .world_context
                            .persistence_manager()
                            .save_character(&world, entity_id.entity())
                            .await
                        {
                            tracing::error!("Failed to save character on exit: {}", e);
                            return Ok(CommandResult {
                                success: false,
                                output: vec![GameOutput::Text(format!(
                                    "Failed to save character: {}",
                                    e
                                ))],
                                error: Some("Save failed".to_string()),
                            });
                        }

                        // Remove the entity from active entities to unload it
                        drop(world);
                        let mut active_entities = self.active_entities.write().await;
                        active_entities.remove(&session_id);
                        drop(active_entities);

                        tracing::info!(
                            "Character unloaded for session {} - returning to character selection",
                            session_id
                        );

                        // Return the message without the marker
                        let clean_text = text
                            .replace("[EXIT_TO_CHARACTER_SELECTION]", "")
                            .trim()
                            .to_string();
                        Ok(CommandResult {
                            success: true,
                            output: vec![
                                GameOutput::Text(clean_text),
                                GameOutput::System(
                                    "Returning to character selection...".to_string(),
                                ),
                            ],
                            error: None,
                        })
                    } else {
                        Ok(CommandResult {
                            success: true,
                            output: vec![GameOutput::Text(text)],
                            error: None,
                        })
                    }
                }
                EcsCommandResult::Failure(text) => Ok(CommandResult {
                    success: false,
                    output: vec![GameOutput::Text(text)],
                    error: Some("Command failed".to_string()),
                }),
                EcsCommandResult::Invalid(text) => Ok(CommandResult {
                    success: false,
                    output: vec![GameOutput::Text(text)],
                    error: Some("Invalid command".to_string()),
                }),
            }
        }
    }

    /// Notify server of session disconnection
    async fn session_disconnected(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
        reason: DisconnectReason,
    ) {
        tracing::info!("Session {} disconnected: {:?}", session_id, reason);

        // Remove the entity mapping to free up the ECS entity
        // (session state is kept for potential reconnection)
        let mut active_entities = self.active_entities.write().await;
        if let Some(entity) = active_entities.remove(&session_id) {
            tracing::info!(
                "Removed entity mapping for disconnected session {} (entity: {:?})",
                session_id,
                entity
            );
        }
    }

    /// Notify server of session reconnection
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn session_reconnected(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
        entity_id: PersistentEntityId,
    ) -> Result<ReconnectResult, ReconnectError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to reconnect session without gateway authentication");
            return Err(ReconnectError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        tracing::info!(
            "Session {} reconnecting with entity {}",
            session_id,
            entity_id
        );

        // Get session state
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or(ReconnectError::SessionNotFound)?;

        // Verify entity ID matches
        if session.entity_id.as_ref() != Some(&entity_id) {
            return Err(ReconnectError::CharacterNotFound);
        }

        // Get queued events
        let queued_events = std::mem::take(&mut session.queued_events);

        // TODO: Load current character state from ECS
        let character_state = Some(CharacterInfo {
            entity_id: entity_id.clone(),
            name: "Test Character".to_string(),
            level: 1,
            race: "Human".to_string(),
            class: "Warrior".to_string(),
            location: "Starting Area".to_string(),
            attributes: HashMap::new(),
            stats: HashMap::from([("hp".to_string(), 100), ("mp".to_string(), 50)]),
        });

        Ok(ReconnectResult {
            success: true,
            queued_events,
            character_state,
        })
    }

    /// Get the list of characters for an authenticated session
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn list_characters(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
    ) -> Result<Vec<CharacterSummary>, CharacterError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to list characters without gateway authentication");
            return Err(CharacterError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        tracing::debug!("Listing characters for session {}", session_id);

        // Check if session is authenticated
        let sessions = self.sessions.read().await;
        if !sessions.get(&session_id).map_or(false, |s| s.authenticated) {
            return Err(CharacterError::NotAuthenticated);
        }

        // TODO: Load real characters from database
        // For now, return mock data
        Ok(vec![
            CharacterSummary {
                entity_id: uuid::Uuid::new_v4().to_string(),
                name: "Test Warrior".to_string(),
                level: 5,
                race: "Human".to_string(),
                class: "Warrior".to_string(),
                last_played: "2025-12-18T12:00:00Z".to_string(),
            },
            CharacterSummary {
                entity_id: uuid::Uuid::new_v4().to_string(),
                name: "Test Mage".to_string(),
                level: 3,
                race: "Elf".to_string(),
                class: "Mage".to_string(),
                last_played: "2025-12-17T18:30:00Z".to_string(),
            },
        ])
    }

    /// Heartbeat to keep session alive
    #[tracing::instrument(level = "debug", skip(self, _context))]
    async fn heartbeat(
        self,
        _context: tarpc::context::Context,
        session_id: SessionId,
    ) -> Result<(), SessionError> {
        // Check gateway authentication first
        if !self.is_authenticated().await {
            tracing::warn!("Attempt to send heartbeat without gateway authentication");
            return Err(SessionError::ServerError(
                "Gateway not authenticated".to_string(),
            ));
        }

        // Check if session exists
        let sessions = self.sessions.read().await;

        if let Some(session_state) = sessions.get(&session_id) {
            tracing::debug!(
                "Heartbeat received from session {} (authenticated: {}, entity_id: {:?})",
                session_id,
                session_state.authenticated,
                session_state.entity_id
            );
            Ok(())
        } else {
            tracing::warn!("Heartbeat from unknown session {}", session_id);
            Err(SessionError::NotFound)
        }
    }

    /// Gateway-level heartbeat (independent of sessions)
    async fn gateway_heartbeat(
        self,
        _context: tarpc::context::Context,
        gateway_id: String,
    ) -> Result<(), String> {
        tracing::debug!("Gateway heartbeat received from gateway: {}", gateway_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Write Tests
}



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

//! Gateway-to-Server Communication Protocol
//!
//! This module defines the RPC protocol for communication between the gateway
//! and the world server using tarpc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session identifier (UUID as string for serialization)
pub type SessionId = String;

/// Persistent entity UUID (as string for RPC serialization)
/// This represents the stable, database-backed UUID for entities
pub type PersistentEntityId = String;

/// Gateway-to-Server RPC service
///
/// This service defines all operations that the gateway can perform on the world server.
#[tarpc::service]
pub trait GatewayServer {
    /// Authenticate the gateway connection with an auth key
    ///
    /// This must be called before any other RPC methods.
    ///
    /// # Arguments
    /// * `auth_key` - The authentication key for gateway-to-server communication
    ///
    /// # Returns
    /// * `Ok(())` - Authentication successful
    /// * `Err(String)` - Authentication failed with error message
    async fn authenticate_gateway(auth_key: String) -> Result<(), String>;
    
    /// Authenticate a session with credentials
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `username` - The username to authenticate
    /// * `password` - The password (should be hashed by gateway)
    ///
    /// # Returns
    /// * `Ok(AuthResult)` - Authentication result with entity ID if successful
    async fn authenticate(
        session_id: SessionId,
        username: String,
        password: String,
    ) -> Result<AuthResult, AuthError>;

    /// Create a new character for an authenticated session
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `character_name` - The desired character name
    /// * `character_data` - Character creation data (race, class, attributes, etc.)
    ///
    /// # Returns
    /// * `Ok(PersistentEntityId)` - The newly created character's entity ID
    async fn create_character(
        session_id: SessionId,
        character_name: String,
        character_data: CharacterCreationData,
    ) -> Result<PersistentEntityId, CharacterError>;

    /// Select a character for a session
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `entity_id` - The character's entity ID to select
    ///
    /// # Returns
    /// * `Ok(CharacterInfo)` - Character information and initial game state
    async fn select_character(
        session_id: SessionId,
        entity_id: PersistentEntityId,
    ) -> Result<CharacterInfo, CharacterError>;

    /// Send a command from a session to the world server
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `command` - The command string to execute
    ///
    /// # Returns
    /// * `Ok(CommandResult)` - The result of command execution
    async fn send_command(
        session_id: SessionId,
        command: String,
    ) -> Result<CommandResult, CommandError>;

    /// Notify server of session disconnection
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `reason` - The reason for disconnection
    async fn session_disconnected(session_id: SessionId, reason: DisconnectReason);

    /// Notify server of session reconnection
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `entity_id` - The character's entity ID
    ///
    /// # Returns
    /// * `Ok(ReconnectResult)` - Reconnection result with queued events
    async fn session_reconnected(
        session_id: SessionId,
        entity_id: PersistentEntityId,
    ) -> Result<ReconnectResult, ReconnectError>;

    /// Get the list of characters for an authenticated session
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    ///
    /// # Returns
    /// * `Ok(Vec<CharacterSummary>)` - List of available characters
    async fn list_characters(session_id: SessionId) -> Result<Vec<CharacterSummary>, CharacterError>;

    /// Heartbeat to keep session alive
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    ///
    /// # Returns
    /// * `Ok(())` - Heartbeat acknowledged
    async fn heartbeat(session_id: SessionId) -> Result<(), SessionError>;
    
    /// Gateway-level heartbeat (independent of sessions)
    ///
    /// # Arguments
    /// * `gateway_id` - The gateway identifier
    ///
    /// # Returns
    /// * `Ok(())` - Heartbeat acknowledged
    async fn gateway_heartbeat(gateway_id: String) -> Result<(), String>;
}

/// Server-to-Gateway RPC service
///
/// This service defines callbacks that the world server can make to the gateway.
#[tarpc::service]
pub trait ServerGateway {
    /// Send output to a session
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `output` - The output to send to the client
    async fn send_output(session_id: SessionId, output: GameOutput);

    /// Send a prompt to a session
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `prompt` - The prompt text
    async fn send_prompt(session_id: SessionId, prompt: String);

    /// Notify gateway of entity state change
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `state_update` - The state update information
    async fn entity_state_changed(session_id: SessionId, state_update: EntityStateUpdate);

    /// Request session disconnection
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `reason` - The reason for disconnection
    async fn disconnect_session(session_id: SessionId, reason: String);
}

// ============================================================================
// Authentication Types
// ============================================================================

/// Authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// Whether authentication was successful
    pub success: bool,
    
    /// The authenticated entity ID (if successful)
    pub entity_id: Option<PersistentEntityId>,
    
    /// Authentication message
    pub message: String,
}

/// Authentication error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthError {
    /// Invalid credentials
    InvalidCredentials,
    
    /// Account locked
    AccountLocked,
    
    /// Session not found
    SessionNotFound,
    
    /// Already authenticated
    AlreadyAuthenticated,
    
    /// Server error
    ServerError(String),
}

// ============================================================================
// Character Types
// ============================================================================

/// Character creation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCreationData {
    /// Character race
    pub race: String,
    
    /// Character class
    pub class: String,
    
    /// Character attributes
    pub attributes: HashMap<String, i32>,
    
    /// Character description
    pub description: String,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Character information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    /// Entity ID
    pub entity_id: PersistentEntityId,
    
    /// Character name
    pub name: String,
    
    /// Character level
    pub level: u32,
    
    /// Character race
    pub race: String,
    
    /// Character class
    pub class: String,
    
    /// Current location description
    pub location: String,
    
    /// Character attributes
    pub attributes: HashMap<String, i32>,
    
    /// Character stats (HP, MP, etc.)
    pub stats: HashMap<String, i32>,
}

/// Character summary for character selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSummary {
    /// Entity ID
    pub entity_id: PersistentEntityId,
    
    /// Character name
    pub name: String,
    
    /// Character level
    pub level: u32,
    
    /// Character race
    pub race: String,
    
    /// Character class
    pub class: String,
    
    /// Last played timestamp
    pub last_played: String,
}

/// Character error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterError {
    /// Character not found
    NotFound,
    
    /// Character name already taken
    NameTaken,
    
    /// Invalid character data
    InvalidData(String),
    
    /// Not authenticated
    NotAuthenticated,
    
    /// Permission denied
    PermissionDenied,
    
    /// Server error
    ServerError(String),
}

// ============================================================================
// Command Types
// ============================================================================

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Whether the command was successful
    pub success: bool,
    
    /// Command output
    pub output: Vec<GameOutput>,
    
    /// Error message (if unsuccessful)
    pub error: Option<String>,
}

/// Command error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandError {
    /// Invalid command syntax
    InvalidSyntax,
    
    /// Command not found
    NotFound,
    
    /// Permission denied
    PermissionDenied,
    
    /// Character not selected
    NoCharacterSelected,
    
    /// Server error
    ServerError(String),
}

// ============================================================================
// Game Output Types
// ============================================================================

/// Game output to send to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameOutput {
    /// Plain text output
    Text(String),
    
    /// Formatted text with ANSI codes
    FormattedText(String),
    
    /// Structured data (for GUI clients)
    Structured(StructuredOutput),
    
    /// Room description
    RoomDescription(RoomDescription),
    
    /// Combat message
    Combat(CombatMessage),
    
    /// System message
    System(String),
}

/// Structured output for GUI clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutput {
    /// Output type
    pub output_type: String,
    
    /// Output data
    pub data: HashMap<String, serde_json::Value>,
}

/// Room description
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDescription {
    /// Room name
    pub name: String,
    
    /// Room description
    pub description: String,
    
    /// Visible exits
    pub exits: Vec<String>,
    
    /// Visible entities
    pub entities: Vec<String>,
    
    /// Visible items
    pub items: Vec<String>,
}

/// Combat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatMessage {
    /// Attacker name
    pub attacker: String,
    
    /// Defender name
    pub defender: String,
    
    /// Action description
    pub action: String,
    
    /// Damage dealt
    pub damage: Option<i32>,
    
    /// Whether the attack was critical
    pub critical: bool,
}

// ============================================================================
// Session Types
// ============================================================================

/// Disconnect reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// Client disconnected normally
    ClientDisconnect,
    
    /// Connection timeout
    Timeout,
    
    /// Network error
    NetworkError,
    
    /// Server shutdown
    ServerShutdown,
    
    /// Kicked by admin
    Kicked(String),
}

/// Reconnection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectResult {
    /// Whether reconnection was successful
    pub success: bool,
    
    /// Queued game events during disconnection
    pub queued_events: Vec<GameOutput>,
    
    /// Current character state
    pub character_state: Option<CharacterInfo>,
}

/// Reconnection error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconnectError {
    /// Session not found
    SessionNotFound,
    
    /// Session expired
    SessionExpired,
    
    /// Character not found
    CharacterNotFound,
    
    /// Server error
    ServerError(String),
}

/// Session error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionError {
    /// Session not found
    NotFound,
    
    /// Session expired
    Expired,
    
    /// Server error
    ServerError(String),
}

// ============================================================================
// Entity State Types
// ============================================================================

/// Entity state update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStateUpdate {
    /// Entity ID
    pub entity_id: PersistentEntityId,
    
    /// Update type
    pub update_type: StateUpdateType,
    
    /// Updated data
    pub data: HashMap<String, serde_json::Value>,
}

/// State update type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateUpdateType {
    /// Health/stats changed
    Stats,
    
    /// Position changed
    Position,
    
    /// Inventory changed
    Inventory,
    
    /// Equipment changed
    Equipment,
    
    /// Status effects changed
    StatusEffects,
    
    /// Custom update
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_result_serialization() {
        let result = AuthResult {
            success: true,
            entity_id: Some("test-entity-id".to_string()),
            message: "Authentication successful".to_string(),
        };
        
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: AuthResult = serde_json::from_str(&json).unwrap();
        
        assert_eq!(result.success, deserialized.success);
        assert_eq!(result.entity_id, deserialized.entity_id);
        assert_eq!(result.message, deserialized.message);
    }

    #[test]
    fn test_game_output_variants() {
        let outputs = vec![
            GameOutput::Text("Hello".to_string()),
            GameOutput::System("System message".to_string()),
        ];
        
        for output in outputs {
            let json = serde_json::to_string(&output).unwrap();
            let _deserialized: GameOutput = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_character_creation_data() {
        let mut attributes = HashMap::new();
        attributes.insert("strength".to_string(), 10);
        attributes.insert("dexterity".to_string(), 12);
        
        let data = CharacterCreationData {
            race: "Human".to_string(),
            class: "Warrior".to_string(),
            attributes,
            description: "A brave warrior".to_string(),
            metadata: HashMap::new(),
        };
        
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: CharacterCreationData = serde_json::from_str(&json).unwrap();
        
        assert_eq!(data.race, deserialized.race);
        assert_eq!(data.class, deserialized.class);
    }
}


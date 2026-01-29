# Character Creation Architecture Refactor

## Overview

This document outlines the implementation plan to refactor character creation from a gateway-centric to a server-centric architecture. This addresses architectural issues where game logic is currently split between gateway and server.

**Status**: Planning  
**Priority**: High  
**Estimated Effort**: 3-5 days  
**Created**: 2026-01-28

## Current Architecture Problems

### Issues
1. **CharacterBuilder** lives in `common/src/character.rs` and is used by gateway
2. Gateway enforces game rules (point costs, validation)
3. Complex data transformation from CharacterBuilder → CharacterCreationData → ECS components
4. Duplicate attribute definitions in common and server
5. Security: gateway is trusted to enforce rules without server validation
6. Multiple gateway implementations would duplicate logic

### Current Flow
```
Gateway (Telnet)
  ├─ Maintains CharacterBuilder state
  ├─ Enforces point costs & validation
  ├─ Renders character sheet UI
  └─ Converts to CharacterCreationData
       ↓ RPC: create_character()
Server
  ├─ Receives flat CharacterCreationData
  ├─ Reconstructs character state
  └─ Creates ECS entity
```

## Target Architecture

### Principles
1. **Server is authoritative** for all game logic
2. **Gateway is presentation layer** only
3. **Incremental validation** via RPC
4. **Single source of truth** for character state

### Target Flow
```
Gateway (Telnet/Web/Mobile)
  ├─ Collects user input
  ├─ Sends incremental actions via RPC
  ├─ Receives validation & updated state
  └─ Renders UI from server state
       ↓ RPC: character_creation_*()
Server
  ├─ Maintains CharacterBuilder per session
  ├─ Validates each action
  ├─ Enforces game rules
  └─ Returns updated state
```

## Implementation Plan

### Phase 0: gRPC Infrastructure (Optional but Recommended)

**File**: `common/proto/gateway.proto` (new file)

Define the gRPC service and messages:

```protobuf
syntax = "proto3";

package wyldlands.gateway;

// Gateway-to-Server service
service GatewayServer {
  // Authenticate gateway connection
  rpc AuthenticateGateway(AuthenticateGatewayRequest) returns (AuthenticateGatewayResponse);
  
  // Authenticate user session
  rpc Authenticate(AuthenticateRequest) returns (AuthenticateResponse);
  
  // Character creation command-based interface
  rpc CharacterCreationCommand(CharacterCreationCommandRequest) returns (CharacterCreationCommandResponse);
  
  // Select character for play
  rpc SelectCharacter(SelectCharacterRequest) returns (SelectCharacterResponse);
  
  // Send gameplay command
  rpc SendCommand(SendCommandRequest) returns (SendCommandResponse);
  
  // Session lifecycle
  rpc SessionDisconnected(SessionDisconnectedRequest) returns (Empty);
  rpc SessionReconnected(SessionReconnectedRequest) returns (SessionReconnectedResponse);
  rpc Heartbeat(HeartbeatRequest) returns (Empty);
}

// Server-to-Gateway service (for callbacks)
service ServerGateway {
  // Send output to client
  rpc SendOutput(SendOutputRequest) returns (Empty);
  
  // Send prompt to client
  rpc SendPrompt(SendPromptRequest) returns (Empty);
  
  // Notify of entity state change
  rpc EntityStateChanged(EntityStateChangedRequest) returns (Empty);
  
  // Request session disconnect
  rpc DisconnectSession(DisconnectSessionRequest) returns (Empty);
}

// Character creation command
message CharacterCreationCommandRequest {
  string session_id = 1;
  
  oneof command {
    StartCommand start = 2;
    ModifyAttributeCommand modify_attribute = 3;
    ModifyTalentCommand modify_talent = 4;
    ModifySkillCommand modify_skill = 5;
    SetStartingLocationCommand set_starting_location = 6;
    GetStateCommand get_state = 7;
    FinalizeCommand finalize = 8;
    CancelCommand cancel = 9;
  }
}

message StartCommand {
  string character_name = 1;
}

message ModifyAttributeCommand {
  AttributeType attribute = 1;
  int32 delta = 2;
}

message ModifyTalentCommand {
  Talent talent = 1;
  bool add = 2;
}

message ModifySkillCommand {
  string skill_name = 1;
  int32 delta = 2;
}

message SetStartingLocationCommand {
  string location_id = 1;
}

message GetStateCommand {}
message FinalizeCommand {}
message CancelCommand {}

// Character creation response
message CharacterCreationCommandResponse {
  bool success = 1;
  optional CharacterBuilderState state = 2;
  optional string entity_id = 3;
  optional string error = 4;
  optional string message = 5;
}

// Character builder state
message CharacterBuilderState {
  string name = 1;
  map<string, int32> attributes = 2;
  repeated string talents = 3;
  map<string, int32> skills = 4;
  int32 attribute_talent_points = 5;
  int32 skill_points = 6;
  int32 max_attribute_talent_points = 7;
  int32 max_skill_points = 8;
  optional string starting_location_id = 9;
  repeated string validation_errors = 10;
  bool is_valid = 11;
}

// Enums
enum AttributeType {
  ATTRIBUTE_TYPE_UNSPECIFIED = 0;
  BODY_OFFENCE = 1;
  BODY_FINESSE = 2;
  BODY_DEFENCE = 3;
  MIND_OFFENCE = 4;
  MIND_FINESSE = 5;
  MIND_DEFENCE = 6;
  SOUL_OFFENCE = 7;
  SOUL_FINESSE = 8;
  SOUL_DEFENCE = 9;
}

enum Talent {
  TALENT_UNSPECIFIED = 0;
  WEAPON_MASTER = 1;
  SHIELD_EXPERT = 2;
  DUAL_WIELDER = 3;
  BERSERKER = 4;
  TACTICIAN = 5;
  SPELLWEAVER = 6;
  ELEMENTAL_AFFINITY = 7;
  ARCANE_SCHOLAR = 8;
  RITUALIST = 9;
  CHANNELER = 10;
  ASTRAL_PROJECTION = 11;
  MASTER_CRAFTSMAN = 12;
  ARTIFICER = 13;
  ALCHEMIST = 14;
  ENCHANTER = 15;
  DIPLOMAT = 16;
  MERCHANT = 17;
  LEADER = 18;
  PERFORMER = 19;
  TRACKER = 20;
  FORAGER = 21;
  BEAST_MASTER = 22;
  SURVIVALIST = 23;
  PRODIGY = 24;
  LUCKY = 25;
  FAST_LEARNER = 26;
  RESILIENT = 27;
}

message Empty {}

// ... other message types for remaining RPC methods ...
```

**File**: `common/build.rs` (new file)

Set up protobuf code generation:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["proto/gateway.proto"],
            &["proto"],
        )?;
    Ok(())
}
```

**File**: `common/Cargo.toml`

Add gRPC dependencies:

```toml
[dependencies]
tonic = "0.11"
prost = "0.12"
prost-types = "0.12"

[build-dependencies]
tonic-build = "0.11"
```

**Benefits of gRPC Approach**:
- Schema-first design with `.proto` files
- Automatic code generation for client and server
- Built-in versioning and backward compatibility
- Can generate clients in other languages (Python, Go, etc.)
- Better tooling (grpcurl, grpc-web, Postman support)
- Industry standard with extensive documentation

---

### Phase 1: Add New RPC Methods (Server-Side)

**Option A: Using gRPC (Current)**

**File**: `common/src/gateway.rs`

Add new command-based RPC method to `GatewayServer` trait:

```rust
/// Character creation session management
#[tonic::async_trait]
pub trait GatewayServer {
    // ... existing methods ...
    
    /// Execute a character creation command
    ///
    /// This is a command-based interface where the gateway sends commands
    /// and the server interprets and executes them, returning updated state.
    ///
    /// # Arguments
    /// * `session_id` - The session identifier
    /// * `command` - The character creation command to execute
    ///
    /// # Returns
    /// * `Ok(CharacterCreationResponse)` - Command result with updated state
    async fn character_creation_command(
        session_id: SessionId,
        command: CharacterCreationCommand,
    ) -> Result<CharacterCreationResponse, CharacterError>;
}
```

Add new command and response types to `common/src/gateway.rs`:

```rust
/// Character creation command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CharacterCreationCommand {
    /// Start a new character creation session
    Start {
        character_name: String,
    },
    
    /// Modify an attribute
    ModifyAttribute {
        attribute: AttributeType,
        delta: i32,
    },
    
    /// Add or remove a talent
    ModifyTalent {
        talent: Talent,
        add: bool,
    },
    
    /// Modify a skill
    ModifySkill {
        skill_name: String,
        delta: i32,
    },
    
    /// Set starting location
    SetStartingLocation {
        location_id: String,
    },
    
    /// Get current state without modifications
    GetState,
    
    /// Finalize and create the character
    Finalize,
    
    /// Cancel character creation
    Cancel,
}

/// Character creation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCreationResponse {
    /// Whether the command was successful
    pub success: bool,
    
    /// Current character builder state (if applicable)
    pub state: Option<CharacterBuilderState>,
    
    /// Created entity ID (for Finalize command)
    pub entity_id: Option<PersistentEntityId>,
    
    /// Error message (if unsuccessful)
    pub error: Option<String>,
    
    /// Informational message
    pub message: Option<String>,
}

/// Character builder state (sent from server to gateway)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBuilderState {
    /// Character name
    pub name: String,
    
    /// Current attribute values (attribute name -> value)
    pub attributes: HashMap<String, i32>,
    
    /// Selected talents (talent names)
    pub talents: Vec<String>,
    
    /// Current skill values (skill name -> rank)
    pub skills: HashMap<String, i32>,
    
    /// Available attribute/talent points
    pub attribute_talent_points: i32,
    
    /// Available skill points
    pub skill_points: i32,
    
    /// Maximum attribute/talent points
    pub max_attribute_talent_points: i32,
    
    /// Maximum skill points
    pub max_skill_points: i32,
    
    /// Selected starting location ID
    pub starting_location_id: Option<String>,
    
    /// Validation errors (if any)
    pub validation_errors: Vec<String>,
    
    /// Whether the character is valid for creation
    pub is_valid: bool,
}

/// Re-export character types from common
pub use wyldlands_common::character::{AttributeType, Talent};
```

**Benefits of Command-Based Approach:**

1. **Single RPC endpoint** - Simpler protocol, easier to version
2. **Extensible** - Add new commands without changing RPC interface
3. **Consistent pattern** - Similar to existing `send_command` for gameplay
4. **Easier to log/audit** - All character creation actions go through one method
5. **Simpler gateway code** - One RPC call pattern for all actions
6. **Better for batching** - Could extend to support multiple commands in future

### Phase 2: Move CharacterBuilder to Server

**File**: `server/src/ecs/character_builder.rs` (new file)

```rust
//! Server-side character builder
//!
//! This module contains the authoritative character creation logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wyldlands_common::character::{
    AttributeType, Talent, attribute_cost, skill_cost, 
    total_attribute_cost, total_skill_cost,
};
use wyldlands_common::gateway::CharacterBuilderState;

/// Server-side character builder
///
/// This is the authoritative source for character creation state.
/// All validation and game rules are enforced here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCharacterBuilder {
    /// Character name
    pub name: String,
    
    /// Attribute allocations (rank 0-20)
    pub attributes: HashMap<AttributeType, i32>,
    
    /// Selected talents
    pub talents: Vec<Talent>,
    
    /// Skill allocations (rank 0-10)
    pub skills: HashMap<String, i32>,
    
    /// Available points for attributes and talents (shared pool)
    pub attribute_talent_points: i32,
    
    /// Available points for skills (separate pool)
    pub skill_points: i32,
    
    /// Maximum attribute/talent points (from config)
    pub max_attribute_talent_points: i32,
    
    /// Maximum skill points (from config)
    pub max_skill_points: i32,
    
    /// Selected starting location ID
    pub starting_location_id: Option<String>,
}

impl ServerCharacterBuilder {
    /// Create a new character builder with configured point pools
    pub fn new(
        name: String,
        max_attribute_talent_points: i32,
        max_skill_points: i32,
    ) -> Self {
        let mut attributes = HashMap::new();
        // Initialize all attributes to 10 (base value)
        for attr in AttributeType::all() {
            attributes.insert(attr, 10);
        }
        
        Self {
            name,
            attributes,
            talents: Vec::new(),
            skills: HashMap::new(),
            attribute_talent_points: max_attribute_talent_points,
            skill_points: max_skill_points,
            max_attribute_talent_points,
            max_skill_points,
            starting_location_id: None,
        }
    }
    
    /// Set the starting location
    pub fn set_starting_location(&mut self, location_id: String) {
        self.starting_location_id = Some(location_id);
    }
    
    /// Get current attribute rank
    pub fn get_attribute(&self, attr: AttributeType) -> i32 {
        *self.attributes.get(&attr).unwrap_or(&10)
    }
    
    /// Try to increase an attribute by delta
    pub fn modify_attribute(&mut self, attr: AttributeType, delta: i32) -> Result<(), String> {
        if delta == 0 {
            return Ok(());
        }
        
        let current = self.get_attribute(attr);
        let new_value = current + delta;
        
        if new_value < 10 {
            return Err("Attribute cannot go below 10".to_string());
        }
        if new_value > 20 {
            return Err("Attribute cannot exceed 20".to_string());
        }
        
        // Calculate cost difference
        let cost_diff = if delta > 0 {
            // Increasing: sum costs from current+1 to new_value
            (current + 1..=new_value).map(attribute_cost).sum()
        } else {
            // Decreasing: refund costs from new_value+1 to current
            -((new_value + 1..=current).map(attribute_cost).sum::<i32>())
        };
        
        if cost_diff > self.attribute_talent_points {
            return Err(format!("Not enough points. Need {} points.", cost_diff));
        }
        
        self.attributes.insert(attr, new_value);
        self.attribute_talent_points -= cost_diff;
        Ok(())
    }
    
    /// Try to add or remove a talent
    pub fn modify_talent(&mut self, talent: Talent, add: bool) -> Result<(), String> {
        if add {
            if self.talents.contains(&talent) {
                return Err("Talent already selected".to_string());
            }
            
            let cost = talent.cost();
            if self.attribute_talent_points < cost {
                return Err(format!("Not enough points. Need {} points.", cost));
            }
            
            self.talents.push(talent);
            self.attribute_talent_points -= cost;
        } else {
            if let Some(pos) = self.talents.iter().position(|t| *t == talent) {
                self.talents.remove(pos);
                self.attribute_talent_points += talent.cost();
            } else {
                return Err("Talent not selected".to_string());
            }
        }
        Ok(())
    }
    
    /// Get current skill rank
    pub fn get_skill(&self, skill: &str) -> i32 {
        *self.skills.get(skill).unwrap_or(&0)
    }
    
    /// Try to modify a skill by delta
    pub fn modify_skill(&mut self, skill: String, delta: i32) -> Result<(), String> {
        if delta == 0 {
            return Ok(());
        }
        
        let current = self.get_skill(&skill);
        let new_value = current + delta;
        
        if new_value < 0 {
            return Err("Skill cannot go below 0".to_string());
        }
        if new_value > 10 {
            return Err("Skill cannot exceed 10".to_string());
        }
        
        // Calculate cost difference
        let cost_diff = if delta > 0 {
            // Increasing: sum costs from current+1 to new_value
            (current + 1..=new_value).map(skill_cost).sum()
        } else {
            // Decreasing: refund costs from new_value+1 to current
            -((new_value + 1..=current).map(skill_cost).sum::<i32>())
        };
        
        if cost_diff > self.skill_points {
            return Err(format!("Not enough skill points. Need {} points.", cost_diff));
        }
        
        if new_value == 0 {
            self.skills.remove(&skill);
        } else {
            self.skills.insert(skill, new_value);
        }
        self.skill_points -= cost_diff;
        Ok(())
    }
    
    /// Validate the character for creation
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        if self.name.is_empty() {
            errors.push("Character name is required".to_string());
        }
        
        if self.starting_location_id.is_none() {
            errors.push("Starting location must be selected".to_string());
        }
        
        if self.attribute_talent_points < 0 {
            errors.push("Overspent attribute/talent points".to_string());
        }
        
        if self.skill_points < 0 {
            errors.push("Overspent skill points".to_string());
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Convert to gateway state for transmission
    pub fn to_state(&self) -> CharacterBuilderState {
        let validation_errors = match self.validate() {
            Ok(()) => Vec::new(),
            Err(errors) => errors,
        };
        
        let mut attributes = HashMap::new();
        for (attr, value) in &self.attributes {
            attributes.insert(attr.name().to_string(), *value);
        }
        
        let talents = self.talents.iter().map(|t| t.name().to_string()).collect();
        
        CharacterBuilderState {
            name: self.name.clone(),
            attributes,
            talents,
            skills: self.skills.clone(),
            attribute_talent_points: self.attribute_talent_points,
            skill_points: self.skill_points,
            max_attribute_talent_points: self.max_attribute_talent_points,
            max_skill_points: self.max_skill_points,
            starting_location_id: self.starting_location_id.clone(),
            validation_errors,
            is_valid: validation_errors.is_empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modify_attribute() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Increase attribute
        assert!(builder.modify_attribute(AttributeType::BodyOffence, 1).is_ok());
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 11);
        assert_eq!(builder.attribute_talent_points, 47); // 50 - 3 (cost of rank 11)
        
        // Decrease attribute
        assert!(builder.modify_attribute(AttributeType::BodyOffence, -1).is_ok());
        assert_eq!(builder.get_attribute(AttributeType::BodyOffence), 10);
        assert_eq!(builder.attribute_talent_points, 50); // Refunded
    }
    
    #[test]
    fn test_modify_talent() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Add talent
        assert!(builder.modify_talent(Talent::WeaponMaster, true).is_ok());
        assert_eq!(builder.talents.len(), 1);
        assert_eq!(builder.attribute_talent_points, 45); // 50 - 5
        
        // Remove talent
        assert!(builder.modify_talent(Talent::WeaponMaster, false).is_ok());
        assert_eq!(builder.talents.len(), 0);
        assert_eq!(builder.attribute_talent_points, 50); // Refunded
    }
    
    #[test]
    fn test_modify_skill() {
        let mut builder = ServerCharacterBuilder::new("Test".to_string(), 50, 30);
        
        // Increase skill
        assert!(builder.modify_skill("Swordsmanship".to_string(), 1).is_ok());
        assert_eq!(builder.get_skill("Swordsmanship"), 1);
        assert_eq!(builder.skill_points, 29); // 30 - 1
        
        // Decrease skill to 0 removes it
        assert!(builder.modify_skill("Swordsmanship".to_string(), -1).is_ok());
        assert_eq!(builder.get_skill("Swordsmanship"), 0);
        assert!(!builder.skills.contains_key("Swordsmanship"));
        assert_eq!(builder.skill_points, 30); // Refunded
    }
}
```

### Phase 3: Implement Server RPC Handler

**File**: `server/src/listener.rs`

Add character builder session storage:

```rust
/// Character builder sessions (SessionId -> ServerCharacterBuilder)
character_builders: Arc<RwLock<HashMap<SessionId, ServerCharacterBuilder>>>,
```

Implement single command-based RPC method:

```rust
async fn character_creation_command(
    self,
    request: tonic::Request<CharacterCreationCommandRequest>,
    session_id: SessionId,
    command: CharacterCreationCommand,
) -> Result<CharacterCreationResponse, CharacterError> {
    // Check gateway authentication first
    if !self.is_authenticated().await {
        return Err(CharacterError::NotAuthenticated);
    }
    
    use CharacterCreationCommand::*;
    
    match command {
        Start { character_name } => {
            // Validate name
            if character_name.is_empty() || character_name.len() > 50 {
                return Ok(CharacterCreationResponse {
                    success: false,
                    state: None,
                    entity_id: None,
                    error: Some("Invalid character name".to_string()),
                    message: None,
                });
            }
            
            // Get point pools from config (TODO: make configurable)
            let max_attribute_talent_points = 50;
            let max_skill_points = 30;
            
            let builder = ServerCharacterBuilder::new(
                character_name.clone(),
                max_attribute_talent_points,
                max_skill_points,
            );
            
            let state = builder.to_state();
            
            let mut builders = self.character_builders.write().await;
            builders.insert(session_id.clone(), builder);
            
            tracing::info!("Started character creation for session {}: {}", session_id, character_name);
            
            Ok(CharacterCreationResponse {
                success: true,
                state: Some(state),
                entity_id: None,
                error: None,
                message: Some("Character creation started".to_string()),
            })
        }
        
        ModifyAttribute { attribute, delta } => {
            let mut builders = self.character_builders.write().await;
            let builder = builders.get_mut(&session_id).ok_or(
                CharacterError::InvalidData("No character creation in progress".to_string())
            )?;
            
            match builder.modify_attribute(attribute, delta) {
                Ok(()) => {
                    let state = builder.to_state();
                    Ok(CharacterCreationResponse {
                        success: true,
                        state: Some(state),
                        entity_id: None,
                        error: None,
                        message: Some(format!("Modified {} by {}", attribute.name(), delta)),
                    })
                }
                Err(e) => Ok(CharacterCreationResponse {
                    success: false,
                    state: Some(builder.to_state()),
                    entity_id: None,
                    error: Some(e),
                    message: None,
                }),
            }
        }
        
        ModifyTalent { talent, add } => {
            let mut builders = self.character_builders.write().await;
            let builder = builders.get_mut(&session_id).ok_or(
                CharacterError::InvalidData("No character creation in progress".to_string())
            )?;
            
            match builder.modify_talent(talent, add) {
                Ok(()) => {
                    let state = builder.to_state();
                    let action = if add { "Added" } else { "Removed" };
                    Ok(CharacterCreationResponse {
                        success: true,
                        state: Some(state),
                        entity_id: None,
                        error: None,
                        message: Some(format!("{} talent: {}", action, talent.name())),
                    })
                }
                Err(e) => Ok(CharacterCreationResponse {
                    success: false,
                    state: Some(builder.to_state()),
                    entity_id: None,
                    error: Some(e),
                    message: None,
                }),
            }
        }
        
        ModifySkill { skill_name, delta } => {
            let mut builders = self.character_builders.write().await;
            let builder = builders.get_mut(&session_id).ok_or(
                CharacterError::InvalidData("No character creation in progress".to_string())
            )?;
            
            match builder.modify_skill(skill_name.clone(), delta) {
                Ok(()) => {
                    let state = builder.to_state();
                    Ok(CharacterCreationResponse {
                        success: true,
                        state: Some(state),
                        entity_id: None,
                        error: None,
                        message: Some(format!("Modified skill {} by {}", skill_name, delta)),
                    })
                }
                Err(e) => Ok(CharacterCreationResponse {
                    success: false,
                    state: Some(builder.to_state()),
                    entity_id: None,
                    error: Some(e),
                    message: None,
                }),
            }
        }
        
        SetStartingLocation { location_id } => {
            let mut builders = self.character_builders.write().await;
            let builder = builders.get_mut(&session_id).ok_or(
                CharacterError::InvalidData("No character creation in progress".to_string())
            )?;
            
            builder.set_starting_location(location_id.clone());
            let state = builder.to_state();
            
            Ok(CharacterCreationResponse {
                success: true,
                state: Some(state),
                entity_id: None,
                error: None,
                message: Some(format!("Set starting location: {}", location_id)),
            })
        }
        
        GetState => {
            let builders = self.character_builders.read().await;
            let builder = builders.get(&session_id).ok_or(
                CharacterError::InvalidData("No character creation in progress".to_string())
            )?;
            
            Ok(CharacterCreationResponse {
                success: true,
                state: Some(builder.to_state()),
                entity_id: None,
                error: None,
                message: None,
            })
        }
        
        Finalize => {
            let mut builders = self.character_builders.write().await;
            let builder = builders.remove(&session_id).ok_or(
                CharacterError::InvalidData("No character creation in progress".to_string())
            )?;
            
            // Validate
            if let Err(errors) = builder.validate() {
                return Ok(CharacterCreationResponse {
                    success: false,
                    state: Some(builder.to_state()),
                    entity_id: None,
                    error: Some(errors.join(", ")),
                    message: None,
                });
            }
            
            // Create character using existing logic
            // Convert ServerCharacterBuilder to ECS components
            // ... (implementation similar to current create_character)
            
            // For now, return placeholder
            tracing::info!("Finalizing character creation for session {}", session_id);
            
            // TODO: Implement actual character creation
            let entity_id = "placeholder-entity-id".to_string();
            
            Ok(CharacterCreationResponse {
                success: true,
                state: None,
                entity_id: Some(entity_id),
                error: None,
                message: Some("Character created successfully!".to_string()),
            })
        }
        
        Cancel => {
            let mut builders = self.character_builders.write().await;
            builders.remove(&session_id);
            
            tracing::info!("Cancelled character creation for session {}", session_id);
            
            Ok(CharacterCreationResponse {
                success: true,
                state: None,
                entity_id: None,
                error: None,
                message: Some("Character creation cancelled".to_string()),
            })
        }
    }
}
```

### Phase 4: Update Gateway to Use Command-Based RPC

**File**: `gateway/src/telnet/login.rs`

Replace `AvatarCreationBuilder` state with server-backed state:

```rust
/// Interactive character builder (server-backed)
AvatarCreationBuilder {
    account: Account,
    state: CharacterBuilderState,  // Received from server
},
```

Add helper function for sending commands:

```rust
impl LoginHandler {
    /// Send a character creation command to the server
    async fn send_character_command(
        &self,
        command: CharacterCreationCommand,
    ) -> Result<CharacterCreationResponse, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.context.rpc_client()
            .client()
            .await
            .ok_or("RPC client not available")?;
        
        let response = client
            .character_creation_command(
                tonic::Request::new(CharacterCreationCommandRequest {
                    session_id: self.session_id.to_string(),
                command,
            )
            .await??;
        
        Ok(response)
    }
}
```

Update character creation flow:

```rust
LoginState::AvatarCreationName { account } => {
    // ... existing name prompt code ...
    
    if let Some(name) = read_line_from_stream(stream, &mut self.input_buffer).await? {
        let name = name.trim();
        
        if !is_valid_name(name) {
            stream.write_all(b"\r\nInvalid name. Please use only letters and spaces.\r\n").await?;
            self.prompt_shown = false;
            continue;
        }
        
        // Start character creation on server
        match self.send_character_command(CharacterCreationCommand::Start {
            character_name: name.to_string(),
        }).await {
            Ok(response) => {
                if response.success {
                    if let Some(state) = response.state {
                        if let Some(msg) = response.message {
                            stream.write_all(format!("\r\n{}\r\n", msg).as_bytes()).await?;
                        }
                        self.state = LoginState::AvatarCreationBuilder { account, state };
                        self.prompt_shown = false;
                    }
                } else {
                    let error = response.error.unwrap_or_else(|| "Unknown error".to_string());
                    stream.write_all(format!("\r\nError: {}\r\n", error).as_bytes()).await?;
                    self.prompt_shown = false;
                }
            }
            Err(e) => {
                stream.write_all(format!("\r\nError: {}\r\n", e).as_bytes()).await?;
                self.prompt_shown = false;
            }
        }
    }
}

LoginState::AvatarCreationBuilder { account, state } => {
    // Show character sheet from server state
    if !self.prompt_shown {
        let sheet = character_sheet::format_character_sheet_from_state(&state);
        stream.write_all(sheet.as_bytes()).await?;
        stream.flush().await?;
        self.prompt_shown = true;
    }
    
    if let Some(input) = read_line_from_stream(stream, &mut self.input_buffer).await? {
        let input = input.trim();
        
        match input {
            "done" => {
                // Finalize character creation on server
                match self.send_character_command(CharacterCreationCommand::Finalize).await {
                    Ok(response) => {
                        if response.success {
                            if let Some(entity_id) = response.entity_id {
                                // Parse entity_id and link avatar
                                let entity_uuid = uuid::Uuid::parse_str(&entity_id)
                                    .map_err(|e| format!("Invalid entity ID: {}", e))?;
                                
                                let avatar = self.context
                                    .auth_manager()
                                    .link_avatar(account.id, entity_uuid)
                                    .await?;
                                
                                stream.write_all(b"\r\nCharacter created successfully!\r\n").await?;
                                self.state = LoginState::Complete { account, avatar };
                            }
                        } else {
                            let error = response.error.unwrap_or_else(|| "Unknown error".to_string());
                            stream.write_all(format!("\r\nError: {}\r\n", error).as_bytes()).await?;
                            self.prompt_shown = false;
                        }
                    }
                    Err(e) => {
                        stream.write_all(format!("\r\nError: {}\r\n", e).as_bytes()).await?;
                        self.prompt_shown = false;
                    }
                }
            }
            "cancel" => {
                // Cancel on server
                let _ = self.send_character_command(CharacterCreationCommand::Cancel).await;
                self.state = LoginState::AvatarSelection { account };
                self.prompt_shown = false;
            }
            "sheet" => {
                // Refresh state from server
                match self.send_character_command(CharacterCreationCommand::GetState).await {
                    Ok(response) => {
                        if let Some(new_state) = response.state {
                            self.state = LoginState::AvatarCreationBuilder { account, state: new_state };
                            self.prompt_shown = false;
                        }
                    }
                    Err(e) => {
                        stream.write_all(format!("\r\nError: {}\r\n", e).as_bytes()).await?;
                    }
                }
            }
            "talents" => {
                // Show talents list (static, no server call needed)
                let talents = character_sheet::format_talents_list();
                stream.write_all(talents.as_bytes()).await?;
                stream.flush().await?;
            }
            "skills" => {
                // Show skills list (static, no server call needed)
                let skills = character_sheet::format_skills_list();
                stream.write_all(skills.as_bytes()).await?;
                stream.flush().await?;
            }
            _ => {
                // Parse and send command to server
                let command = if let Some((attr, increase)) = character_sheet::parse_attribute_command(input) {
                    let delta = if increase { 1 } else { -1 };
                    Some(CharacterCreationCommand::ModifyAttribute { attribute: attr, delta })
                } else if let Some((add, talent_name)) = character_sheet::parse_talent_command(input) {
                    if let Some(talent) = character_sheet::find_talent_by_name(&talent_name) {
                        Some(CharacterCreationCommand::ModifyTalent { talent, add })
                    } else {
                        stream.write_all(format!("\r\nUnknown talent: {}\r\n", talent_name).as_bytes()).await?;
                        None
                    }
                } else if let Some((skill_name, increase)) = character_sheet::parse_skill_command(input) {
                    let delta = if increase { 1 } else { -1 };
                    Some(CharacterCreationCommand::ModifySkill { skill_name, delta })
                } else {
                    stream.write_all(b"\r\nUnknown command. Type 'sheet' to see available commands.\r\n").await?;
                    None
                };
                
                if let Some(cmd) = command {
                    match self.send_character_command(cmd).await {
                        Ok(response) => {
                            if response.success {
                                if let Some(msg) = response.message {
                                    stream.write_all(format!("\r\n{}\r\n", msg).as_bytes()).await?;
                                }
                                if let Some(new_state) = response.state {
                                    self.state = LoginState::AvatarCreationBuilder { account, state: new_state };
                                    self.prompt_shown = false;
                                }
                            } else {
                                let error = response.error.unwrap_or_else(|| "Unknown error".to_string());
                                stream.write_all(format!("\r\nError: {}\r\n", error).as_bytes()).await?;
                                // Keep current state and re-prompt
                                self.prompt_shown = false;
                            }
                        }
                        Err(e) => {
                            stream.write_all(format!("\r\nError: {}\r\n", e).as_bytes()).await?;
                            self.prompt_shown = false;
                        }
                    }
                }
            }
        }
    }
}
```

### Phase 5: Update Character Sheet Rendering

**File**: `gateway/src/telnet/character_sheet.rs`

Add function to render from server state:

```rust
/// Format the character sheet from server state
pub fn format_character_sheet_from_state(state: &CharacterBuilderState) -> String {
    let mut output = String::new();
    
    output.push_str("\r\n");
    output.push_str("╔════════════════════════════════════════════════════════════════════════════╗\r\n");
    output.push_str(&format!("║ CHARACTER CREATION: {:<58} ║\r\n", state.name));
    output.push_str("╠════════════════════════════════════════════════════════════════════════════╣\r\n");
    
    // Point pools
    output.push_str(&format!(
        "║ Attribute/Talent Points: {}/{:<3}  Skill Points: {}/{:<3}                  ║\r\n",
        state.attribute_talent_points,
        state.max_attribute_talent_points,
        state.skill_points,
        state.max_skill_points
    ));
    
    // Show validation errors if any
    if !state.validation_errors.is_empty() {
        output.push_str("╠════════════════════════════════════════════════════════════════════════════╣\r\n");
        output.push_str("║ VALIDATION ERRORS:                                                         ║\r\n");
        for error in &state.validation_errors {
            output.push_str(&format!("║   • {:<72} ║\r\n", error));
        }
    }
    
    output.push_str("╠════════════════════════════════════════════════════════════════════════════╣\r\n");
    
    // Attributes section (render from state.attributes HashMap)
    // ... similar to existing format_character_sheet but using state data ...
    
    // Talents section (render from state.talents Vec<String>)
    // ... similar to existing format_character_sheet but using state data ...
    
    // Skills section (render from state.skills HashMap)
    // ... similar to existing format_character_sheet but using state data ...
    
    output.push_str("╠════════════════════════════════════════════════════════════════════════════╣\r\n");
    output.push_str("║ COMMANDS:                                                                  ║\r\n");
    output.push_str("║   attr <body|mind|soul> <off|fin|def> <+|->  - Adjust attributes           ║\r\n");
    output.push_str("║   talents                                     - View available talents     ║\r\n");
    output.push_str("║   talent add <name>                           - Add a talent               ║\r\n");
    output.push_str("║   talent remove <name>                        - Remove a talent            ║\r\n");
    output.push_str("║   skills                                      - View available skills      ║\r\n");
    output.push_str("║   skill <name> <+|->                          - Adjust skill               ║\r\n");
    output.push_str("║   done                                        - Finish character creation  ║\r\n");
    output.push_str("║   cancel                                      - Cancel character creation  ║\r\n");
    output.push_str("╚════════════════════════════════════════════════════════════════════════════╝\r\n");
    output.push_str("\r\n> ");
    
    output
}
```

## Migration Steps

### Step 1: Preparation
- [ ] Review and approve this implementation plan
- [ ] Create feature branch: `feature/character-creation-refactor`
- [ ] Add configuration for character point pools to `server/config.yaml`

### Step 2: Server Implementation
- [ ] Add new RPC methods to `common/src/gateway.rs`
- [ ] Create `server/src/ecs/character_builder.rs`
- [ ] Add character builder storage to `server/src/listener.rs`
- [ ] Implement all new RPC handlers
- [ ] Write unit tests for `ServerCharacterBuilder`
- [ ] Write integration tests for RPC methods

### Step 3: Gateway Update
- [ ] Update `gateway/src/telnet/login.rs` to use new RPC methods
- [ ] Update `gateway/src/telnet/character_sheet.rs` for server state rendering
- [ ] Remove local `CharacterBuilder` usage from gateway
- [ ] Test telnet character creation flow

### Step 4: Cleanup
- [ ] Mark old `create_character` RPC method as deprecated
- [ ] Remove `CharacterBuilder` from `common/src/character.rs` (keep only types)
- [ ] Remove `create_avatar_from_builder` from gateway
- [ ] Update documentation

### Step 5: Testing & Validation
- [ ] Manual testing of full character creation flow
- [ ] Test error handling and validation
- [ ] Test session cleanup on disconnect
- [ ] Performance testing with multiple concurrent creations
- [ ] Test reconnection during character creation

### Step 6: Deployment
- [ ] Merge to main branch
- [ ] Deploy server first (backward compatible)
- [ ] Deploy gateway
- [ ] Monitor for issues

## Benefits

### Immediate
- ✅ Server is authoritative for all game rules
- ✅ Gateway is simplified to presentation layer
- ✅ Validation happens server-side
- ✅ Single source of truth for character state

### Long-term
- ✅ Easy to add web/mobile gateways (reuse server logic)
- ✅ Game balance changes don't require gateway updates
- ✅ Character creation can be tested independently
- ✅ Can add features like "save draft" easily
- ✅ Can implement character templates server-side
- ✅ Better security (no client-side rule enforcement)

## Risks & Mitigation

### Risk: Increased RPC calls
**Impact**: More network round-trips for each action  
**Mitigation**: 
- Batch operations where possible
- Optimize RPC serialization
- Add caching for static data (talents, skills)
- Monitor latency in production

### Risk: Server state management complexity
**Impact**: Need to track builder state per session  
**Mitigation**:
- Implement session cleanup on disconnect
- Add timeout for abandoned character creations
- Use existing session management infrastructure

### Risk: Breaking existing character creation
**Impact**: Users in middle of creation could be disrupted  
**Mitigation**:
- Deploy during low-traffic period
- Keep old RPC method temporarily
- Add migration path for in-progress creations

## Future Enhancements

1. **Character Templates**: Server-side presets for quick creation
2. **Draft Saving**: Save incomplete characters to database
3. **Undo/Redo**: Track change history server-side
4. **Validation Preview**: Show what would happen before applying
5. **Point Reallocation**: Allow respec after creation
6. **Web UI**: Build React/Vue character creator using same RPC
7. **Mobile App**: Native mobile character creation

## Questions & Decisions

### Q: Should we switch from tarpc to gRPC?

**Status**: ✅ **COMPLETED** - The project has been successfully migrated from tarpc to gRPC.

**Context**: tarpc was a Rust-native RPC framework, while gRPC is an industry-standard protocol with broader ecosystem support.

**Comparison**:

| Aspect | tarpc | gRPC |
|--------|-------|------|
| **Language Support** | Rust only | Multi-language (Go, Python, Java, etc.) |
| **Protocol** | Custom binary (serde) | Protobuf |
| **Tooling** | Limited | Extensive (grpcurl, grpc-web, etc.) |
| **Performance** | Excellent (native Rust) | Excellent (optimized C core) |
| **Type Safety** | Strong (Rust traits) | Strong (protobuf schemas) |
| **Versioning** | Manual | Built-in (protobuf) |
| **Streaming** | Supported | First-class support |
| **HTTP/2** | Yes | Yes |
| **Browser Support** | No | Yes (grpc-web) |
| **Learning Curve** | Low (if you know Rust) | Medium (protobuf + gRPC) |
| **Maintenance** | Small community | Large ecosystem |

**Pros of Switching to gRPC**:
1. ✅ **Future-proof**: Industry standard, won't be abandoned
2. ✅ **Multi-language**: Could write gateway in Go, Python, or other languages
3. ✅ **Better tooling**: grpcurl for testing, grpc-web for browser clients
4. ✅ **Schema evolution**: Protobuf handles versioning elegantly
5. ✅ **Documentation**: Extensive resources and best practices
6. ✅ **Monitoring**: Better integration with observability tools
7. ✅ **Web support**: Could build web-based character creator easily

**Cons of Switching to gRPC**:
1. ❌ **Migration effort**: Rewrite all RPC definitions and implementations
2. ❌ **Protobuf overhead**: Need to maintain .proto files
3. ❌ **Code generation**: Build process becomes more complex
4. ❌ **Less Rust-native**: Loses some Rust ergonomics
5. ❌ **Breaking change**: All existing RPC calls need updating

**Original Recommendation**:

**YES, switch to gRPC** - This recommendation was implemented successfully. The migration was completed as part of the refactor for these reasons:

1. **Already breaking changes**: We're redesigning the character creation protocol anyway
2. **Web gateway planned**: gRPC-web would enable browser-based character creation
3. **Better long-term**: More maintainable as project grows
4. **Cleaner migration**: Do it once during major refactor vs. later disruption
5. **Command pattern fits**: gRPC services work well with command-based APIs

**Implementation Strategy**:

1. **Phase 0**: Add gRPC infrastructure
   - Add `tonic` (Rust gRPC) and `prost` (protobuf) dependencies
   - Create `.proto` files for gateway protocol
   - Set up code generation in build process
   - ✅ Completed: tarpc has been fully removed

2. **Phase 1-5**: Implement character creation with gRPC
   - ✅ Completed: Character creation now uses gRPC

3. **Phase 6**: Migrate remaining RPC methods
   - ✅ Completed: All RPC methods converted to gRPC
   - ✅ Completed: tarpc endpoints deprecated and removed
   - ✅ Completed: tarpc fully removed from codebase

**Migration Completed**: The gRPC migration has been successfully completed. All code and documentation now use gRPC/tonic instead of tarpc.

---

### Q: Should we keep old `create_character` RPC method?
**A**: Yes, mark as deprecated but keep for backward compatibility during transition.

### Q: How to handle sessions that disconnect during creation?
**A**: Store builder state in session metadata, allow resumption on reconnect.

### Q: Should point pools be configurable per account/character?
**A**: Start with global config, add per-account customization later if needed.

### Q: How to handle concurrent modifications?
**A**: Use RwLock per session, only one modification at a time per session.

## Success Criteria

- [ ] All character creation happens via new RPC methods
- [ ] Gateway contains no game logic
- [ ] Server validates all modifications
- [ ] Character creation works in telnet
- [ ] All tests pass
- [ ] No performance regression
- [ ] Documentation updated

## Timeline

- **Day 1**: Phase 1 & 2 (RPC methods, ServerCharacterBuilder)
- **Day 2**: Phase 3 (Server RPC handlers, tests)
- **Day 3**: Phase 4 (Gateway updates)
- **Day 4**: Phase 5 (Character sheet rendering)
- **Day 5**: Testing, cleanup, documentation

**Total**: 5 days

## References

- [Gateway Protocol Documentation](./GATEWAY_PROTOCOL.md)
- [ECS Components](../../server/src/ecs/components/)
- [Current Character Builder](../../common/src/character.rs)
- [Bob Findings Panel Issues](#) - See architectural review findings
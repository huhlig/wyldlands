# Session State Engine Architecture

**Last Updated**: February 1, 2026
**Status**: ✅ Complete - Layered State Machine Implementation

## Overview

Wyldlands uses a **layered state machine architecture** that separates connection-level concerns (gateway) from game-level concerns (server). This design provides clean separation of responsibilities, protocol independence, and easier testing.

## Architecture Principles

### Separation of Concerns

- **Gateway**: Handles protocol-specific details, authentication flow, and input modes
- **Server**: Handles game logic, character management, and gameplay commands
- **Communication**: Unified `SendInput` RPC for all commands, with server-side routing

### Layered State Machines

Both gateway and server maintain independent state machines that work together:
- **Gateway states** control how input is collected and formatted
- **Server states** control how commands are interpreted and executed

---

## Gateway-Side State Machine

**Location:** `gateway/src/session.rs`, `gateway/src/server/telnet/state_handler.rs`

The gateway manages connection-level states and input modes, providing protocol-independent state management.

### Top-Level Gateway States

```
┌─────────────────┐
│ Unauthenticated │ ──────────────┐
└─────────────────┘                │
                                   │ Authentication
┌─────────────────┐                │ Success
│  Authenticated  │ ◄──────────────┘
└─────────────────┘
         │
         │ Disconnect
         ▼
┌─────────────────┐
│  Disconnected   │
└─────────────────┘
```

### Authenticated Substates (Input Modes)

Once authenticated, the gateway operates in one of two input modes:

```
Authenticated
  │
  ├──► Playing (default)
  │      │
  │      │ BeginEditing RPC from server
  │      ▼
  └──► Editing
         │
         │ FinishEditing (ctrl+enter or ctrl+escape)
         ▼
       Playing
```

**Input Modes:**
1. **Playing** - Normal gameplay mode
   - Line-buffered input
   - Trim whitespace before sending
   - Send via `SendInput` RPC to server
   
2. **Editing** - Builder/admin editing mode
   - Keystroke-buffered input
   - Maintain local buffer of content being edited
   - **Ctrl+Enter** - Save and send buffer via `FinishEditing` RPC
   - **Ctrl+Escape** - Cancel and discard buffer via `FinishEditing` RPC with empty content

---

## Server-Side State Machine

**Location:** `server/src/listener.rs`
**Main Entry Point:** `send_input()` method

The server manages game-level states and command routing, independent of connection protocols.

## State Machine Flow

```
┌─────────────────┐
│ Gateway Client  │
└────────┬────────┘
         │ gRPC: SendCommand
         ▼
┌─────────────────────────────────────────┐
│  send_command() - Main State Router     │
│  (server/src/listener.rs:427-481)       │
└────────┬────────────────────────────────┘
         │
         │ 1. Authenticate gateway
         │ 2. Get session state
         │ 3. Route to handler
         │
         ▼
    ┌────────────────────────┐
    │  Session State Check   │
    └────────┬───────────────┘
             │
    ┌────────┴────────┬──────────────┬─────────────┐
    │                 │              │             │
    ▼                 ▼              ▼             ▼
┌─────────┐   ┌──────────────┐  ┌─────────┐  ┌─────────┐
│Unauthen-│   │Authenticated │  │Character│  │ Playing │
│ticated  │   │              │  │Creation │  │         │
└─────────┘   └──────┬───────┘  └────┬────┘  └────┬────┘
                     │                │            │
                     ▼                ▼            ▼
              handle_authenticated  handle_char  handle_playing
                   _command()      _creation()    _command()
```

## Session States

Defined in `server/src/session.rs`:

```rust
pub enum ServerSessionState {
    Unauthenticated,      // Not logged in
    Authenticated,        // Logged in, character selection
    CharacterCreation,    // Creating a new character
    Playing,              // Actively playing with a character
    Editing,              // Builder/admin text editing mode (✅ IMPLEMENTED)
}
```

**Note:** The gateway-side `SessionState` includes an `Editing` substate:

```rust
pub enum AuthenticatedState {
    Playing,              // Normal gameplay - line-buffered input
    Editing {             // Text editor - keystroke-buffered input
        title: String,    // What is being edited
        content: String,  // Current content buffer
    },
}
```

## Main State Router

### `send_command()` - Entry Point

**File:** `server/src/listener.rs` (lines 427-481)

This is the main gRPC endpoint that receives all commands from the gateway.

**Flow:**
1. **Authentication Check**: Verifies gateway is authenticated
2. **Session Lookup**: Retrieves session state from in-memory storage
3. **State-Based Routing**: Dispatches to appropriate handler based on state

**Code:**
```rust
async fn send_command(
    &self,
    request: Request<SendCommandRequest>,
) -> Result<Response<SendCommandResponse>, Status> {
    // 1. Verify gateway authentication
    if !self.is_authenticated().await {
        return Err(Status::unauthenticated("Gateway not authenticated"));
    }

    let req = request.into_inner();
    
    // 2. Get session state
    let sessions = self.sessions.read().await;
    let session = sessions.get(&req.session_id)
        .ok_or_else(|| Status::not_found("Session not found"))?;
    let current_state = session.state.clone();
    drop(sessions);

    // 3. Route based on state
    match current_state {
        ServerSessionState::Authenticated => 
            self.handle_authenticated_command(req.session_id, req.command).await,
        ServerSessionState::CharacterCreation => 
            self.handle_character_creation_command(req.session_id, req.command).await,
        ServerSessionState::Playing => 
            self.handle_playing_command(req.session_id, req.command).await,
        _ => Ok(Response::new(SendCommandResponse {
            success: false,
            output: vec![],
            error: Some(format!("Commands not implemented for state: {:?}", current_state)),
        }))
    }
}
```

## State Handlers

### 1. `handle_authenticated_command()` - Character Selection

**File:** `server/src/listener.rs` (lines 566-675)

**Purpose:** Handle commands when user is logged in but hasn't selected/created a character

**Supported Commands:**
- `create` - Start character creation
- `select <name>` - Select existing character (TODO)
- `list` / `characters` - List available characters (TODO)

**State Transitions:**
- `create` → `CharacterCreation` state

**Example:**
```rust
async fn handle_authenticated_command(
    &self,
    session_id: String,
    command: String,
) -> Result<Response<SendCommandResponse>, Status> {
    if command == "create" {
        // Initialize character builder
        let builder = ServerCharacterBuilder::new(
            char_name,
            max_attr_talent_points,
            max_skill_points
        );
        
        // Store builder
        self.character_builders.write().await.insert(session_id.clone(), builder);
        
        // Transition to CharacterCreation state
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.transition(ServerSessionState::CharacterCreation);
        }
        
        // Return character creation UI
        Ok(Response::new(SendCommandResponse { ... }))
    }
    // ... other commands
}
```

### 2. `handle_character_creation_command()` - Character Builder

**File:** `server/src/listener.rs` (lines 736-900)

**Purpose:** Handle character creation commands

**Supported Commands:**
- `attr +/-<name>` - Modify attributes (e.g., `attr +BodyOffence`)
- `talent +/-<name>` - Add/remove talents (e.g., `talent +WeaponMaster`)
- `skill +/-<name>` - Modify skills (e.g., `skill +Swordsmanship`)
- `sheet` / `show` / `status` - View character sheet
- `finalize` / `done` / `create finalize` - Complete character creation
- `help` - Show help text

**State Transitions:**
- `finalize` → `Playing` state (on success)

**Key Features:**
- Validates all modifications against game rules
- Tracks point pools (attributes/talents vs skills)
- Provides detailed error messages
- Shows character sheet with current stats

**Example:**
```rust
async fn handle_character_creation_command(
    &self,
    session_id: String,
    command: String,
) -> Result<Response<SendCommandResponse>, Status> {
    let mut builders = self.character_builders.write().await;
    let builder = builders.get_mut(&session_id)
        .ok_or_else(|| Status::not_found("Character builder not found"))?;

    if command.starts_with("attr ") {
        let arg = command.strip_prefix("attr ").unwrap().trim();
        self.parse_attr_command(builder, arg)
    } else if command == "finalize" {
        // Validate character
        match builder.validate() {
            Ok(()) => {
                // Create character entity in world
                let entity_id = self.finalize_character_creation(...).await?;
                
                // Transition to Playing state
                let mut sessions = self.sessions.write().await;
                if let Some(session) = sessions.get_mut(&session_id) {
                    session.entity_id = Some(entity_id.uuid().into());
                    session.transition(ServerSessionState::Playing);
                }
                
                // Return success with room description
                Ok(Response::new(SendCommandResponse { ... }))
            }
            Err(errors) => {
                // Return validation errors
                Err(format!("Cannot finalize:\n{}", errors.join("\n")))
            }
        }
    }
    // ... other commands
}
```

### 3. `handle_playing_command()` - Gameplay

**File:** `server/src/listener.rs` (lines 1227-1288)

**Purpose:** Handle gameplay commands when actively playing

**Current Implementation:**
- Routes commands to CommandSystem (when integrated)
- Placeholder for full command processing

**Future Integration:**
```rust
async fn handle_playing_command(
    &self,
    session_id: String,
    command: String,
) -> Result<Response<SendCommandResponse>, Status> {
    // Get entity for this session
    let active_entities = self.active_entities.read().await;
    let entity_id = active_entities.get(&session_id)
        .ok_or_else(|| Status::not_found("No active entity for session"))?;

    // Execute command through CommandSystem
    let command_system = self.world_context.command_system().read().await;
    let result = command_system.execute(
        self.world_context.clone(),
        *entity_id,
        command,
        vec![]
    ).await;

    // Convert result to response
    match result {
        CommandResult::Success(msg) => Ok(Response::new(SendCommandResponse {
            success: true,
            output: vec![GameOutput { ... }],
            error: None,
        })),
        CommandResult::Failure(msg) => Ok(Response::new(SendCommandResponse {
            success: false,
            output: vec![GameOutput { ... }],
            error: Some(msg),
        })),
    }
}
```

## Session Storage

### In-Memory Storage

**Location:** `ServerRpcHandler` struct (lines 37-59)

```rust
pub struct ServerRpcHandler {
    /// Session state storage
    sessions: Arc<RwLock<HashMap<SessionId, ServerSession>>>,

    /// Active entity mapping (SessionId -> EntityId)
    active_entities: Arc<RwLock<HashMap<SessionId, EntityId>>>,

    /// Character builders for sessions in character creation
    character_builders: Arc<RwLock<HashMap<SessionId, ServerCharacterBuilder>>>,

    /// World engine context
    world_context: Arc<WorldContext>,
    
    // ... other fields
}
```

### Session Data Structure

**File:** `server/src/session.rs`

```rust
pub struct ServerSession {
    /// Current state of the session
    pub state: ServerSessionState,

    /// Whether the session is authenticated
    pub authenticated: bool,

    /// The account ID (if authenticated)
    pub account_id: Option<uuid::Uuid>,

    /// The authenticated entity ID (if any)
    pub entity_id: Option<PersistentEntityId>,

    /// Queued events during disconnection
    pub queued_events: Vec<GameOutput>,
}
```

## State Transitions

### Valid Transitions

```
Unauthenticated → Authenticated (on login)
Authenticated → CharacterCreation (on "create" command)
Authenticated → Playing (on "select" command - future)
CharacterCreation → Playing (on "finalize" command)
Playing → Authenticated (on character logout - future)
Any State → Disconnected (on disconnect)
Disconnected → Previous State (on reconnect)
```

### Transition Implementation

State transitions are handled by updating the session state:

```rust
let mut sessions = self.sessions.write().await;
if let Some(session) = sessions.get_mut(&session_id) {
    session.transition(ServerSessionState::Playing);
}
```

## Session Lifecycle

### 1. Connection & Authentication

```
Gateway connects → authenticate_gateway()
User logs in → authenticate_session()
Session created with Authenticated state
```

### 2. Character Selection/Creation

```
User types "create" → handle_authenticated_command()
Transition to CharacterCreation state
Character builder initialized
User modifies character → handle_character_creation_command()
User types "finalize" → Character created, transition to Playing
```

### 3. Gameplay

```
User in Playing state
Commands routed to handle_playing_command()
Commands executed through CommandSystem
Results sent back to gateway
```

### 4. Disconnection & Reconnection

```
Client disconnects → session_disconnected()
Session marked as Disconnected
Events queued during disconnection
Client reconnects → session_reconnected()
Session state restored
Queued events delivered
```

## Error Handling

### Gateway Authentication
- All commands require authenticated gateway
- Returns `Status::unauthenticated` if not authenticated

### Session Validation
- Session must exist in storage
- Returns `Status::not_found` if session not found

### State-Specific Errors
- Each handler validates commands for its state
- Returns appropriate error messages
- Provides helpful usage examples

## Integration Points

### Gateway Integration
- Gateway calls `send_command()` via gRPC
- Gateway manages client connections
- Gateway handles protocol-specific details (Telnet, WebSocket)

### WorldContext Integration
- Commands access world through `world_context`
- Entity operations use WorldContext methods
- CommandSystem accessed through WorldContext

### Persistence Integration
- Character creation saves to database
- Character loading from database
- Session state is in-memory only

## Future Enhancements

1. **Character Selection**: Implement `select <name>` command
2. **Character List**: Load and display user's characters
3. **Character Deletion**: Add character deletion support
4. ~~**Editing Mode**: Implement builder/admin state~~ ✅ **COMPLETED** - See [Editor Implementation](EDITOR_IMPLEMENTATION.md)
5. **Command History**: Track command history per session
6. **Session Timeout**: Automatic cleanup of inactive sessions
7. **Rate Limiting**: Prevent command spam
8. **Command Queuing**: Queue commands during high load

## Related Files

- `server/src/listener.rs` - Main state engine
- `server/src/session.rs` - Session data structures
- `server/src/ecs/character_builder.rs` - Character creation logic
- `server/src/ecs/systems/command.rs` - Command system
- `gateway/src/session.rs` - Gateway-side session management

## Summary

The session state engine provides:
- ✅ Clean state-based command routing
- ✅ Separation of concerns (authentication, creation, gameplay)
- ✅ Type-safe state transitions
- ✅ Comprehensive error handling
- ✅ Session reconnection support
- ✅ Integration with WorldContext and CommandSystem


## Key Benefits of Layered Architecture

1. **Protocol Independence**: Gateway states work for Telnet, WebSocket, or any future protocol
2. **Simplified Server**: Server doesn't need to know about authentication flows or input modes
3. **Easier Testing**: Each layer can be tested independently
4. **Clear Separation**: Connection concerns vs game logic concerns
5. **Flexibility**: Easy to add new states without changing protocol

## See Also

- [GATEWAY_WORLD_REFACTOR.md](../../GATEWAY_WORLD_REFACTOR.md) - Complete refactor implementation details
- [EDITOR_IMPLEMENTATION.md](EDITOR_IMPLEMENTATION.md) - Editing mode implementation
- [GATEWAY_PROTOCOL.md](GATEWAY_PROTOCOL.md) - RPC protocol reference
- [PROJECT_STATUS.md](PROJECT_STATUS.md) - Overall project status
This architecture makes it easy to add new states and commands while maintaining clear separation between different phases of the user experience.
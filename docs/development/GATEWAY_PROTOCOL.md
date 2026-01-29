# Gateway-Server Communication Protocol

## Overview

The Wyldlands gateway-server protocol provides **bidirectional RPC communication** between the gateway (connection handler) and the world server (game logic) using [gRPC](https://grpc.io/) with [tonic](https://github.com/hyperium/tonic).

## Architecture

```
┌─────────────────┐                    ┌─────────────────┐
│                 │                    │                 │
│     Gateway     │◄──────────────────►│  World Server   │
│                 │                    │                 │
│  - Telnet       │   Bidirectional    │  - Game Logic   │
│  - WebSocket    │   RPC (gRPC)       │  - ECS Systems  │
│  - Sessions     │                    │  - Persistence  │
│                 │                    │                 │
└─────────────────┘                    └─────────────────┘
        │                                      │
        │ GatewayServer trait                  │ ServerGateway trait
        │ (Gateway → Server)                   │ (Server → Gateway)
        │                                      │
        ▼                                      ▼
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  • authenticate()          • send_output()             │
│  • create_character()      • send_prompt()             │
│  • select_character()      • entity_state_changed()    │
│  • send_command()          • disconnect_session()      │
│  • session_disconnected()                              │
│  • session_reconnected()                               │
│  • list_characters()                                   │
│  • heartbeat()                                         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Bidirectional Communication

### Gateway → Server (GatewayServer trait)

The gateway calls these methods on the world server:

#### Authentication & Character Management
- **`authenticate(session_id, username, password)`**
  - Authenticates user credentials
  - Returns entity ID on success
  - Used during login flow

- **`create_character(session_id, name, data)`**
  - Creates a new character
  - Returns new entity ID
  - Used during character creation

- **`select_character(session_id, entity_id)`**
  - Selects a character for play
  - Returns character info and initial state
  - Used after character selection

- **`list_characters(session_id)`**
  - Lists available characters for authenticated session
  - Returns character summaries
  - Used in character selection screen

#### Game Commands
- **`send_command(session_id, command)`**
  - Sends player command to server
  - Returns command result with output
  - Main gameplay interaction

#### Session Lifecycle
- **`session_disconnected(session_id, reason)`**
  - Notifies server of disconnection
  - Server can queue events for reconnection
  - No return value (fire-and-forget)

- **`session_reconnected(session_id, entity_id)`**
  - Notifies server of reconnection
  - Returns queued events and current state
  - Enables seamless reconnection

- **`heartbeat(session_id)`**
  - Keeps session alive
  - Prevents timeout
  - Called periodically by gateway

### Server → Gateway (ServerGateway trait)

The world server calls these methods on the gateway:

#### Output Delivery
- **`send_output(session_id, output)`**
  - Sends game output to client
  - Supports multiple output types (text, formatted, structured, room, combat)
  - Gateway routes to appropriate client connection

- **`send_prompt(session_id, prompt)`**
  - Sends command prompt to client
  - Indicates server is ready for input
  - Gateway displays prompt to user

#### State Updates
- **`entity_state_changed(session_id, state_update)`**
  - Notifies gateway of entity state changes
  - Includes stats, position, inventory, equipment, status effects
  - Gateway can update client UI

#### Session Control
- **`disconnect_session(session_id, reason)`**
  - Requests gateway to disconnect a session
  - Used for kicks, bans, or server shutdown
  - Gateway performs graceful disconnect

## Message Flow Examples

### Example 1: Login Flow

```
Client → Gateway: "login username password"
Gateway → Server: authenticate(session_id, "username", "password_hash")
Server → Gateway: Ok(AuthResult { success: true, entity_id: Some("...") })
Server → Gateway: send_output(session_id, Text("Welcome back!"))
Gateway → Client: "Welcome back!"
```

### Example 2: Command Execution

```
Client → Gateway: "look"
Gateway → Server: send_command(session_id, "look")
Server → Gateway: Ok(CommandResult { 
    success: true, 
    output: [RoomDescription { ... }] 
})
Server → Gateway: send_prompt(session_id, "> ")
Gateway → Client: [Room description]
Gateway → Client: "> "
```

### Example 3: Combat Event

```
[Server detects combat action]
Server → Gateway: send_output(session_id, Combat(CombatMessage {
    attacker: "Goblin",
    defender: "You",
    action: "slashes",
    damage: Some(5),
    critical: false
}))
Server → Gateway: entity_state_changed(session_id, EntityStateUpdate {
    update_type: Stats,
    data: { "hp": 45 }
})
Gateway → Client: "The Goblin slashes you for 5 damage!"
Gateway → Client: [Update HP display]
```

### Example 4: Reconnection

```
[Client disconnects]
Gateway → Server: session_disconnected(session_id, NetworkError)
[Server queues events]
[Client reconnects with token]
Gateway → Server: session_reconnected(session_id, entity_id)
Server → Gateway: Ok(ReconnectResult {
    success: true,
    queued_events: [Text("A goblin attacked!"), ...],
    character_state: Some(CharacterInfo { ... })
})
Gateway → Client: [Replay queued events]
```

## Data Types

### Core Types

- **SessionId**: String (UUID)
- **EntityId**: String (UUID)

### Authentication

```rust
struct AuthResult {
    success: bool,
    entity_id: Option<EntityId>,
    message: String,
}

enum AuthError {
    InvalidCredentials,
    AccountLocked,
    SessionNotFound,
    AlreadyAuthenticated,
    ServerError(String),
}
```

### Character Management

```rust
struct CharacterCreationData {
    race: String,
    class: String,
    attributes: HashMap<String, i32>,
    description: String,
    metadata: HashMap<String, String>,
}

struct CharacterInfo {
    entity_id: EntityId,
    name: String,
    level: u32,
    race: String,
    class: String,
    location: String,
    attributes: HashMap<String, i32>,
    stats: HashMap<String, i32>,
}

struct CharacterSummary {
    entity_id: EntityId,
    name: String,
    level: u32,
    race: String,
    class: String,
    last_played: String,
}
```

### Game Output

```rust
enum GameOutput {
    Text(String),                    // Plain text
    FormattedText(String),           // ANSI formatted
    Structured(StructuredOutput),    // For GUI clients
    RoomDescription(RoomDescription), // Room info
    Combat(CombatMessage),           // Combat events
    System(String),                  // System messages
}

struct RoomDescription {
    name: String,
    description: String,
    exits: Vec<String>,
    entities: Vec<String>,
    items: Vec<String>,
}

struct CombatMessage {
    attacker: String,
    defender: String,
    action: String,
    damage: Option<i32>,
    critical: bool,
}
```

### Session Management

```rust
enum DisconnectReason {
    ClientDisconnect,
    Timeout,
    NetworkError,
    ServerShutdown,
    Kicked(String),
}

struct ReconnectResult {
    success: bool,
    queued_events: Vec<GameOutput>,
    character_state: Option<CharacterInfo>,
}
```

### Entity State

```rust
struct EntityStateUpdate {
    entity_id: EntityId,
    update_type: StateUpdateType,
    data: HashMap<String, serde_json::Value>,
}

enum StateUpdateType {
    Stats,           // HP, MP, etc.
    Position,        // Location changes
    Inventory,       // Item changes
    Equipment,       // Equipment changes
    StatusEffects,   // Buffs/debuffs
    Custom(String),  // Custom updates
}
```

## Implementation Guide

### Gateway Side

```rust
use wyldlands_protocol::gateway::{GatewayServer, ServerGateway};
use tonic::{transport::Channel, Request, Response, Status};

// Implement ServerGateway to receive calls from server
#[derive(Clone)]
struct GatewayHandler {
    connection_pool: Arc<ConnectionPool>,
}

#[tonic::async_trait]
impl ServerGateway for GatewayHandler {
    async fn send_output(self, _: context::Context, session_id: SessionId, output: GameOutput) {
        // Route output to client connection
        self.connection_pool.send(session_id, output).await;
    }
    
    async fn send_prompt(self, _: context::Context, session_id: SessionId, prompt: String) {
        // Send prompt to client
        self.connection_pool.send(session_id, prompt).await;
    }
    
    // ... implement other methods
}

// Create client to call server
async fn connect_to_server() -> GatewayServerClient<Channel> {
    let channel = Channel::from_static("http://127.0.0.1:5000").connect().await?;
    GatewayServerClient::new(channel)
}
```

### Server Side

```rust
use wyldlands_protocol::gateway::{GatewayServer, ServerGateway};
use tonic::{transport::Channel, Request, Response, Status};

// Implement GatewayServer to receive calls from gateway
#[derive(Clone)]
struct WorldServer {
    ecs: Arc<EcsWorld>,
}

#[tonic::async_trait]
impl GatewayServer for WorldServer {
    async fn authenticate(
        self,
        _: context::Context,
        session_id: SessionId,
        username: String,
        password: String,
    ) -> Result<AuthResult, AuthError> {
        // Authenticate user
        // Return entity ID on success
    }
    
    async fn send_command(
        self,
        _: context::Context,
        session_id: SessionId,
        command: String,
    ) -> Result<CommandResult, CommandError> {
        // Process command in ECS
        // Return results
    }
    
    // ... implement other methods
}

// Create client to call gateway
async fn connect_to_gateway() -> ServerGatewayClient<Channel> {
    let channel = Channel::from_static("http://127.0.0.1:5001").connect().await?;
    ServerGatewayClient::new(channel)
}
```

## Connection Setup

### Gateway Startup

1. Start ServerGateway RPC server (listens for server calls)
2. Connect to WorldServer as GatewayServer client
3. Handle client connections (Telnet/WebSocket)
4. Route messages between clients and server

### Server Startup

1. Start GatewayServer RPC server (listens for gateway calls)
2. Connect to Gateway as ServerGateway client
3. Process game logic in ECS
4. Send updates to gateway

## Error Handling

All RPC methods return `Result` types with specific error enums:

- **AuthError**: Authentication failures
- **CharacterError**: Character management errors
- **CommandError**: Command execution errors
- **ReconnectError**: Reconnection failures
- **SessionError**: Session management errors

Errors are serialized and transmitted over RPC, allowing proper error handling on both sides.

## Performance Considerations

### Batching
- Group multiple `send_output` calls when possible
- Use `Structured` output for GUI clients to reduce bandwidth

### Caching
- Gateway caches character info to reduce RPC calls
- Server caches session state to avoid database queries

### Async/Await
- All methods are async for non-blocking I/O
- gRPC/tonic handles connection pooling and multiplexing
- Protocol Buffers provide efficient serialization

### Heartbeats
- Gateway sends periodic heartbeats (30s default)
- Server can detect dead sessions and clean up

## Security Considerations

### Authentication
- Passwords should be hashed before transmission
- Use TLS for production deployments
- Implement rate limiting on authentication attempts

### Session Validation
- All RPC calls include session_id for validation
- Server verifies session ownership before processing
- Expired sessions are rejected

### Input Validation
- Server validates all command input
- Sanitize user-provided strings
- Enforce length limits on all text fields

## Future Enhancements

1. **Compression**: Add message compression for large outputs
2. **Encryption**: TLS support for secure communication
3. **Load Balancing**: Support multiple gateway instances
4. **Metrics**: Add RPC call metrics and monitoring
5. **Versioning**: Protocol version negotiation
6. **Streaming**: Support streaming for large data transfers

## Testing

### Unit Tests
- Test serialization/deserialization of all types
- Verify error handling
- Test edge cases

### Integration Tests
- Test full RPC flow with mock server/gateway
- Test reconnection scenarios
- Test concurrent operations

### Load Tests
- Test with 1000+ concurrent sessions
- Measure RPC latency
- Test failure scenarios

---

**Status**: Implemented  
**Last Updated**: 2025-12-18  
**Author**: Bob (AI Assistant)
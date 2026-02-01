---
parent: ADR
nav_order: 0012
title: Session State Management Strategy
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0012: Session State Management Strategy

## Context and Problem Statement

A MUD server must manage complex session lifecycles including authentication, character selection, character creation, gameplay, and disconnection/reconnection. The system must:
- Handle multiple protocols (Telnet, WebSocket, future protocols)
- Separate connection-level concerns from game-level concerns
- Support disconnection and reconnection with state recovery
- Enable different input modes (playing vs editing)
- Maintain session state across server restarts
- Provide clear state transitions and validation

How should we design the session state management to be protocol-independent, maintainable, and extensible?

## Decision Drivers

* **Protocol Independence**: Support multiple protocols without duplicating state logic
* **Separation of Concerns**: Gateway handles connections, server handles game logic
* **State Persistence**: Sessions survive disconnections and server restarts
* **Clear Boundaries**: Well-defined responsibilities for each layer
* **Extensibility**: Easy to add new states and protocols
* **Testability**: Each layer can be tested independently
* **User Experience**: Smooth transitions and clear feedback

## Considered Options

* Layered State Machine Architecture (Gateway + Server)
* Monolithic State Machine (Single Layer)
* Event-Driven State Management
* Actor-Based State Management

## Decision Outcome

Chosen option: "Layered State Machine Architecture", because it provides the best separation of concerns, protocol independence, and maintainability while enabling independent testing of each layer.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Gateway Layer                         │
│  Protocol-Independent Connection Management              │
│                                                          │
│  States: Unauthenticated → Authenticated → Disconnected │
│  Modes:  Playing | Editing                              │
└─────────────────┬───────────────────────────────────────┘
                  │ gRPC: SendInput
                  ▼
┌─────────────────────────────────────────────────────────┐
│                    Server Layer                          │
│  Game Logic and Command Routing                         │
│                                                          │
│  States: Authentication → CharacterSelection →          │
│          CharacterCreation → Playing → Editing          │
└─────────────────────────────────────────────────────────┘
```

### Gateway-Side State Machine

**Location:** `gateway/src/session.rs`, `gateway/src/server/telnet/state_handler.rs`

**Top-Level States:**
1. **Unauthenticated**: Initial connection state
   - Collect username and password
   - Send authentication RPC to server
   - Transition to Authenticated on success

2. **Authenticated**: Logged in, ready for gameplay
   - Two input modes: Playing or Editing
   - Handle input based on current mode
   - Send commands via SendInput RPC

3. **Disconnected**: Connection lost, awaiting reconnection
   - Preserve session state in database
   - Generate reconnection token
   - Allow reconnection within TTL window

**Input Modes (Authenticated Substates):**
- **Playing**: Line-buffered input, normal gameplay
- **Editing**: Keystroke-buffered input, text editing (Ctrl+Enter to save, Ctrl+Escape to cancel)

### Server-Side State Machine

**Location:** `server/src/listener.rs`

**Game States:**
1. **Authentication**: Validating credentials
   - Verify username/password
   - Create or load account
   - Transition to CharacterSelection

2. **CharacterSelection**: Choosing or creating character
   - List available characters
   - Handle character selection
   - Handle character creation request
   - Transition to CharacterCreation or Playing

3. **CharacterCreation**: Building new character
   - Validate attribute/talent/skill allocations
   - Enforce point pool limits
   - Persist character on completion
   - Transition to Playing

4. **Playing**: Active gameplay
   - Route commands to appropriate systems
   - Process game events
   - Handle state changes (entering editing mode)

5. **Editing**: Builder/admin text editing
   - Buffer text input
   - Save or cancel on completion
   - Transition back to Playing

### Positive Consequences

* **Protocol Independence**: Gateway states work for any protocol
* **Clean Separation**: Connection logic separate from game logic
* **Independent Testing**: Each layer tested separately
* **Simplified Server**: Server doesn't handle protocol details
* **Extensibility**: Easy to add new protocols or states
* **State Recovery**: Sessions persist across disconnections
* **Clear Responsibilities**: Each layer has well-defined role

### Negative Consequences

* **Complexity**: Two state machines to maintain
* **Coordination**: Must keep gateway and server states synchronized
* **Debugging**: State issues may span multiple layers

## Pros and Cons of the Options

### Layered State Machine Architecture

* Good, because protocol-independent design
* Good, because clean separation of concerns
* Good, because independently testable
* Good, because easy to add new protocols
* Good, because server doesn't handle protocol details
* Neutral, because requires coordination between layers
* Bad, because more complex than single layer
* Bad, because state spans multiple components

### Monolithic State Machine

```
Single state machine handling both connection and game logic
```

* Good, because simpler architecture
* Good, because single source of truth
* Good, because easier to debug
* Neutral, because all logic in one place
* Bad, because protocol-specific code in game logic
* Bad, because harder to add new protocols
* Bad, because tight coupling
* Bad, because harder to test independently

### Event-Driven State Management

```
States managed through event bus with handlers
```

* Good, because decoupled components
* Good, because flexible event routing
* Neutral, because requires event infrastructure
* Bad, because harder to reason about state flow
* Bad, because implicit state transitions
* Bad, because harder to debug
* Bad, because potential race conditions

### Actor-Based State Management

```
Each session is an actor with message-based state
```

* Good, because natural concurrency model
* Good, because isolated state per session
* Neutral, because requires actor framework
* Bad, because more complex than state machines
* Bad, because harder to persist state
* Bad, because overkill for this use case

## Implementation Details

### Gateway State Transitions

```rust
// gateway/src/session.rs
pub enum SessionState {
    Unauthenticated,
    Authenticated {
        account_id: Uuid,
        input_mode: InputMode,
    },
    Disconnected {
        reconnect_token: String,
        expires_at: DateTime<Utc>,
    },
}

pub enum InputMode {
    Playing,
    Editing {
        buffer: String,
        prompt: String,
    },
}
```

**State Transitions:**
- `Unauthenticated` → `Authenticated`: After successful authentication RPC
- `Authenticated` → `Disconnected`: On connection loss
- `Disconnected` → `Authenticated`: On successful reconnection
- `Playing` ↔ `Editing`: Via BeginEditing/FinishEditing RPCs

### Server State Transitions

```rust
// server/src/listener.rs
pub enum ServerSessionState {
    Authentication,
    CharacterSelection { account_id: Uuid },
    CharacterCreation { account_id: Uuid, builder: CharacterBuilder },
    Playing { character_id: Uuid },
    Editing { character_id: Uuid, context: EditContext },
}
```

**State Transitions:**
- `Authentication` → `CharacterSelection`: After account validation
- `CharacterSelection` → `CharacterCreation`: On create character request
- `CharacterSelection` → `Playing`: On character selection
- `CharacterCreation` → `Playing`: On character finalization
- `Playing` → `Editing`: On edit command
- `Editing` → `Playing`: On save/cancel

### Unified SendInput RPC

All commands flow through a single RPC:

```protobuf
service GatewayServer {
    rpc SendInput(SendInputRequest) returns (SendInputResponse);
}

message SendInputRequest {
    string session_id = 1;
    string input = 2;
}
```

Server routes input based on current state:
- `Authentication`: Handle login commands
- `CharacterSelection`: Handle character selection/creation
- `CharacterCreation`: Handle attribute/talent/skill allocation
- `Playing`: Route to command system
- `Editing`: Buffer text or save/cancel

### State Persistence

**Database Schema:**
```sql
CREATE TYPE wyldlands.session_state AS ENUM (
    'Connecting',
    'Authenticating',
    'Authenticated',
    'CharacterSelection',
    'CharacterCreation',
    'Playing',
    'Editing',
    'Disconnected',
    'Closed'
);

CREATE TABLE wyldlands.sessions (
    session_id UUID PRIMARY KEY,
    account_id UUID,
    character_id UUID,
    state wyldlands.session_state NOT NULL,
    state_data JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ
);
```

**State Data Examples:**
- `CharacterCreation`: Stores CharacterBuilder as JSON
- `Editing`: Stores edit context and buffer
- `Disconnected`: Stores reconnection token

## Validation

The session state management is validated by:

1. **Unit Tests**: State transition logic (70+ gateway tests, 145+ server tests)
2. **Integration Tests**: Full authentication and character creation flows
3. **State Machine Tests**: Invalid transition prevention
4. **Persistence Tests**: State recovery after disconnection
5. **Protocol Tests**: WebSocket and Telnet state handling

## More Information

### State Synchronization

Gateway and server states are synchronized via:
1. **RPC Responses**: Server returns state changes in responses
2. **Server-Initiated RPCs**: Server can trigger gateway state changes (BeginEditing)
3. **Database**: Shared session state in PostgreSQL

### Reconnection Flow

```
1. Client disconnects
   ↓
2. Gateway: Transition to Disconnected, generate token
   ↓
3. Database: Persist session state and token
   ↓
4. Client reconnects with token
   ↓
5. Gateway: Validate token, restore session
   ↓
6. Server: Replay queued commands
   ↓
7. Gateway: Transition to Authenticated
```

### Future Enhancements

1. **State Snapshots**: Periodic state snapshots for faster recovery
2. **State Migrations**: Versioned state data for schema evolution
3. **State Analytics**: Track state transition patterns
4. **State Debugging**: Tools for inspecting session state
5. **Multi-Server Sessions**: Session state shared across multiple servers

### Related Decisions

- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Separation enables layered states
- [ADR-0006](ADR-0006-Layered-State-Machine-Architecture.md) - Original layered architecture decision
- [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md) - RPC enables state coordination
- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - Database stores session state
- [ADR-0011](ADR-0011-Character-Creation-System-Architecture.md) - Character creation is a session state
- [ADR-0018](ADR-0018-Input-Mode-Architecture.md) - Input modes are gateway substates

### References

- Session State Engine: [docs/development/SESSION_STATE_ENGINE.md](../development/SESSION_STATE_ENGINE.md)
- Gateway Session: [gateway/src/session.rs](../../gateway/src/session.rs)
- Gateway State Handler: [gateway/src/server/telnet/state_handler.rs](../../gateway/src/server/telnet/handler.rs)
- Server Listener: [server/src/listener.rs](../../server/src/listener.rs)
- Session Tests: [gateway/tests/session_tests.rs](../../gateway/tests/session_tests.rs)
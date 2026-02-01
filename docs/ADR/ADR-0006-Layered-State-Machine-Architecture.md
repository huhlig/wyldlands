---
parent: ADR
nav_order: 0006
title: Layered State Machine Architecture
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0006: Layered State Machine Architecture

## Context and Problem Statement

With a separated gateway and server architecture, we need to design how session state is managed across both components. The system must handle:
- Connection-level state (authentication, input modes)
- Game-level state (character selection, character creation, playing)
- Protocol-independent state management
- State synchronization between gateway and server
- Clear separation of concerns

How should we structure state management across the gateway and server components?

## Decision Drivers

* **Separation of Concerns**: Connection state vs game state should be clearly separated
* **Protocol Independence**: Gateway states should work for any protocol (Telnet, WebSocket, etc.)
* **Simplicity**: Server should not handle protocol-specific concerns
* **Flexibility**: Easy to add new states without changing protocols
* **Testability**: Each layer can be tested independently
* **Maintainability**: Clear state transitions and responsibilities

## Considered Options

* Layered State Machines (Gateway + Server)
* Single Unified State Machine
* Event-Driven State Management
* Stateless Server with Gateway-Only State

## Decision Outcome

Chosen option: "Layered State Machines", because it provides the best separation of concerns while maintaining clear state management at each layer. The gateway manages connection-level states and input modes, while the server manages game-level states.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Gateway State Machine                     │
│                                                              │
│  ┌─────────────────┐                                        │
│  │ Unauthenticated │ ──────────────┐                        │
│  └─────────────────┘                │                        │
│                                     │ Auth Success           │
│  ┌─────────────────┐                │                        │
│  │  Authenticated  │ ◄──────────────┘                        │
│  └────────┬────────┘                                         │
│           │                                                  │
│           ├──► Playing (line-buffered input)                │
│           │                                                  │
│           └──► Editing (keystroke-buffered input)           │
│                                                              │
└──────────────────────────┬───────────────────────────────────┘
                           │ SendInput RPC
                           │
┌──────────────────────────▼───────────────────────────────────┐
│                    Server State Machine                      │
│                                                              │
│  ┌─────────────────┐                                        │
│  │ Unauthenticated │                                        │
│  └────────┬────────┘                                         │
│           │ authenticate_session()                           │
│           ▼                                                  │
│  ┌─────────────────┐                                        │
│  │  Authenticated  │ (character selection)                  │
│  └────────┬────────┘                                         │
│           │                                                  │
│           ├──► CharacterCreation (on "create")              │
│           │                                                  │
│           └──► Playing (on "select" or finalize)            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Positive Consequences

* **Clear Separation**: Connection concerns (gateway) vs game logic (server)
* **Protocol Independence**: Gateway states work for any protocol
* **Simplified Server**: Server doesn't handle authentication flows or input modes
* **Independent Testing**: Each state machine can be tested separately
* **Easy Extension**: New states can be added to appropriate layer
* **Unified Input**: Single `SendInput` RPC for all commands
* **Flexible Input Modes**: Gateway handles different input modes (playing vs editing)

### Negative Consequences

* **State Synchronization**: Must keep gateway and server states aligned
* **Complexity**: Two state machines to understand and maintain
* **Coordination**: State transitions may require coordination between layers

## Pros and Cons of the Options

### Layered State Machines (Gateway + Server)

* Good, because clear separation of connection and game concerns
* Good, because protocol-independent server implementation
* Good, because each layer handles appropriate responsibilities
* Good, because easy to test each layer independently
* Good, because gateway can handle reconnections without server involvement
* Neutral, because requires state synchronization via RPC
* Bad, because two state machines to maintain
* Bad, because state transitions may span both layers

### Single Unified State Machine

```
Unauthenticated → Authenticated → CharacterSelection → 
CharacterCreation → Playing → Editing
```

* Good, because single source of truth for state
* Good, because simpler mental model
* Neutral, because all state in one place
* Bad, because mixes connection and game concerns
* Bad, because server must handle protocol-specific details
* Bad, because harder to add new protocols
* Bad, because gateway becomes a thin proxy

### Event-Driven State Management

```
Events: Connected, Authenticated, CharacterSelected, 
        CommandReceived, Disconnected, etc.
```

* Good, because flexible and extensible
* Good, because decoupled components
* Neutral, because event-driven architecture
* Bad, because harder to reason about state
* Bad, because potential for race conditions
* Bad, because more complex debugging
* Bad, because overkill for this use case

### Stateless Server with Gateway-Only State

* Good, because server is completely stateless
* Good, because simple server implementation
* Neutral, because all state in gateway
* Bad, because gateway must track game state
* Bad, because harder to implement game logic
* Bad, because poor separation of concerns
* Bad, because gateway becomes too complex

## Implementation Details

### Gateway State Machine

**Location:** `gateway/src/session.rs`, `gateway/src/server/telnet/state_handler.rs`

**Top-Level States:**
```rust
pub enum SessionState {
    Unauthenticated,
    Authenticated(AuthenticatedState),
    Disconnected { previous_state: Box<SessionState> },
}
```

**Authenticated Substates (Input Modes):**
```rust
pub enum AuthenticatedState {
    Playing,              // Normal gameplay - line-buffered input
    Editing {             // Text editor - keystroke-buffered input
        title: String,    // What is being edited
        content: String,  // Current content buffer
    },
}
```

**Responsibilities:**
- Manage connection lifecycle
- Handle authentication flow (username, password, email, etc.)
- Control input buffering mode (line vs keystroke)
- Translate protocol-specific input to generic commands
- Route commands to server via `SendInput` RPC
- Handle reconnections and command queuing

**State Transitions:**
- `Unauthenticated` → `Authenticated(Playing)` on successful login
- `Authenticated(Playing)` → `Authenticated(Editing)` on `BeginEditing` RPC from server
- `Authenticated(Editing)` → `Authenticated(Playing)` on Ctrl+Enter or Ctrl+Escape
- Any state → `Disconnected` on connection loss
- `Disconnected` → Previous state on reconnection

### Server State Machine

**Location:** `server/src/listener.rs`, `server/src/session.rs`

**States:**
```rust
pub enum ServerSessionState {
    Unauthenticated,      // Not logged in
    Authenticated,        // Logged in, character selection
    CharacterCreation,    // Creating a new character
    Playing,              // Actively playing with a character
    Editing,              // Builder/admin text editing mode
}
```

**Responsibilities:**
- Manage game-level state
- Process game commands
- Handle character selection and creation
- Execute game logic (movement, combat, inventory)
- Trigger editing mode when needed
- Persist game state

**State Transitions:**
- `Unauthenticated` → `Authenticated` on `authenticate_session()` RPC
- `Authenticated` → `CharacterCreation` on "create" command
- `Authenticated` → `Playing` on "select" command (future)
- `CharacterCreation` → `Playing` on "finalize" command
- `Playing` → `Editing` when builder enters edit mode
- `Editing` → `Playing` when editing completes

### State Synchronization

**Gateway → Server:**
- `authenticate_session()` - Transition to Authenticated
- `send_input("create")` - Transition to CharacterCreation
- `send_input("finalize")` - Transition to Playing
- `session_disconnected()` - Mark session as disconnected
- `session_reconnected()` - Restore session state

**Server → Gateway:**
- `begin_editing()` - Transition to Editing mode
- `finish_editing()` - Exit editing mode
- `send_output()` - Deliver game output
- `disconnect_session()` - Force disconnect

### Input Mode Handling

**Playing Mode (Line-Buffered):**
- User types command and presses Enter
- Gateway trims whitespace
- Gateway sends complete line via `SendInput` RPC
- Server processes command and returns output

**Editing Mode (Keystroke-Buffered):**
- Server sends `BeginEditing` RPC with title and initial content
- Gateway transitions to Editing mode
- Gateway buffers all keystrokes locally
- User presses Ctrl+Enter to save or Ctrl+Escape to cancel
- Gateway sends `FinishEditing` RPC with final content
- Server processes edited content

### Example Flow: Character Creation

```
1. Client connects
   Gateway: Unauthenticated
   Server: N/A

2. User logs in
   Gateway: authenticate_session() RPC
   Gateway: Unauthenticated → Authenticated(Playing)
   Server: Unauthenticated → Authenticated

3. User types "create"
   Gateway: send_input("create") RPC
   Server: Authenticated → CharacterCreation
   Server: Returns character creation UI

4. User modifies character
   Gateway: send_input("attr +BodyOffence") RPC
   Server: Processes in CharacterCreation state
   Server: Returns updated character sheet

5. User types "finalize"
   Gateway: send_input("finalize") RPC
   Server: CharacterCreation → Playing
   Server: Creates character entity
   Server: Returns room description

6. User plays game
   Gateway: Authenticated(Playing)
   Server: Playing
   Gateway: send_input("look") RPC
   Server: Processes in Playing state
```

### Example Flow: Builder Editing

```
1. User in Playing state
   Gateway: Authenticated(Playing)
   Server: Playing

2. User types "room edit description"
   Gateway: send_input("room edit description") RPC
   Server: Sends begin_editing() RPC
   Gateway: Playing → Editing { title: "Room Description", content: "..." }

3. User edits text
   Gateway: Buffers keystrokes locally
   (No RPC calls during editing)

4. User presses Ctrl+Enter
   Gateway: Sends finish_editing(content) RPC
   Gateway: Editing → Playing
   Server: Saves edited content
   Server: Returns confirmation

5. User continues playing
   Gateway: Authenticated(Playing)
   Server: Playing
```

## Validation

The layered architecture is validated by:

1. **Protocol Independence Achieved:**
   - Server has zero protocol-specific code
   - Same server code works for Telnet and WebSocket
   - Easy to add new protocols to gateway

2. **State Management Success:**
   - 215 tests passing (70 gateway + 145 server)
   - Clear state transitions in both layers
   - No state synchronization bugs reported

3. **Feature Velocity:**
   - Added editing mode without changing server
   - Added character creation without changing gateway
   - Each layer can evolve independently

4. **Operational Success:**
   - Reconnection works transparently
   - State persists across disconnections
   - Clean error handling at each layer

## More Information

### Key Benefits

1. **Protocol Independence**: Gateway states work for any protocol (Telnet, WebSocket, future protocols)
2. **Simplified Server**: Server doesn't need to know about authentication flows or input modes
3. **Easier Testing**: Each layer can be tested independently
4. **Clear Separation**: Connection concerns vs game logic concerns
5. **Flexibility**: Easy to add new states without changing protocol

### State Persistence

**Gateway State:**
- Stored in memory only
- Recreated on reconnection
- Command queue persisted in database during disconnection

**Server State:**
- Stored in memory with database backing
- Session state persisted to database
- Character state persisted via ECS persistence system

### Future Enhancements

1. **Additional Input Modes:**
   - Paging mode for long text
   - Menu mode for structured choices
   - Form mode for data entry

2. **State Transitions:**
   - Character selection screen
   - Character deletion confirmation
   - Admin command mode

3. **Advanced Features:**
   - State history for debugging
   - State transition logging
   - State machine visualization

### Related Decisions

- [ADR-0003](ADR-0003-Use-Rust-Programming-Language.md) - Rust enables type-safe state machines
- [ADR-0004](ADR-0004-Use-Entity-Component-System.md) - ECS manages game state
- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Architectural separation enables layered states
- [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md) - gRPC for state synchronization
- [ADR-0009](ADR-0009-Protocol-Independence-Design.md) - Protocol adapters work with gateway states

### References

- Architecture Documentation: [docs/development/SESSION_STATE_ENGINE.md](../development/SESSION_STATE_ENGINE.md)
- Gateway State Implementation: [gateway/src/session.rs](../../gateway/src/session.rs)
- Server State Implementation: [server/src/listener.rs](../../server/src/listener.rs)
- State Handler: [gateway/src/server/telnet/state_handler.rs](../../gateway/src/server/telnet/handler.rs)
---
parent: ADR
nav_order: 0018
title: Input Mode Architecture
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0018: Input Mode Architecture

## Context and Problem Statement

Different game activities require different input handling:
- Normal gameplay: Line-buffered commands
- Text editing: Keystroke-buffered with special keys
- Dialogue: Context-aware input
- Combat: Fast command processing

How should we handle different input modes while maintaining protocol independence?

## Decision Drivers

* **User Experience**: Appropriate input handling for each activity
* **Protocol Independence**: Works across all protocols
* **Flexibility**: Easy to add new input modes
* **State Management**: Clear mode transitions
* **Special Keys**: Support Ctrl+Enter, Ctrl+Escape, etc.
* **Buffer Management**: Efficient text buffering

## Considered Options

* Dual Input Mode System (Playing/Editing)
* Single Mode with Flags
* Context-Based Input Routing
* Protocol-Specific Input Handling

## Decision Outcome

Chosen option: "Dual Input Mode System", because it provides clear separation between normal gameplay and editing while maintaining protocol independence.

### Input Mode Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Gateway Input Modes                         │
│                                                          │
│  ┌────────────────────┐      ┌────────────────────┐   │
│  │   Playing Mode     │◄────►│   Editing Mode     │   │
│  │  (Line-buffered)   │      │ (Keystroke-buffer) │   │
│  └────────────────────┘      └────────────────────┘   │
│           │                            │                │
│           │ Commands                   │ Text + Keys    │
│           ▼                            ▼                │
│  ┌─────────────────────────────────────────────────┐  │
│  │          SendInput RPC                          │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              Server Command Routing                      │
│  • Playing: Route to command system                     │
│  • Editing: Buffer text, handle save/cancel            │
└─────────────────────────────────────────────────────────┘
```

### Input Modes

**1. Playing Mode (Default)**
- Line-buffered input
- Trim whitespace
- Send complete lines
- Normal command processing

**2. Editing Mode**
- Keystroke-buffered input
- Maintain local buffer
- Special key handling:
  - **Ctrl+Enter**: Save and exit
  - **Ctrl+Escape**: Cancel and exit
  - **Backspace**: Delete character
  - **Enter**: New line in buffer

### Positive Consequences

* **Clear Separation**: Distinct handling for different activities
* **Protocol Independent**: Works with Telnet, WebSocket, etc.
* **User Friendly**: Appropriate input for each context
* **Extensible**: Easy to add new modes
* **Testable**: Each mode tested independently

### Negative Consequences

* **Mode Switching**: Must manage transitions
* **State Synchronization**: Gateway and server must agree on mode
* **Complexity**: Two input paths to maintain

## Implementation Details

### Gateway-Side Implementation

**Location:** `gateway/src/server/telnet/state_handler.rs`

```rust
pub enum InputMode {
    Playing,
    Editing {
        buffer: String,
        prompt: String,
    },
}

impl StateHandler {
    pub async fn handle_input(&mut self, input: &str) -> Result<()> {
        match &mut self.input_mode {
            InputMode::Playing => {
                // Line-buffered: send complete line
                let trimmed = input.trim();
                if !trimmed.is_empty() {
                    self.send_input(trimmed).await?;
                }
            }
            InputMode::Editing { buffer, .. } => {
                // Keystroke-buffered: handle special keys
                match input {
                    "\x0D\x0A" => {
                        // Ctrl+Enter: Save
                        self.finish_editing(buffer.clone()).await?;
                    }
                    "\x1B" => {
                        // Ctrl+Escape: Cancel
                        self.finish_editing(String::new()).await?;
                    }
                    "\x7F" | "\x08" => {
                        // Backspace: Delete character
                        buffer.pop();
                    }
                    "\n" => {
                        // Enter: New line
                        buffer.push('\n');
                    }
                    _ => {
                        // Regular character: Add to buffer
                        buffer.push_str(input);
                    }
                }
            }
        }
        Ok(())
    }
}
```

### Server-Side Handling

**Location:** `server/src/listener.rs`

```rust
pub async fn send_input(&self, request: SendInputRequest) -> Result<SendInputResponse> {
    let session_state = self.get_session_state(&request.session_id).await?;
    
    match session_state {
        ServerSessionState::Playing { character_id } => {
            // Route to command system
            self.execute_command(character_id, &request.input).await?;
        }
        ServerSessionState::Editing { character_id, context } => {
            // Handle editing completion
            if request.input.is_empty() {
                // Cancelled
                self.cancel_editing(character_id, context).await?;
            } else {
                // Saved
                self.save_editing(character_id, context, &request.input).await?;
            }
            // Transition back to Playing
            self.transition_to_playing(character_id).await?;
        }
        _ => {
            return Err("Invalid state for input".into());
        }
    }
    
    Ok(SendInputResponse::default())
}
```

### Mode Transitions

**Enter Editing Mode:**
```
1. Server: Determine editing needed (e.g., room description)
2. Server: Send BeginEditing RPC to gateway
3. Gateway: Switch to Editing mode
4. Gateway: Display editing prompt
5. User: Type text with special keys
```

**Exit Editing Mode:**
```
1. User: Press Ctrl+Enter (save) or Ctrl+Escape (cancel)
2. Gateway: Send FinishEditing RPC with buffer content
3. Server: Process saved text or discard
4. Server: Transition to Playing state
5. Gateway: Switch to Playing mode
```

### Use Cases

**1. Room Description Editing:**
```
> room edit description
[Entering edit mode. Ctrl+Enter to save, Ctrl+Escape to cancel]
A dark and mysterious cave.
The walls glimmer with strange crystals.
^Enter (save)
Room description updated.
```

**2. Item Description Editing:**
```
> item edit 123e4567 description
[Entering edit mode. Ctrl+Enter to save, Ctrl+Escape to cancel]
An ancient sword with glowing runes.
The blade hums with magical energy.
^Escape (cancel)
Edit cancelled.
```

**3. NPC Dialogue Editing:**
```
> npc edit 123e4567 greeting
[Entering edit mode. Ctrl+Enter to save, Ctrl+Escape to cancel]
Greetings, traveler! Welcome to my shop.
^Enter (save)
NPC greeting updated.
```

## Validation

Input mode architecture is validated by:

1. **Unit Tests**: Mode switching logic
2. **Integration Tests**: Full editing flow
3. **Protocol Tests**: Works with Telnet and WebSocket
4. **User Testing**: Usability of editing interface
5. **Special Key Tests**: Ctrl+Enter, Ctrl+Escape handling

## More Information

### Protocol-Specific Considerations

**Telnet:**
- Raw mode for keystroke capture
- ANSI escape sequences for special keys
- Line discipline handling

**WebSocket:**
- JavaScript keydown events
- Key code mapping
- Browser compatibility

### Buffer Management

**Editing Buffer:**
- Stored in gateway session state
- Maximum size limit (e.g., 64KB)
- Automatic cleanup on mode exit

**Performance:**
- Minimal overhead for Playing mode
- Efficient string operations in Editing mode
- No server round-trips for keystrokes

### Future Enhancements

1. **Syntax Highlighting**: Color coding in edit mode
2. **Line Numbers**: Display line numbers
3. **Undo/Redo**: Edit history
4. **Search/Replace**: Text manipulation
5. **Multiple Buffers**: Edit multiple texts
6. **Collaborative Editing**: Multiple users editing

### Related Decisions

- [ADR-0006](ADR-0006-Layered-State-Machine-Architecture.md) - Input modes are gateway substates
- [ADR-0012](ADR-0012-Session-State-Management-Strategy.md) - Input modes integrate with session states
- [ADR-0009](ADR-0009-Protocol-Independence-Design.md) - Protocol-independent input handling

### References

- State Handler: [gateway/src/server/telnet/state_handler.rs](../../gateway/src/server/telnet/handler.rs)
- Session Management: [gateway/src/session.rs](../../gateway/src/session.rs)
- Server Listener: [server/src/listener.rs](../../server/src/listener.rs)
- Session State Engine: [docs/development/SESSION_STATE_ENGINE.md](../development/SESSION_STATE_ENGINE.md)
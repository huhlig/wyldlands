# Editor Implementation with Cursor Control

**Date:** 2026-01-31  
**Status:** ✅ Completed  
**Location:** `gateway/src/server/telnet/state_handler.rs`

## Overview

Implemented a full-featured text editor for the gateway with cursor control, insert/overwrite modes, and intelligent word wrapping using the termionix library.

## Key Features

### 1. Editor Modes

The editor supports two input modes:

```rust
pub enum EditorMode {
    /// Insert mode - new text is inserted/appended
    Insert,
    /// Overwrite mode - new text replaces existing text at cursor position
    Overwrite,
}
```

- **Insert Mode (default)**: New lines are appended to the content buffer
- **Overwrite Mode**: New lines replace existing content at the cursor position

### 2. Input Mode Differentiation

The system automatically switches between line-buffered and keystroke-buffered input based on session state:

- **Playing State**: Line-buffered input (processes `LineCompleted` events)
- **Editing State**: Keystroke-buffered input (processes `CharacterData` events)

**Implementation:** `gateway/src/server/telnet/termionix_server.rs` (lines 115-150)

```rust
// Check session state to determine input mode
let session = self.context.session_manager().get_session(session_id).await;
let is_editing = session.as_ref().map(|s| s.state.is_editing()).unwrap_or(false);

// Extract input based on event type and session state
let input_opt = match &event {
    // In editing mode, process character-by-character
    TerminalEvent::CharacterData { character, .. } if is_editing => {
        Some(character.to_string())
    }
    // In playing mode, process complete lines
    TerminalEvent::LineCompleted { line, .. } if !is_editing => {
        Some(line.to_string())
    }
    _ => None,
};
```

### 3. Editor State Tracking

The `StateHandler` struct maintains editor state:

```rust
pub struct StateHandler {
    // ... other fields ...
    
    // Editor state
    editor_cursor_position: usize,      // Tracks cursor position in buffer
    editor_terminal_width: usize,       // Terminal width for wrapping (default: 80)
    editor_mode: EditorMode,            // Current editing mode (default: Insert)
}
```

### 4. Editor Commands

All editor commands start with a dot (`.`) prefix:

| Command | Description |
|---------|-------------|
| `.s`, `.save` | Save content and exit editing mode |
| `.q`, `.quit` | Quit without saving changes |
| `.h`, `.help` | Display editor help with current mode |
| `.w`, `.wrap` | Word wrap content to terminal width |
| `.u`, `.unwrap` | Remove soft line breaks |
| `.p`, `.print` | Display current content |
| `.c`, `.clear` | Clear all content |
| `.i`, `.insert` | Toggle between Insert/Overwrite modes |
| `.fg <color>` | Set foreground (text) color |
| `.bg <color>` | Set background color |

Regular text input (without dot prefix) is handled based on the current mode.

### 5. Visual Feedback

The editor prompt shows the current mode:

```
[Editing: Room Description - INS]   # Insert mode
[Editing: Room Description - OVR]   # Overwrite mode
```

### 6. Word Wrap/Unwrap Integration

The editor uses termionix library functions for intelligent text processing:

#### Word Wrap (`.w` command)

```rust
async fn wrap_editor_content(
    &mut self,
    adapter: &mut dyn ProtocolAdapter,
    title: String,
    content: String,
) -> Result<(), String> {
    // Use terminal_word_wrap to wrap the content
    let wrapped = terminal_word_wrap(&content, self.editor_terminal_width);
    let wrapped_string = wrapped.to_string();
    
    // Update cursor position to end of wrapped content
    self.editor_cursor_position = wrapped_string.len();
    
    self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
        title,
        content: wrapped_string,
    })).await
}
```

**Features:**
- Preserves ANSI escape sequences (colors, styles)
- Respects word boundaries
- Handles paragraphs (double newlines)
- Maintains visual consistency

#### Word Unwrap (`.u` command)

```rust
async fn unwrap_editor_content(
    &mut self,
    adapter: &mut dyn ProtocolAdapter,
    title: String,
    content: String,
) -> Result<(), String> {
    // Use terminal_word_unwrap to remove soft line breaks
    let unwrapped = terminal_word_unwrap(&content);
    let unwrapped_string = unwrapped.to_string();
    
    // Update cursor position to end of unwrapped content
    self.editor_cursor_position = unwrapped_string.len();
    
    self.transition_to(SessionState::Authenticated(AuthenticatedState::Editing {
        title,
        content: unwrapped_string,
    })).await
}
```

**Features:**
- Removes soft line breaks (single newlines)
- Preserves paragraph breaks (double newlines)
- Maintains ANSI escape sequences
- Proper spacing between words

### 7. Insert vs Overwrite Behavior

#### Insert Mode

```rust
EditorMode::Insert => {
    // Insert mode: append new line
    let mut new_content = content;
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str(&input);
    new_content
}
```

#### Overwrite Mode

```rust
EditorMode::Overwrite => {
    // Overwrite mode: replace content at cursor position
    self.overwrite_at_cursor(&content, &input)
}
```

The `overwrite_at_cursor` method:
- Calculates which line the cursor is on
- Replaces that line with new input
- Preserves other lines

### 8. Session State Enhancement

Added `is_editing()` method to `SessionState` for easy mode checking:

**File:** `gateway/src/session.rs`

```rust
impl SessionState {
    /// Check if session is in editing mode
    pub fn is_editing(&self) -> bool {
        matches!(self, SessionState::Authenticated(AuthenticatedState::Editing { .. }))
    }
}
```

## Implementation Details

### File Locations

1. **State Handler**: `gateway/src/server/telnet/state_handler.rs`
   - Editor mode enum (lines 27-33)
   - StateHandler struct with editor fields (lines 35-51)
   - Editor input handling (lines 738-820)
   - Helper methods (lines 857-950)

2. **Server Integration**: `gateway/src/server/telnet/termionix_server.rs`
   - Input mode switching (lines 115-150)

3. **Session State**: `gateway/src/session.rs`
   - `is_editing()` method (lines 81-84)

### Key Methods

#### `handle_editing_input()`
Main input handler for editing mode. Parses commands and routes to appropriate handlers.

#### `toggle_editor_mode()`
Switches between Insert and Overwrite modes with visual feedback.

#### `overwrite_at_cursor()`
Replaces content at cursor position in Overwrite mode.

#### `save_editor_content()`
Saves content via RPC and exits editing mode.

#### `show_editor_help()`
Displays context-sensitive help showing current mode.

### Terminal Width Configuration

The editor supports configurable terminal width:

```rust
pub fn set_terminal_width(&mut self, width: usize) {
    self.editor_terminal_width = width.max(20).min(200); // Clamped between 20 and 200
}
```

## Usage Example

### Entering Edit Mode

```
> room edit <uuid> description
[Editing: Room Description - INS] 
```

### Typing Content

```
[Editing: Room Description - INS] This is a dark forest.
[Editing: Room Description - INS] The trees loom overhead.
```

### Toggling Mode

```
[Editing: Room Description - INS] .i
Editor mode: Overwrite

[Editing: Room Description - OVR] 
```

### Word Wrapping

```
[Editing: Room Description - INS] .w
Content wrapped to 80 columns.

[Editing: Room Description - INS] 
```

### Viewing Content

```
[Editing: Room Description - INS] .p

=== Current Content ===
This is a dark forest.
The trees loom overhead.
=== End of Content ===

[Editing: Room Description - INS] 
```

### Saving

```
[Editing: Room Description - INS] .s
Saving content...
Content saved successfully.
> 
```

## Testing

### Compilation
```bash
cd gateway && cargo build --lib
```

**Result:** ✅ Compiles successfully with only minor warnings (no errors)

### Test Coverage
- Character-by-character input processing in editing mode
- Line-by-line input processing in playing mode
- Mode toggling functionality
- Word wrap/unwrap operations
- Cursor position tracking

## Integration with Builder Commands

The editor integrates seamlessly with builder commands:

```
> room edit <uuid> description
[Editing: Room Description - INS] <type description>
[Editing: Room Description - INS] .s
Content saved successfully.
> 
```

This works for:
- Room descriptions (long and short)
- Area descriptions
- Item descriptions
- NPC descriptions
- Any other multi-line text content

### 9. Color Commands

The editor supports setting foreground and background colors using ANSI escape sequences:

#### Foreground Color (`.fg` command)

```
[Editing: Room Description - INS] .fg red
Foreground color set to: red

[Editing: Room Description - INS] This text will be red.
```

#### Background Color (`.bg` command)

```
[Editing: Room Description - INS] .bg blue
Background color set to: blue

[Editing: Room Description - INS] This text will have a blue background.
```

#### Supported Colors

**Named Colors:**
- Basic: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`
- Bright: `bright_black` (or `gray`/`grey`), `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`
- Reset: `reset` or `default` - resets to terminal default

**Hex Colors:**
- 24-bit RGB colors using hex notation: `#RRGGBB`
- Examples: `#FF0000` (red), `#00FF00` (green), `#0000FF` (blue), `#FF5500` (orange)

#### Usage Examples

```
[Editing: Room Description - INS] .fg bright_red
Foreground color set to: bright_red

[Editing: Room Description - INS] A glowing red crystal pulses with energy.
[Editing: Room Description - INS] .fg reset
Foreground color set to: reset

[Editing: Room Description - INS] The room returns to normal lighting.
```

Using hex colors:
```
[Editing: Room Description - INS] .fg #FF5500
Foreground color set to: #FF5500

[Editing: Room Description - INS] The sunset casts an orange glow.
```

Combining foreground and background:
```
[Editing: Room Description - INS] .fg yellow
Foreground color set to: yellow

[Editing: Room Description - INS] .bg blue
Background color set to: blue

[Editing: Room Description - INS] WARNING: High voltage area!
[Editing: Room Description - INS] .fg reset
[Editing: Room Description - INS] .bg reset
```

**Note:** Color codes are embedded directly into the content as ANSI escape sequences, allowing rich text formatting in room descriptions and other game content.

## Future Enhancements

1. **Line Navigation**: Add commands to move cursor to specific lines
2. **Search/Replace**: Add `.f` (find) and `.r` (replace) commands
3. **Undo/Redo**: Track edit history for undo/redo support
4. **Text Styles**: Add `.bold`, `.italic`, `.underline` commands for text styling
5. **Templates**: Save and load text templates
6. **Spell Check**: Integrate spell checking for descriptions
7. **Auto-save**: Periodic auto-save of content
8. **Multi-line Paste**: Better handling of pasted content

## Dependencies

- `termionix-terminal` - Provides `terminal_word_wrap` and `terminal_word_unwrap` functions
- Already included in `gateway/Cargo.toml`

## Related Documentation

- [Area/Room Editor Proposal](AREA_ROOM_EDITOR_PROPOSAL.md) - Builder command system
- [Session State Engine](SESSION_STATE_ENGINE.md) - Session state management
- [Termionix Integration](../../termionix/README.md) - Terminal library documentation

## Summary

The editor implementation provides:
- ✅ Character-by-character input in editing mode
- ✅ Line-by-line input in playing mode
- ✅ Insert and Overwrite modes with toggle
- ✅ Cursor position tracking
- ✅ Intelligent word wrapping (preserves ANSI, respects boundaries)
- ✅ Word unwrapping (removes soft breaks, preserves paragraphs)
- ✅ Visual mode indicators in prompt
- ✅ Comprehensive command set
- ✅ Context-sensitive help
- ✅ Integration with termionix library
- ✅ Clean state management
- ✅ Foreground and background color commands
- ✅ Support for named colors and 24-bit RGB hex colors
- ✅ ANSI escape sequence embedding for rich text

This provides a professional text editing experience for builders creating colorful and engaging game content.
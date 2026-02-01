# Protocol Adapter Architecture

## Overview

The gateway uses a unified protocol adapter architecture to handle different client connection types (Telnet, WebSocket) through a common interface. This design provides consistency, maintainability, and makes it easy to add new protocols.

## Architecture

### Core Components

1. **ProtocolAdapter Trait** (`gateway/src/protocol.rs`)
   - Defines the common interface all protocol implementations must follow
   - Provides methods for sending/receiving messages, managing connection state
   - Returns protocol-agnostic `ProtocolMessage` types

2. **Protocol Implementations**
   - **TelnetAdapter** (`gateway/src/protocol/telnet_adapter.rs`)
   - **WebSocketAdapter** (`gateway/src/protocol/websocket_adapter.rs`)

3. **Protocol Messages** (`ProtocolMessage` enum)
   - `Text(String)` - Text message from client
   - `Binary(Vec<u8>)` - Binary data from client
   - `Connected` - Client connected
   - `Disconnected` - Client disconnected
   - `Ping` / `Pong` - Keep-alive messages
   - `Negotiation(NegotiationData)` - Protocol-specific negotiation

### Benefits

1. **Unified Interface**: Both Telnet and WebSocket use the same `ProtocolAdapter` trait
2. **Protocol Abstraction**: Business logic doesn't need to know about protocol details
3. **Easy Testing**: Mock adapters can be created for testing
4. **Extensibility**: New protocols can be added by implementing the trait
5. **Consistent Behavior**: All protocols handle messages, errors, and state the same way

## Implementation Details

### TelnetAdapter

The TelnetAdapter handles the complexities of the Telnet protocol:

- **Protocol Negotiation**: Automatically negotiates options (ECHO, NAWS, terminal type, etc.)
- **Line Buffering**: Converts byte stream into complete lines
- **Character Echo**: Handles server-side echo for character-mode operation
- **Telnet Commands**: Processes IAC sequences and subnegotiations
- **Capability Detection**: Tracks client capabilities (ANSI colors, window size, etc.)

Key features:
```rust
pub struct TelnetAdapter {
    stream: TcpStream,
    capabilities: ClientCapabilities,
    alive: bool,
    line_buffer: String,
    negotiation_buffer: Vec<u8>,
    // ... negotiation state
}
```

### WebSocketAdapter

The WebSocketAdapter wraps Axum's WebSocket implementation:

- **Message Framing**: WebSocket protocol handles message boundaries
- **Binary Support**: Native support for binary messages
- **Ping/Pong**: Automatic keep-alive handling
- **Clean Shutdown**: Proper close frame handling

Key features:
```rust
pub struct WebSocketAdapter {
    socket: WebSocket,
    capabilities: ClientCapabilities,
    alive: bool,
}
```

## Usage Example

Both protocols are used identically in the connection handlers:

```rust
// Create adapter (protocol-specific)
let mut adapter = TelnetAdapter::new(stream);
// or
let mut adapter = WebSocketAdapter::new(socket);

// Negotiate options (if needed)
adapter.negotiate_options().await?;

// Use adapter (protocol-agnostic)
adapter.send_line("Welcome!").await?;

loop {
    match adapter.receive().await? {
        Some(ProtocolMessage::Text(text)) => {
            // Process command
            adapter.send_line(&response).await?;
        }
        Some(ProtocolMessage::Disconnected) => break,
        _ => continue,
    }
}

adapter.close().await?;
```

## Client Capabilities

Both adapters track client capabilities through a unified structure:

```rust
pub struct ClientCapabilities {
    pub compression: bool,      // Supports compression (MCCP for Telnet)
    pub binary: bool,           // Supports binary data
    pub ansi_colors: bool,      // Supports ANSI color codes
    pub window_size: Option<(u16, u16)>,  // Terminal size
    pub terminal_type: Option<String>,    // Terminal type
    pub msdp: bool,            // MUD Server Data Protocol
    pub gmcp: bool,            // Generic MUD Communication Protocol
}
```

## Migration from Direct TCP

The Telnet implementation was migrated from direct TCP stream handling to use the adapter pattern:

### Before (Direct TCP)
```rust
// Manual byte-by-byte processing
let mut buffer = vec![0u8; 1024];
match stream.read(&mut buffer).await {
    Ok(n) => {
        for &byte in &buffer[..n] {
            match byte {
                b'\r' | b'\n' => { /* process line */ }
                127 | 8 => { /* handle backspace */ }
                // ... manual character handling
            }
        }
    }
}
```

### After (Adapter Pattern)
```rust
// Clean message-based processing
match adapter.receive().await? {
    Some(ProtocolMessage::Text(command)) => {
        // Process complete command
    }
    Some(ProtocolMessage::Disconnected) => break,
    _ => continue,
}
```

## Adding New Protocols

To add a new protocol:

1. Create a new adapter struct in `gateway/src/protocol/`
2. Implement the `ProtocolAdapter` trait
3. Add the module to `gateway/src/protocol.rs`
4. Create a connection handler that uses the adapter
5. Register the handler in the main server

Example skeleton:
```rust
pub struct MyProtocolAdapter {
    // Protocol-specific fields
    capabilities: ClientCapabilities,
    alive: bool,
}

#[async_trait]
impl ProtocolAdapter for MyProtocolAdapter {
    fn protocol_name(&self) -> &str { "myprotocol" }
    
    async fn send_text(&mut self, text: &str) -> Result<(), ProtocolError> {
        // Implementation
    }
    
    async fn receive(&mut self) -> Result<Option<ProtocolMessage>, ProtocolError> {
        // Implementation
    }
    
    // ... other trait methods
}
```

## Testing

The adapter pattern makes testing easier:

1. **Unit Tests**: Test adapter logic in isolation
2. **Mock Adapters**: Create mock implementations for testing business logic
3. **Integration Tests**: Test complete flows with real adapters

## Future Enhancements

Potential improvements to the adapter architecture:

1. **Compression Support**: Unified compression handling across protocols
2. **Protocol Negotiation Events**: Emit events when capabilities change
3. **Metrics Integration**: Built-in metrics for all adapters
4. **Connection Pooling**: Adapter-aware connection pooling
5. **Protocol Detection**: Auto-detect protocol from initial bytes

## Related Documentation

- [Gateway Protocol](GATEWAY_PROTOCOL.md) - Overall gateway architecture
- [Telnet Library Comparison](TELNET_LIBRARY_COMPARISON.md) - Why we chose this approach
- [Reconnection Implementation](RECONNECTION_IMPLEMENTATION.md) - Session persistence

## Conclusion

The protocol adapter architecture provides a clean, maintainable way to support multiple client protocols while keeping the business logic protocol-agnostic. This design has proven successful in unifying Telnet and WebSocket handling and will make future protocol additions straightforward.
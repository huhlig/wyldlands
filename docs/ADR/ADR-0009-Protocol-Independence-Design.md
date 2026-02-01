---
parent: ADR
nav_order: 0009
title: Protocol Independence Design
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0009: Protocol Independence Design

## Context and Problem Statement

We need to support multiple client connection protocols (Telnet, WebSocket, future protocols like SSH or mobile apps) while maintaining a single game server codebase. The system must:
- Support multiple protocols simultaneously
- Add new protocols without modifying game logic
- Provide consistent user experience across protocols
- Handle protocol-specific features (ANSI codes, JSON, etc.)
- Maintain clean separation of concerns

How should we design the system to be protocol-independent?

## Decision Drivers

* **Protocol Agnostic**: Game logic should not depend on connection protocol
* **Extensibility**: Easy to add new protocols
* **Consistency**: Same game experience regardless of protocol
* **Maintainability**: Protocol code isolated from game logic
* **Feature Parity**: All protocols support core features
* **Performance**: Minimal overhead for protocol translation
* **Testing**: Each protocol can be tested independently

## Considered Options

* Protocol Adapter Pattern with Gateway Layer
* Protocol-Specific Servers with Shared Game Logic
* Universal Protocol with Client-Side Translation
* Plugin-Based Protocol System

## Decision Outcome

Chosen option: "Protocol Adapter Pattern with Gateway Layer", because it provides the best separation between protocol handling and game logic while allowing easy addition of new protocols.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Client Protocols                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Telnet  │  │WebSocket │  │   SSH    │  │  Mobile  │   │
│  │  Client  │  │  Client  │  │  Client  │  │   App    │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
└───────┼─────────────┼─────────────┼─────────────┼──────────┘
        │             │             │             │
        │ TCP:4000    │ WS:8080     │ SSH:22      │ HTTPS
        │             │             │             │
┌───────▼─────────────▼─────────────▼─────────────▼──────────┐
│                  Protocol Adapters                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  ProtocolAdapter Trait                               │  │
│  │  • handle_input()  - Process protocol-specific input│  │
│  │  • send_output()   - Format protocol-specific output│  │
│  │  • negotiate()     - Handle protocol negotiation    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │TelnetAdapter │  │WebSocketAdapt│  │  SSHAdapter  │     │
│  │• ANSI codes  │  │• JSON msgs   │  │• Terminal    │     │
│  │• IAC/WILL/DO │  │• Binary data │  │• Auth        │     │
│  │• MCCP/MSDP   │  │• Ping/Pong   │  │• Channels    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└──────────────────────────┬───────────────────────────────────┘
                           │ Generic Commands
                           │ (protocol-independent)
┌──────────────────────────▼───────────────────────────────────┐
│                    Game Server                                │
│  • Receives generic text commands                            │
│  • No protocol-specific code                                 │
│  • Returns generic text output                               │
│  • Protocol-agnostic game logic                              │
└──────────────────────────────────────────────────────────────┘
```

### Positive Consequences

* **Complete Protocol Independence**: Game server has zero protocol-specific code
* **Easy Protocol Addition**: New protocols added by implementing adapter trait
* **Consistent Experience**: Same game logic for all protocols
* **Protocol-Specific Features**: Adapters can leverage unique protocol capabilities
* **Clean Testing**: Protocols and game logic tested independently
* **Flexible Output**: Adapters format output appropriately for each protocol
* **Maintainability**: Protocol code isolated in adapters

### Negative Consequences

* **Abstraction Overhead**: Additional layer between client and game logic
* **Feature Limitations**: Must find common denominator for all protocols
* **Complexity**: More components to understand and maintain

## Pros and Cons of the Options

### Protocol Adapter Pattern with Gateway Layer

* Good, because complete separation of protocol and game logic
* Good, because easy to add new protocols
* Good, because each adapter can optimize for its protocol
* Good, because game server is completely protocol-agnostic
* Good, because adapters can be tested independently
* Good, because consistent game experience across protocols
* Neutral, because requires adapter implementation for each protocol
* Bad, because additional abstraction layer
* Bad, because some protocol-specific features may be hard to expose

### Protocol-Specific Servers with Shared Game Logic

```
TelnetServer ──┐
WebSocketServer├──► Shared Game Library
SSHServer ─────┘
```

* Good, because each server optimized for its protocol
* Good, because full access to protocol features
* Neutral, because shared game logic library
* Bad, because game logic may leak into servers
* Bad, because harder to maintain consistency
* Bad, because more code duplication
* Bad, because harder to test

### Universal Protocol with Client-Side Translation

```
Clients → Universal Protocol → Game Server
```

* Good, because single protocol to implement
* Good, because simple server implementation
* Neutral, because requires client-side adapters
* Bad, because limits protocol-specific features
* Bad, because requires custom client for each protocol
* Bad, because harder to support standard protocols (Telnet)

### Plugin-Based Protocol System

* Good, because dynamic protocol loading
* Good, because third-party protocol support
* Neutral, because requires plugin API
* Bad, because more complex architecture
* Bad, because potential security issues
* Bad, because harder to maintain API stability
* Bad, because overkill for current requirements

## Implementation Details

### ProtocolAdapter Trait

**Location:** `gateway/src/protocol/mod.rs`

```rust
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// Handle incoming data from client
    async fn handle_input(&mut self, data: &[u8]) -> Result<Vec<String>>;
    
    /// Format output for client
    async fn send_output(&mut self, output: &GameOutput) -> Result<Vec<u8>>;
    
    /// Handle protocol negotiation
    async fn negotiate(&mut self) -> Result<()>;
    
    /// Get protocol capabilities
    fn capabilities(&self) -> ProtocolCapabilities;
    
    /// Get protocol type
    fn protocol_type(&self) -> ProtocolType;
}

pub struct ProtocolCapabilities {
    pub supports_ansi: bool,
    pub supports_json: bool,
    pub supports_binary: bool,
    pub supports_compression: bool,
    pub supports_side_channels: bool,
}
```

### Telnet Adapter

**Location:** `gateway/src/protocol/telnet.rs`

```rust
pub struct TelnetAdapter {
    codec: TelnetCodec,
    capabilities: ProtocolCapabilities,
    negotiated_options: HashSet<TelnetOption>,
}

#[async_trait]
impl ProtocolAdapter for TelnetAdapter {
    async fn handle_input(&mut self, data: &[u8]) -> Result<Vec<String>> {
        // Parse Telnet protocol (IAC commands, ANSI codes)
        let events = self.codec.decode(data)?;
        
        let mut commands = Vec::new();
        for event in events {
            match event {
                TelnetEvent::Data(text) => {
                    // Strip ANSI codes, convert to UTF-8
                    let clean_text = self.clean_input(&text);
                    commands.push(clean_text);
                }
                TelnetEvent::Command(cmd) => {
                    // Handle IAC commands (WILL, WONT, DO, DONT)
                    self.handle_telnet_command(cmd).await?;
                }
                TelnetEvent::Subnegotiation(sub) => {
                    // Handle MSDP, GMCP, etc.
                    self.handle_subnegotiation(sub).await?;
                }
            }
        }
        
        Ok(commands)
    }
    
    async fn send_output(&mut self, output: &GameOutput) -> Result<Vec<u8>> {
        // Format with ANSI codes if supported
        let formatted = if self.capabilities.supports_ansi {
            self.format_with_ansi(output)
        } else {
            output.text.clone()
        };
        
        // Encode as Telnet protocol
        self.codec.encode(&formatted)
    }
    
    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            supports_ansi: true,
            supports_json: false,
            supports_binary: true,
            supports_compression: self.negotiated_options.contains(&TelnetOption::MCCP),
            supports_side_channels: true, // MSDP, GMCP
        }
    }
}
```

### WebSocket Adapter

**Location:** `gateway/src/protocol/websocket.rs`

```rust
pub struct WebSocketAdapter {
    capabilities: ProtocolCapabilities,
}

#[async_trait]
impl ProtocolAdapter for WebSocketAdapter {
    async fn handle_input(&mut self, data: &[u8]) -> Result<Vec<String>> {
        // Parse WebSocket message
        match Message::from_bytes(data)? {
            Message::Text(text) => {
                // JSON or plain text
                if text.starts_with('{') {
                    self.handle_json_command(&text)
                } else {
                    Ok(vec![text])
                }
            }
            Message::Binary(data) => {
                // Binary protocol (future)
                self.handle_binary_command(&data)
            }
            Message::Ping(_) => {
                // Handle ping/pong
                Ok(vec![])
            }
            _ => Ok(vec![])
        }
    }
    
    async fn send_output(&mut self, output: &GameOutput) -> Result<Vec<u8>> {
        // Format as JSON for web clients
        let json = serde_json::to_string(&OutputMessage {
            type_: output.output_type.to_string(),
            text: output.text.clone(),
            channel: output.channel.clone(),
            timestamp: Utc::now(),
        })?;
        
        Ok(Message::Text(json).into_bytes())
    }
    
    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            supports_ansi: false, // Web client handles formatting
            supports_json: true,
            supports_binary: true,
            supports_compression: false, // HTTP compression
            supports_side_channels: true, // JSON messages
        }
    }
}
```

### Protocol-Independent Game Output

**Location:** `common/src/gateway.rs`

```rust
pub struct GameOutput {
    pub text: String,
    pub output_type: OutputType,
    pub channel: Option<String>,
}

pub enum OutputType {
    Normal,
    Prompt,
    Error,
    System,
    Combat,
    Social,
    Emote,
    Tell,
}
```

**Server generates protocol-independent output:**

```rust
// server/src/listener.rs
let output = GameOutput {
    text: "You see a dark forest ahead.".to_string(),
    output_type: OutputType::Normal,
    channel: None,
};

// Gateway adapter formats for specific protocol
```

### Input Processing Flow

```
1. Client sends data
   Telnet: "look\r\n" with ANSI codes
   WebSocket: {"command": "look"}

2. Protocol Adapter processes
   TelnetAdapter: Strips ANSI, extracts "look"
   WebSocketAdapter: Parses JSON, extracts "look"

3. Gateway sends to server
   SendInput RPC: { session_id, input: "look" }

4. Server processes command
   CommandSystem: Executes "look" command
   Returns: GameOutput { text: "...", type: Normal }

5. Gateway sends to adapter
   Adapter formats for protocol

6. Adapter sends to client
   Telnet: "You see...\r\n" with ANSI colors
   WebSocket: {"type": "normal", "text": "You see..."}
```

### Side Channel Support

**Telnet Side Channels:**
- **MSDP** (Mud Server Data Protocol): Structured data
- **GMCP** (Generic Mud Communication Protocol): JSON data
- **MSSP** (Mud Server Status Protocol): Server info

**WebSocket Side Channels:**
- JSON messages with type field
- Separate channels for different data types

**Implementation:**

```rust
// Telnet MSDP
pub async fn send_msdp_data(&mut self, var: &str, val: &str) {
    let msdp = format!("IAC SB MSDP VAR {} VAL {} IAC SE", var, val);
    self.send_raw(msdp.as_bytes()).await;
}

// WebSocket JSON
pub async fn send_json_data(&mut self, data: &serde_json::Value) {
    let msg = Message::Text(data.to_string());
    self.send_message(msg).await;
}
```

## Validation

The protocol independence design is validated by:

1. **Zero Protocol Code in Server:**
   - Server has no Telnet-specific code
   - Server has no WebSocket-specific code
   - All protocol handling in gateway adapters

2. **Multiple Protocols Working:**
   - WebSocket protocol fully functional
   - Telnet protocol architecture ready
   - Same game experience on both

3. **Easy Protocol Addition:**
   - WebSocket added without touching server
   - Telnet adapter being integrated
   - Future protocols (SSH, mobile) straightforward

4. **Testing Independence:**
   - Server tests don't depend on protocols
   - Protocol adapters tested independently
   - Integration tests verify end-to-end

## More Information

### Protocol-Specific Features

**Telnet:**
- ANSI color codes
- Terminal size negotiation
- MCCP compression
- MSDP/GMCP side channels
- Character encoding negotiation

**WebSocket:**
- JSON structured messages
- Binary data support
- Ping/Pong keepalive
- Automatic reconnection
- Browser-based client

**Future SSH:**
- Secure authentication
- Terminal emulation
- Port forwarding
- Session multiplexing

**Future Mobile:**
- Touch-optimized UI
- Push notifications
- Offline mode
- Native performance

### Adding a New Protocol

1. **Implement ProtocolAdapter trait:**
   ```rust
   pub struct NewProtocolAdapter { ... }
   
   #[async_trait]
   impl ProtocolAdapter for NewProtocolAdapter {
       // Implement required methods
   }
   ```

2. **Register in gateway:**
   ```rust
   match protocol_type {
       ProtocolType::Telnet => Box::new(TelnetAdapter::new()),
       ProtocolType::WebSocket => Box::new(WebSocketAdapter::new()),
       ProtocolType::NewProtocol => Box::new(NewProtocolAdapter::new()),
   }
   ```

3. **Add protocol-specific server:**
   ```rust
   // gateway/src/server/newprotocol.rs
   pub async fn run_newprotocol_server(config: Config) {
       // Accept connections
       // Create NewProtocolAdapter for each connection
       // Register with connection pool
   }
   ```

4. **No server changes needed!**

### Best Practices

**For Protocol Adapters:**
- Keep protocol logic isolated
- Use standard libraries when possible
- Handle protocol errors gracefully
- Support graceful degradation
- Document protocol-specific features

**For Game Server:**
- Never reference specific protocols
- Use generic output types
- Provide rich output metadata
- Support all output types
- Keep commands protocol-agnostic

### Related Decisions

- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Gateway layer enables protocol independence
- [ADR-0006](ADR-0006-Layered-State-Machine-Architecture.md) - Gateway states are protocol-independent
- [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md) - Protocol-independent RPC

### References

- Protocol Adapter Trait: [gateway/src/protocol/mod.rs](../../gateway/src/protocol/mod.rs)
- Telnet Adapter: [gateway/src/protocol/telnet.rs](../../gateway/src/protocol/telnet.rs)
- WebSocket Handler: [gateway/src/server/websocket/handler.rs](../../gateway/src/server/websocket/handler.rs)
- Architecture Documentation: [docs/development/PROTOCOL_ADAPTER_ARCHITECTURE.md](../development/PROTOCOL_ADAPTER_ARCHITECTURE.md)
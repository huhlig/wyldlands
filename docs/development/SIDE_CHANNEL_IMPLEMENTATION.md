# Side Channel Implementation Plan

## Overview

This document outlines the implementation of side channel protocols for structured data transmission in Wyldlands. Side channels allow the server to send structured game data (character stats, room info, combat data, etc.) to clients that support out-of-band communication.

## Supported Protocols

### 1. MSDP (Mud Server Data Protocol) - Telnet Option 69
- **Target Clients**: Traditional MUD clients (TinTin++, Mudlet, MUSHclient)
- **Data Format**: Binary protocol with type markers
- **Capabilities**: Variables, arrays, tables, commands
- **Negotiation**: IAC WILL/DO MSDP

### 2. GMCP (Generic Mud Communication Protocol) - Telnet Option 201
- **Target Clients**: Modern MUD clients (Mudlet, Aardwolf client)
- **Data Format**: JSON over telnet
- **Capabilities**: Package-based structured data
- **Negotiation**: IAC WILL/DO GMCP
- **Special**: Supports MSDP-over-GMCP for compatibility

### 3. WebSocket JSON Side Channel
- **Target Clients**: Web-based clients
- **Data Format**: JSON messages
- **Capabilities**: Full structured data support
- **Negotiation**: WebSocket message-based

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Server (World)                        │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Game Logic generates StructuredOutput             │    │
│  │  (DataValue/DataTable/DataArray from proto)        │    │
│  └────────────────┬───────────────────────────────────┘    │
│                   │                                          │
│                   ▼                                          │
│  ┌────────────────────────────────────────────────────┐    │
│  │  SendOutput RPC with StructuredOutput              │    │
│  └────────────────┬───────────────────────────────────┘    │
└───────────────────┼──────────────────────────────────────────┘
                    │
                    │ gRPC
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    Gateway (Session Layer)                   │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │  RPC Handler receives StructuredOutput             │    │
│  └────────────────┬───────────────────────────────────┘    │
│                   │                                          │
│                   ▼                                          │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Check Session Capabilities                        │    │
│  │  - MSDP enabled?                                   │    │
│  │  - GMCP enabled?                                   │    │
│  │  - WebSocket JSON enabled?                         │    │
│  └────────────────┬───────────────────────────────────┘    │
│                   │                                          │
│                   ▼                                          │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Format Converter                                  │    │
│  │  - StructuredOutput → MSDP binary                  │    │
│  │  - StructuredOutput → GMCP JSON                    │    │
│  │  - StructuredOutput → WebSocket JSON               │    │
│  └────────────────┬───────────────────────────────────┘    │
│                   │                                          │
│                   ▼                                          │
│  ┌────────────────────────────────────────────────────┐    │
│  │  Protocol Adapter sends formatted data             │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                    │
                    ▼
              ┌──────────┐
              │  Client  │
              └──────────┘
```

## Implementation Phases

### Phase 1: Foundation (COMPLETED ✅)
- [x] Enhanced `SessionMetadata` with `SideChannelCapabilities`
- [x] Added capability tracking fields:
  - `msdp`: bool
  - `gmcp`: bool
  - `websocket_json`: bool
  - `msdp_reported_variables`: HashSet<String>
  - `gmcp_supported_packages`: HashSet<String>
  - `msdp_over_gmcp`: bool
- [x] Added helper methods for capability management
- [x] Gateway compiles successfully

### Phase 2: MSDP Encoding/Decoding Module

Create `gateway/src/protocol/msdp.rs`:

```rust
/// MSDP sidechannel constants
pub const MSDP: u8 = 69;
pub const MSDP_VAR: u8 = 1;
pub const MSDP_VAL: u8 = 2;
pub const MSDP_TABLE_OPEN: u8 = 3;
pub const MSDP_TABLE_CLOSE: u8 = 4;
pub const MSDP_ARRAY_OPEN: u8 = 5;
pub const MSDP_ARRAY_CLOSE: u8 = 6;

/// Convert StructuredOutput to MSDP binary format
pub fn encode_msdp(output: &StructuredOutput) -> Vec<u8>;

/// Parse MSDP binary data to structured format
pub fn decode_msdp(data: &[u8]) -> Result<MsdpData, MsdpError>;

/// MSDP command handlers
pub enum MsdpCommand {
    List(String),      // LIST <type>
    Report(Vec<String>), // REPORT <var1> <var2> ...
    Send(Vec<String>),   // SEND <var1> <var2> ...
    Unreport(Vec<String>), // UNREPORT <var1> <var2> ...
    Reset(String),     // RESET <type>
}
```

**Key Functions:**
- `encode_data_value()` - Convert DataValue to MSDP bytes
- `encode_data_table()` - Convert DataTable to MSDP table
- `encode_data_array()` - Convert DataArray to MSDP array
- `parse_msdp_command()` - Parse client MSDP commands

### Phase 3: GMCP Encoding/Decoding Module

Create `gateway/src/protocol/gmcp.rs`:

```rust
/// GMCP sidechannel constants
pub const GMCP: u8 = 201;

/// Convert StructuredOutput to GMCP JSON format
pub fn encode_gmcp(package: &str, output: &StructuredOutput) -> String;

/// Parse GMCP JSON data
pub fn decode_gmcp(data: &str) -> Result<GmcpMessage, GmcpError>;

/// GMCP package structure
pub struct GmcpMessage {
    pub package: String,
    pub data: serde_json::Value,
}

/// MSDP over GMCP support
pub fn encode_msdp_over_gmcp(output: &StructuredOutput) -> String;
```

**Key Functions:**
- `data_value_to_json()` - Convert DataValue to JSON
- `data_table_to_json()` - Convert DataTable to JSON object
- `data_array_to_json()` - Convert DataArray to JSON array
- `parse_gmcp_package()` - Extract package name and data

### Phase 4: WebSocket JSON Module

Create `gateway/src/protocol/websocket_json.rs`:

```rust
/// WebSocket side channel message format
#[derive(Serialize, Deserialize)]
pub struct WsMessage {
    pub msg_type: String,  // "structured_data", "command", etc.
    pub data: serde_json::Value,
}

/// Convert StructuredOutput to WebSocket JSON
pub fn encode_ws_json(output: &StructuredOutput) -> String;

/// Parse WebSocket JSON message
pub fn decode_ws_json(data: &str) -> Result<WsMessage, WsError>;
```

### Phase 5: Protocol Adapter Enhancements

#### Termionix Adapter Updates

Update `gateway/src/server/telnet/termionix_adapter.rs`:

```rust
impl TermionixAdapter {
    /// Send MSDP data
    pub async fn send_msdp(&mut self, data: &[u8]) -> Result<(), ProtocolError>;
    
    /// Send GMCP data
    pub async fn send_gmcp(&mut self, package: &str, json: &str) -> Result<(), ProtocolError>;
    
    /// Handle MSDP negotiation
    async fn handle_msdp_negotiation(&mut self, enabled: bool);
    
    /// Handle GMCP negotiation
    async fn handle_gmcp_negotiation(&mut self, enabled: bool);
    
    /// Process MSDP command from client
    async fn process_msdp_command(&mut self, command: MsdpCommand);
}
```

#### WebSocket Adapter Updates

Update `gateway/src/server/websocket/adapter.rs`:

```rust
impl WebSocketAdapter {
    /// Send structured data via WebSocket JSON
    pub async fn send_structured(&mut self, data: &str) -> Result<(), ProtocolError>;
    
    /// Enable WebSocket JSON side channel
    pub fn enable_json_side_channel(&mut self);
}
```

### Phase 6: Output Routing Layer

Create `gateway/src/output_router.rs`:

```rust
/// Routes structured output to appropriate sidechannel format
pub struct OutputRouter {
    msdp_encoder: MsdpEncoder,
    gmcp_encoder: GmcpEncoder,
    ws_encoder: WsJsonEncoder,
}

impl OutputRouter {
    /// Route structured output based on session capabilities
    pub async fn route_output(
        &self,
        session: &GatewaySession,
        adapter: &mut dyn ProtocolAdapter,
        output: &StructuredOutput,
    ) -> Result<(), String>;
    
    /// Send via preferred side channel
    async fn send_via_side_channel(
        &self,
        capabilities: &SideChannelCapabilities,
        adapter: &mut dyn ProtocolAdapter,
        output: &StructuredOutput,
    ) -> Result<(), String>;
}
```

### Phase 7: RPC Handler Integration

Update `gateway/src/grpc/server.rs`:

```rust
impl WorldToSession for GatewayRpcServer {
    async fn send_output(
        &self,
        request: Request<SendOutputRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let session_id = Uuid::parse_str(&req.session_id)?;
        
        // Get session and check capabilities
        let session = self.session_manager.get_session(session_id).await?;
        
        // Route each output item
        for output in req.output {
            match output.output_type {
                Some(OutputType::Structured(structured)) => {
                    // Use OutputRouter to send via side channel
                    self.output_router.route_output(
                        &session,
                        adapter,
                        &structured,
                    ).await?;
                }
                Some(OutputType::Text(text)) => {
                    // Send as regular text
                    adapter.send_line(&text.content).await?;
                }
                // ... handle other types
            }
        }
        
        Ok(Response::new(Empty {}))
    }
}
```

## Data Flow Examples

### Example 1: Character Stats via MSDP

**Server generates:**
```rust
StructuredOutput {
    output_type: "character".to_string(),
    data: DataValue::Table(DataTable {
        entries: {
            "HEALTH": DataValue::String("85".to_string()),
            "HEALTH_MAX": DataValue::String("100".to_string()),
            "MANA": DataValue::String("42".to_string()),
            "MANA_MAX": DataValue::String("50".to_string()),
        }
    })
}
```

**MSDP encoding:**
```
IAC SB MSDP
  MSDP_VAR "CHARACTER"
  MSDP_VAL MSDP_TABLE_OPEN
    MSDP_VAR "HEALTH" MSDP_VAL "85"
    MSDP_VAR "HEALTH_MAX" MSDP_VAL "100"
    MSDP_VAR "MANA" MSDP_VAL "42"
    MSDP_VAR "MANA_MAX" MSDP_VAL "50"
  MSDP_TABLE_CLOSE
IAC SE
```

### Example 2: Room Data via GMCP

**Server generates:**
```rust
StructuredOutput {
    output_type: "room".to_string(),
    data: DataValue::Table(DataTable {
        entries: {
            "vnum": DataValue::String("1001".to_string()),
            "name": DataValue::String("Town Square".to_string()),
            "exits": DataValue::Array(DataArray {
                values: vec![
                    DataValue::String("north".to_string()),
                    DataValue::String("south".to_string()),
                ]
            })
        }
    })
}
```

**GMCP encoding:**
```
IAC SB GMCP Room.Info {
  "vnum": "1001",
  "name": "Town Square",
  "exits": ["north", "south"]
} IAC SE
```

### Example 3: Combat Data via WebSocket JSON

**Server generates:**
```rust
StructuredOutput {
    output_type: "combat".to_string(),
    data: DataValue::Table(DataTable {
        entries: {
            "attacker": DataValue::String("Goblin".to_string()),
            "target": DataValue::String("You".to_string()),
            "damage": DataValue::String("15".to_string()),
            "hit_type": DataValue::String("slash".to_string()),
        }
    })
}
```

**WebSocket JSON:**
```json
{
  "msg_type": "structured_data",
  "data": {
    "type": "combat",
    "attacker": "Goblin",
    "target": "You",
    "damage": "15",
    "hit_type": "slash"
  }
}
```

## Testing Strategy

### Unit Tests
- MSDP encoding/decoding
- GMCP encoding/decoding
- WebSocket JSON encoding/decoding
- Data format conversions

### Integration Tests
- End-to-end MSDP flow
- End-to-end GMCP flow
- End-to-end WebSocket JSON flow
- MSDP-over-GMCP compatibility
- Multiple simultaneous clients with different capabilities

### Performance Tests
- Encoding performance for large data structures
- Memory usage for buffered side channel data
- Throughput with multiple active side channels

## Client Support Matrix

| Client | MSDP | GMCP | WebSocket JSON | Notes |
|--------|------|------|----------------|-------|
| TinTin++ | ✅ | ✅ | ❌ | Full MSDP support |
| Mudlet | ⚠️ | ✅ | ❌ | Limited MSDP (no control codes) |
| MUSHclient | ✅ | ✅ | ❌ | Full support |
| Web Client | ❌ | ❌ | ✅ | Custom implementation |
| Telnet (raw) | ❌ | ❌ | ❌ | Text only |

## Implementation Priority

1. **High Priority**
   - MSDP encoding/decoding (most widely supported)
   - Basic GMCP support (modern clients)
   - Output routing infrastructure

2. **Medium Priority**
   - WebSocket JSON (web clients)
   - MSDP-over-GMCP compatibility
   - Advanced GMCP packages

3. **Low Priority**
   - Performance optimizations
   - Extended MSDP variables
   - Custom GMCP packages

## References

- [MSDP Specification](https://tintin.mudhalla.net/protocols/msdp/)
- [GMCP Specification](https://www.gammon.com.au/gmcp)
- [Termionix Documentation](../../../termionix/README.md)
- [Gateway Protocol Definition](../../common/proto/gateway.proto)

## Status

- **Phase 1**: ✅ Complete (Session capability tracking)
- **Phase 2**: ⏳ Next (MSDP implementation)
- **Phase 3**: 📋 Planned (GMCP implementation)
- **Phase 4**: 📋 Planned (WebSocket JSON)
- **Phase 5**: 📋 Planned (Adapter enhancements)
- **Phase 6**: 📋 Planned (Output routing)
- **Phase 7**: 📋 Planned (RPC integration)

---

*Last Updated: 2026-01-31*
*Author: Bob (AI Assistant)*
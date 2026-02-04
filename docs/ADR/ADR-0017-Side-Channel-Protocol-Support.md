---
parent: ADR
nav_order: 0017
title: Side Channel Protocol Support
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0017: Side Channel Protocol Support

## Context and Problem Statement

Modern MUD clients support side-channel protocols for enhanced features beyond basic text. We need to support:
- MSDP (Mud Server Data Protocol) - Structured data
- GMCP (Generic Mud Communication Protocol) - JSON-based data
- WebSocket JSON - Web client data channel
- Future protocol extensions

How should we design side-channel support to be protocol-independent and extensible?

## Decision Drivers

* **Protocol Independence**: Support multiple side-channel protocols
* **Extensibility**: Easy to add new protocols
* **Type Safety**: Structured data with validation
* **Performance**: Efficient serialization/deserialization
* **Client Compatibility**: Work with popular MUD clients
* **Maintainability**: Clean protocol abstraction

## Considered Options

* Multi-Protocol Side Channel Architecture
* Single Protocol (GMCP Only)
* Custom Binary Protocol
* No Side Channels (Text Only)

## Decision Outcome

Chosen option: "Multi-Protocol Side Channel Architecture", because it provides maximum client compatibility while maintaining clean abstractions.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Gateway Layer                           │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │    MSDP      │  │     GMCP     │  │  WebSocket   │ │
│  │   Handler    │  │   Handler    │  │  JSON        │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
│         │                  │                  │         │
│         └──────────────────┼──────────────────┘         │
│                            │                            │
│                   ┌────────▼────────┐                   │
│                   │  Side Channel   │                   │
│                   │    Manager      │                   │
│                   └────────┬────────┘                   │
└────────────────────────────┼──────────────────────────┘
                             │
                             │ Structured Data
                             ▼
┌─────────────────────────────────────────────────────────┐
│                   Server Layer                           │
│  • Character stats                                       │
│  • Room data                                             │
│  • Combat events                                         │
│  • Map data                                              │
└─────────────────────────────────────────────────────────┘
```

### Supported Protocols

**1. MSDP (Mud Server Data Protocol)**
- Telnet-based structured data
- Key-value pairs
- Arrays and tables
- Used by: MUSHclient, Mudlet

**2. GMCP (Generic Mud Communication Protocol)**
- JSON-based data
- Hierarchical structure
- Event-driven
- Used by: Mudlet, TinTin++

**3. WebSocket JSON**
- Native JSON over WebSocket
- Bidirectional communication
- Web client support
- Custom protocol

### Positive Consequences

* **Client Compatibility**: Works with popular MUD clients
* **Rich Features**: Enable advanced client features
* **Protocol Independence**: Gateway handles protocol details
* **Extensibility**: Easy to add new protocols
* **Type Safety**: Structured data validation

### Negative Consequences

* **Complexity**: Multiple protocol implementations
* **Testing**: Must test each protocol variant
* **Maintenance**: Keep protocols synchronized

## Implementation Details

### Side Channel Messages

**Character Stats:**
```json
{
  "type": "character.stats",
  "data": {
    "health": { "current": 100, "max": 100 },
    "mana": { "current": 50, "max": 50 },
    "level": 5,
    "experience": 1250
  }
}
```

**Room Data:**
```json
{
  "type": "room.info",
  "data": {
    "id": "room-123",
    "name": "Town Square",
    "exits": ["north", "south", "east", "west"],
    "players": ["Alice", "Bob"],
    "npcs": ["Guard", "Merchant"]
  }
}
```

**Combat Events:**
```json
{
  "type": "combat.damage",
  "data": {
    "attacker": "Goblin",
    "target": "Player",
    "damage": 15,
    "type": "physical"
  }
}
```

### Protocol Handlers

**MSDP Handler:**
```rust
pub struct MsdpHandler {
    enabled: bool,
    variables: HashSet<String>,
}

impl MsdpHandler {
    pub fn send_variable(&self, key: &str, value: &str) -> Vec<u8> {
        // MSDP sidechannel encoding
        let mut data = vec![IAC, SB, MSDP, MSDP_VAR];
        data.extend_from_slice(key.as_bytes());
        data.push(MSDP_VAL);
        data.extend_from_slice(value.as_bytes());
        data.extend_from_slice(&[IAC, SE]);
        data
    }
}
```

**GMCP Handler:**
```rust
pub struct GmcpHandler {
    enabled: bool,
    modules: HashSet<String>,
}

impl GmcpHandler {
    pub fn send_message(&self, module: &str, data: &serde_json::Value) -> Vec<u8> {
        let json = format!("{} {}", module, data.to_string());
        let mut msg = vec![IAC, SB, GMCP];
        msg.extend_from_slice(json.as_bytes());
        msg.extend_from_slice(&[IAC, SE]);
        msg
    }
}
```

**WebSocket JSON Handler:**
```rust
pub struct WebSocketJsonHandler;

impl WebSocketJsonHandler {
    pub async fn send_message(&self, ws: &WebSocket, msg: &SideChannelMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        ws.send(Message::Text(json)).await?;
        Ok(())
    }
}
```

### Client Negotiation

**MSDP Negotiation:**
```
Client: IAC DO MSDP
Server: IAC WILL MSDP
Client: IAC SB MSDP MSDP_VAR "LIST" MSDP_VAL "COMMANDS" IAC SE
Server: IAC SB MSDP MSDP_VAR "COMMANDS" MSDP_VAL "REPORT" ... IAC SE
```

**GMCP Negotiation:**
```
Client: IAC DO GMCP
Server: IAC WILL GMCP
Client: Core.Hello { "client": "Mudlet", "version": "4.10" }
Server: Core.Supports.Set ["Char 1", "Room 1", "Combat 1"]
```

### Data Types

**Common Side Channel Data:**
- Character vitals (health, mana, stamina)
- Character stats (attributes, skills)
- Room information (name, exits, entities)
- Combat events (damage, healing, status)
- Inventory updates
- Map data
- Quest status
- Group information

## Validation

Side channel support is validated by:

1. **Protocol Tests**: Test each protocol implementation
2. **Client Testing**: Test with real MUD clients
3. **Integration Tests**: End-to-end side channel flow
4. **Performance Tests**: Measure overhead
5. **Compatibility Tests**: Test with multiple clients

## More Information

### Supported Clients

**MSDP:**
- MUSHclient
- Mudlet (legacy)
- SimpleMU

**GMCP:**
- Mudlet
- TinTin++
- MUSHclient (newer versions)

**WebSocket JSON:**
- Custom web client
- Browser-based clients

### Performance Considerations

- Side channel messages are async
- Batching for high-frequency updates
- Rate limiting to prevent spam
- Selective updates (only changed data)

### Future Enhancements

1. **MSSP (Mud Server Status Protocol)**: Server discovery
2. **MXP (MUD eXtension Protocol)**: Rich text formatting
3. **ATCP (Achaea Telnet Client Protocol)**: IRE games protocol
4. **Custom Extensions**: Game-specific protocols

### Related Decisions

- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Gateway handles protocol details
- [ADR-0009](ADR-0009-Protocol-Independence-Design.md) - Protocol-independent design
- [ADR-0012](ADR-0012-Session-State-Management-Strategy.md) - Side channels work with session states

### References

- MSDP Specification: [termionix/doc/msdp.md](../../termionix/doc/msdp.md)
- GMCP Specification: [termionix/doc/gmcp.md](../../termionix/doc/gmcp.md)
- Protocol Handler: [gateway/src/protocol/](../../gateway/src/sidechannel/)
- Session State Engine: [docs/development/SESSION_STATE_ENGINE.md](../development/SESSION_STATE_ENGINE.md)
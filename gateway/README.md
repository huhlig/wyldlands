# Wyldlands Gateway

The Wyldlands Gateway is a high-performance connection gateway that handles multiple protocol types (Telnet, WebSocket, Web) and manages session state for the Wyldlands MUD server.

## Architecture

The gateway acts as a protocol adapter layer between clients and the world server:

```
Clients (Telnet/WebSocket/Web)
    ↓
Gateway (Protocol Adapters)
    ↓
Session Management
    ↓
gRPC Connection Pool
    ↓
World Server
```

## Features

### Protocol Support

- **Telnet** - Full MUD client support via Termionix library
  - RFC 854 compliant Telnet protocol
  - RFC 1143 Q-state option negotiation
  - ANSI color support
  - MUD protocols: GMCP, MSDP, MSSP, MCCP, NAWS
  - Terminal type negotiation
  - Window size negotiation (NAWS)
  
- **WebSocket** - Modern web-based clients
  - Real-time bidirectional communication
  - JSON message protocol
  - Automatic reconnection support
  
- **Web** - HTTP/REST API
  - Admin interface
  - Status monitoring
  - Session management

### Session Management

- Unique session IDs (UUID v4)
- State machine with transitions:
  - `Connecting` → `Connected` → `Authenticated` → `Playing`
  - Support for `Disconnected` state with reconnection tokens
- Session expiration and cleanup
- Concurrent session handling
- Session metadata storage

### Connection Pool

- Efficient message routing to active sessions
- Broadcast capabilities
- Connection lifecycle management
- Automatic cleanup of disconnected sessions

### Reconnection Support

- Time-limited reconnection tokens
- Secure token encoding/decoding
- Automatic session restoration
- Configurable token TTL

## Termionix Integration

As of 2026-01-31, the gateway uses the **Termionix** library for all Telnet connections, replacing the previous custom implementation.

### Benefits

- ✅ **Production-ready**: Battle-tested telnet implementation
- ✅ **RFC Compliant**: Proper Q-state option negotiation (RFC 1143)
- ✅ **Feature-rich**: Comprehensive MUD protocol support
- ✅ **Observable**: Built-in metrics and tracing
- ✅ **Maintainable**: Reduced codebase by 595 lines

### Key Components

1. **TermionixTelnetServer** (`src/server/telnet/termionix_server.rs`)
   - Main server using Termionix service layer
   - Manages connection lifecycle
   - Integrates with session management

2. **TermionixAdapter** (`src/server/telnet/termionix_adapter.rs`)
   - Implements `ProtocolAdapter` trait
   - Bridges Termionix events to gateway protocol messages
   - Tracks client capabilities

3. **StateHandler** (`src/server/telnet/state_handler.rs`)
   - Manages authentication flow
   - Handles state transitions
   - Processes user input

### Configuration

Telnet server configuration in `config.yaml`:

```yaml
telnet:
  addr: "0.0.0.0:4000"  # Bind address and port
```

Environment variable override:

```bash
TELNET_ADDR="0.0.0.0:4000"
```

## Building

```bash
# Build the gateway
cargo build --release

# Run tests
cargo nextest run

# Run benchmarks
cargo bench
```

## Running

```bash
# With default config
cargo run --release

# With custom config
cargo run --release -- --config path/to/config.yaml

# With environment file
cargo run --release -- --env path/to/.env
```

## Configuration

### Environment Variables

- `TELNET_ADDR` - Telnet server bind address (default: `0.0.0.0:4000`)
- `WEBSOCKET_ADDR` - WebSocket server bind address (default: `0.0.0.0:8080`)
- `WORLD_SERVER_ADDR` - World server gRPC address (default: `localhost:6006`)
- `WORLD_SERVER_AUTH_KEY` - Authentication key for world server

### Config File

See `config.yaml` for full configuration options.

## Testing

The gateway includes comprehensive test coverage:

- **Unit Tests**: Component-level testing
- **Integration Tests**: Full lifecycle testing
- **Benchmarks**: Performance testing

```bash
# Run all tests
cargo nextest run

# Run specific test suite
cargo nextest run --test session_integration_tests

# Run benchmarks
cargo bench
```

## Metrics

The gateway exposes metrics for monitoring:

### Termionix Metrics

- `termionix.connections.total` - Total connections created
- `termionix.connections.active` - Currently active connections
- `termionix.messages.sent` - Messages sent to clients
- `termionix.messages.received` - Messages received from clients
- `termionix.message.send_duration` - Send latency histogram
- `termionix.message.receive_duration` - Receive latency histogram
- `termionix.errors.send` - Send error count
- `termionix.errors.receive` - Receive error count

### Session Metrics

- Session creation/deletion rates
- Active session count
- Session state distribution
- Authentication success/failure rates

## Development

### Adding New Protocol Support

1. Implement the `ProtocolAdapter` trait
2. Create a server implementation
3. Register in `main.rs`
4. Add configuration support

### Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

Enable trace logging for specific modules:

```bash
RUST_LOG=wyldlands_gateway::server::telnet=trace cargo run
```

## Architecture Decisions

### Why Termionix?

The gateway previously used a custom telnet implementation (~595 lines). We migrated to Termionix for:

1. **Standards Compliance**: RFC 1143 Q-state machine
2. **Feature Completeness**: Full MUD protocol support
3. **Maintainability**: Well-tested, documented library
4. **Observability**: Built-in metrics and tracing
5. **Performance**: Optimized async implementation

See `TERMIONIX_INTEGRATION_REFACTOR.md` for full migration details.

### Session State Machine

The gateway uses a strict state machine to ensure proper authentication flow:

```
Connecting → Connected → Authenticated(CharacterCreation)
                              ↓
                         Authenticated(CharacterSelection)
                              ↓
                         Authenticated(Playing)
                              ↓
                         Disconnected (with reconnection token)
```

Invalid transitions are rejected to maintain security and consistency.

## Contributing

When contributing to the gateway:

1. Ensure all tests pass: `cargo nextest run`
2. Run clippy: `cargo clippy --all-targets`
3. Format code: `cargo fmt`
4. Update documentation for new features
5. Add tests for new functionality

## License

See LICENSE.md in the project root.

## References

- [Termionix Library](https://github.com/huhlig/termionix)
- [RFC 854 - Telnet Protocol](https://tools.ietf.org/html/rfc854)
- [RFC 1143 - Q Method of Option Negotiation](https://tools.ietf.org/html/rfc1143)
- [TERMIONIX_INTEGRATION_REFACTOR.md](../TERMIONIX_INTEGRATION_REFACTOR.md) - Integration details
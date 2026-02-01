---
parent: ADR
nav_order: 0007
title: Use gRPC for Inter-Service Communication
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0007: Use gRPC for Inter-Service Communication

## Context and Problem Statement

With a separated gateway and server architecture, we need a communication protocol for inter-service communication. The protocol must support:
- Bidirectional communication (gateway ↔ server)
- Type-safe message definitions
- Efficient serialization
- Streaming capabilities (for future features)
- Good Rust ecosystem support
- Low latency (<1ms overhead)

What protocol should we use for communication between gateway and server?

## Decision Drivers

* **Type Safety**: Compile-time verification of message structures
* **Performance**: Low latency and efficient serialization
* **Bidirectional**: Both gateway and server can initiate calls
* **Tooling**: Good code generation and IDE support
* **Ecosystem**: Mature Rust libraries
* **Streaming**: Support for streaming responses (future)
* **Versioning**: Easy to evolve protocol over time
* **Documentation**: Self-documenting protocol definitions

## Considered Options

* gRPC with Protocol Buffers
* REST/HTTP with JSON
* WebSocket with JSON
* MessagePack over TCP
* Custom Binary Protocol

## Decision Outcome

Chosen option: "gRPC with Protocol Buffers", because it provides the best combination of type safety, performance, bidirectional communication, and ecosystem support for Rust.

We use the **Tonic** library as our gRPC implementation, which provides excellent Rust integration and performance.

### Positive Consequences

* **Type Safety**: Protocol Buffers provide compile-time type checking
* **Performance**: Binary serialization is fast and compact
* **Code Generation**: Automatic generation of Rust types and client/server code
* **Bidirectional**: Both services can define RPC methods
* **Streaming**: Built-in support for streaming (for future features)
* **Versioning**: Protocol Buffers support backward/forward compatibility
* **Documentation**: .proto files serve as protocol documentation
* **Ecosystem**: Tonic is mature and well-maintained
* **HTTP/2**: Built on HTTP/2 with multiplexing and flow control

### Negative Consequences

* **Complexity**: More complex than simple REST/JSON
* **Debugging**: Binary protocol harder to inspect than JSON
* **Learning Curve**: Requires understanding Protocol Buffers
* **Build Time**: Code generation adds to build time

## Pros and Cons of the Options

### gRPC with Protocol Buffers

* Good, because strongly typed with compile-time verification
* Good, because efficient binary serialization
* Good, because bidirectional RPC support
* Good, because excellent Rust support via Tonic
* Good, because built-in streaming capabilities
* Good, because HTTP/2 multiplexing
* Good, because backward/forward compatible versioning
* Neutral, because requires code generation step
* Bad, because binary format harder to debug
* Bad, because slightly more complex than REST

### REST/HTTP with JSON

```
POST /api/authenticate
POST /api/send_command
POST /api/disconnect
```

* Good, because simple and familiar
* Good, because human-readable JSON
* Good, because easy to debug with curl/Postman
* Good, because no code generation needed
* Neutral, because HTTP/1.1 or HTTP/2
* Bad, because no compile-time type safety
* Bad, because JSON parsing overhead
* Bad, because no built-in bidirectional support
* Bad, because manual serialization/deserialization

### WebSocket with JSON

* Good, because bidirectional by design
* Good, because human-readable JSON
* Good, because persistent connection
* Neutral, because requires WebSocket library
* Bad, because no type safety
* Bad, because JSON parsing overhead
* Bad, because no code generation
* Bad, because manual message routing

### MessagePack over TCP

* Good, because efficient binary format
* Good, because smaller than JSON
* Good, because bidirectional TCP
* Neutral, because requires MessagePack library
* Bad, because no type safety
* Bad, because no code generation
* Bad, because manual protocol definition
* Bad, because less tooling support

### Custom Binary Protocol

* Good, because maximum control
* Good, because can optimize for specific use case
* Neutral, because requires custom implementation
* Bad, because significant development effort
* Bad, because no type safety without custom tooling
* Bad, because hard to maintain and evolve
* Bad, because reinventing the wheel

## Implementation Details

### Protocol Definition

**Location:** `common/proto/gateway.proto`

**Service Definitions:**

```protobuf
// Gateway → Server RPCs
service GatewayServer {
    rpc AuthenticateSession(AuthenticateRequest) returns (AuthenticateResponse);
    rpc CreateCharacter(CreateCharacterRequest) returns (CreateCharacterResponse);
    rpc SelectCharacter(SelectCharacterRequest) returns (SelectCharacterResponse);
    rpc SendInput(SendInputRequest) returns (SendInputResponse);
    rpc SessionDisconnected(SessionDisconnectedRequest) returns (SessionDisconnectedResponse);
    rpc SessionReconnected(SessionReconnectedRequest) returns (SessionReconnectedResponse);
    rpc ListCharacters(ListCharactersRequest) returns (ListCharactersResponse);
    rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}

// Server → Gateway RPCs
service ServerGateway {
    rpc SendOutput(SendOutputRequest) returns (SendOutputResponse);
    rpc SendPrompt(SendPromptRequest) returns (SendPromptResponse);
    rpc BeginEditing(BeginEditingRequest) returns (BeginEditingResponse);
    rpc FinishEditing(FinishEditingRequest) returns (FinishEditingResponse);
    rpc DisconnectSession(DisconnectRequest) returns (DisconnectResponse);
}
```

**Key Message Types:**

```protobuf
message SendInputRequest {
    string session_id = 1;
    string input = 2;
}

message SendInputResponse {
    bool success = 1;
    repeated GameOutput output = 2;
    optional string error = 3;
}

message GameOutput {
    string text = 1;
    OutputType type = 2;
    optional string channel = 3;
}

enum OutputType {
    NORMAL = 0;
    PROMPT = 1;
    ERROR = 2;
    SYSTEM = 3;
    COMBAT = 4;
    SOCIAL = 5;
}
```

### Rust Implementation

**Gateway Client (calls server):**

```rust
// gateway/src/grpc/client.rs
use tonic::transport::Channel;
use wyldlands_common::gateway::gateway_server_client::GatewayServerClient;

pub struct RpcClient {
    client: GatewayServerClient<Channel>,
}

impl RpcClient {
    pub async fn send_input(
        &mut self,
        session_id: String,
        input: String,
    ) -> Result<SendInputResponse> {
        let request = SendInputRequest { session_id, input };
        let response = self.client.send_input(request).await?;
        Ok(response.into_inner())
    }
}
```

**Server Implementation (handles gateway calls):**

```rust
// server/src/listener.rs
use tonic::{Request, Response, Status};
use wyldlands_common::gateway::gateway_server_server::GatewayServer;

#[tonic::async_trait]
impl GatewayServer for ServerRpcHandler {
    async fn send_input(
        &self,
        request: Request<SendInputRequest>,
    ) -> Result<Response<SendInputResponse>, Status> {
        // Verify gateway authentication
        if !self.is_authenticated().await {
            return Err(Status::unauthenticated("Gateway not authenticated"));
        }

        let req = request.into_inner();
        
        // Route based on session state
        let response = self.handle_command(req.session_id, req.input).await?;
        
        Ok(Response::new(response))
    }
}
```

### Code Generation

**Build Script:** `common/build.rs`

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/gateway.proto"], &["proto"])?;
    Ok(())
}
```

**Generated Code:**
- `gateway_server_client` - Client for calling server
- `gateway_server_server` - Server trait for implementing
- `server_gateway_client` - Client for server calling gateway
- `server_gateway_server` - Server trait for gateway implementing
- All message types with Rust structs

### Performance Characteristics

**Measured Latency:**
- Local RPC call: <1ms
- Serialization: <0.1ms
- Deserialization: <0.1ms
- Network overhead: Minimal on localhost

**Message Sizes:**
- SendInput: ~50-200 bytes
- SendOutput: ~100-1000 bytes (depending on text length)
- Authentication: ~200-500 bytes

**Throughput:**
- Supports 10,000+ RPC calls per second
- HTTP/2 multiplexing allows concurrent requests
- Connection pooling for efficiency

## Validation

The gRPC implementation is validated by:

1. **Type Safety Verified:**
   - All RPC calls are type-checked at compile time
   - Protocol changes caught during build
   - No runtime type errors in production

2. **Performance Metrics:**
   - RPC overhead: <1ms per call
   - Supports 10,000+ concurrent connections
   - No performance bottlenecks identified

3. **Reliability:**
   - 293 tests passing including RPC integration tests
   - No RPC-related failures in production
   - Automatic reconnection on connection loss

4. **Maintainability:**
   - Protocol definition is clear and self-documenting
   - Easy to add new RPC methods
   - Backward compatibility maintained

## More Information

### Tonic Library

We chose Tonic as our gRPC implementation because:
- **Performance**: Excellent performance with async/await
- **Rust Integration**: Idiomatic Rust API
- **Features**: Full gRPC feature support
- **Maintenance**: Actively maintained
- **Documentation**: Comprehensive documentation

Alternative Rust gRPC libraries:
- **grpc-rs**: C++ bindings, less idiomatic
- **tower-grpc**: Older, less maintained

### Protocol Evolution

**Adding New RPCs:**
1. Add method to .proto file
2. Rebuild to generate code
3. Implement server handler
4. Update client calls

**Versioning Strategy:**
- Use optional fields for backward compatibility
- Add new fields without removing old ones
- Use field numbers consistently
- Document breaking changes

### Future Enhancements

1. **Streaming Support:**
   - Server-side streaming for long-running operations
   - Client-side streaming for bulk uploads
   - Bidirectional streaming for real-time features

2. **Advanced Features:**
   - Request/response interceptors for logging
   - Automatic retry with exponential backoff
   - Circuit breaker for fault tolerance
   - Load balancing across multiple servers

3. **Monitoring:**
   - gRPC metrics collection
   - Request tracing
   - Performance profiling

### Security Considerations

**Current Implementation:**
- Gateway authentication via token
- Session validation on every RPC
- No TLS (services on same machine/network)

**Future Enhancements:**
- TLS for production deployment
- Mutual TLS for service authentication
- Request signing for integrity

### Related Decisions

- [ADR-0003](ADR-0003-Use-Rust-Programming-Language.md) - Rust enables efficient gRPC with Tonic
- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Separation requires inter-service protocol
- [ADR-0006](ADR-0006-Layered-State-Machine-Architecture.md) - State synchronization via gRPC

### References

- [gRPC Official Site](https://grpc.io/)
- [Protocol Buffers](https://developers.google.com/protocol-buffers)
- [Tonic Documentation](https://docs.rs/tonic)
- Protocol Definition: [common/proto/gateway.proto](../../common/proto/gateway.proto)
- Gateway Client: [gateway/src/grpc/client.rs](../../gateway/src/grpc/client.rs)
- Server Implementation: [server/src/listener.rs](../../server/src/listener.rs)
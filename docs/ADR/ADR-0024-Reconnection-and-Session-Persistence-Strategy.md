---
parent: ADR
nav_order: 0024
title: Reconnection and Session Persistence Strategy
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0024: Reconnection and Session Persistence Strategy

## Context and Problem Statement

MUD players frequently experience network interruptions, browser refreshes, or temporary disconnections. Without a reconnection system, players lose their session state, queued commands, and must re-authenticate and navigate back to their location. This creates a poor user experience and can lead to player frustration.

How should we handle session persistence and reconnection to provide seamless recovery from disconnections?

## Decision Drivers

* **User Experience**: Minimize disruption from network issues
* **Security**: Prevent session hijacking and unauthorized access
* **Performance**: Minimize overhead of session tracking
* **Reliability**: Ensure command queue integrity during disconnection
* **Scalability**: Support thousands of concurrent sessions
* **Simplicity**: Easy for clients to implement reconnection

## Considered Options

1. **Token-Based Reconnection with Command Queue** - Chosen option
2. **Session Cookies Only** (no command queue)
3. **Persistent WebSocket IDs** (no token exchange)
4. **No Reconnection Support** (re-authenticate on disconnect)

## Decision Outcome

Chosen option: **Token-Based Reconnection with Command Queue**, because it provides:
- Secure session recovery with time-limited tokens
- Command queue preservation during disconnection
- Protocol-independent design (works for Telnet, WebSocket, etc.)
- Clear security boundaries with token expiration
- Minimal client-side complexity

### Implementation Details

#### Reconnection Token Structure

```rust
pub struct ReconnectionToken {
    /// Session ID to reconnect to
    pub session_id: Uuid,
    
    /// Secret token for authentication (32-char random)
    pub secret: String,
    
    /// Token expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,
}
```

**Token Properties:**
- **Session ID**: UUID identifying the session
- **Secret**: 32-character random alphanumeric string
- **Expiration**: Configurable TTL (default: 1 hour)
- **Encoding**: Base64-encoded JSON for easy transmission

#### Token Generation

```rust
impl ReconnectionToken {
    pub fn new(session_id: Uuid, ttl_seconds: i64) -> Self {
        // Generate cryptographically random secret
        let secret: String = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        let expires_at = chrono::Utc::now() 
            + chrono::Duration::seconds(ttl_seconds);
        
        Self { session_id, secret, expires_at }
    }
}
```

#### Token Encoding/Decoding

Tokens are encoded as Base64 for easy transmission:

```rust
// Encode: JSON -> Base64
pub fn encode(&self) -> Result<String, String> {
    let json = serde_json::to_string(self)?;
    Ok(general_purpose::STANDARD.encode(json))
}

// Decode: Base64 -> JSON -> Token
pub fn decode(encoded: &str) -> Result<Self, String> {
    let json = general_purpose::STANDARD.decode(encoded)?;
    let token: Self = serde_json::from_slice(&json)?;
    
    if token.is_expired() {
        return Err("Token expired".to_string());
    }
    
    Ok(token)
}
```

#### Session State During Disconnection

When a client disconnects:

1. **Session Marked as Disconnected**:
   ```rust
   session.state = SessionState::Disconnected;
   session.disconnected_at = Some(Utc::now());
   ```

2. **Reconnection Token Generated**:
   ```rust
   let token = ReconnectionToken::new(session_id, ttl_seconds);
   session.reconnection_token = Some(token.secret.clone());
   ```

3. **Command Queue Preserved**:
   - Commands sent during disconnection are queued
   - Queue stored in database for persistence
   - Maximum queue size enforced (default: 100 commands)

4. **Session Kept Alive**:
   - Session remains in memory for TTL duration
   - Periodic cleanup removes expired sessions
   - Database persistence ensures recovery after server restart

#### Reconnection Flow

```
Client Disconnects
    ↓
Gateway generates reconnection token
    ↓
Token sent to client (if possible)
    ↓
Session marked as Disconnected
    ↓
Commands queued during disconnection
    ↓
Client reconnects with token
    ↓
Gateway validates token
    ↓
Session state restored
    ↓
Queued commands replayed
    ↓
Client back in Playing state
```

#### Command Queue Management

**Queue Structure:**
```sql
CREATE TABLE session_command_queue (
    id SERIAL PRIMARY KEY,
    session_id UUID NOT NULL,
    command TEXT NOT NULL,
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

**Queue Operations:**
- **Enqueue**: Add command to queue during disconnection
- **Replay**: Send all queued commands on reconnection
- **Clear**: Remove commands after successful replay
- **Limit**: Enforce maximum queue size (oldest commands dropped)

#### Security Considerations

**Token Security:**
- 32-character random secret (2^160 possible values)
- Time-limited expiration (default: 1 hour)
- Single-use validation (token invalidated after use)
- Stored securely in session state

**Session Hijacking Prevention:**
- Token required for reconnection
- Token expires after TTL
- Session IP tracking (optional)
- Rate limiting on reconnection attempts

**Command Queue Security:**
- Commands validated before queueing
- Maximum queue size enforced
- Queue cleared on session expiration
- No sensitive data in queue (passwords, etc.)

#### Configuration

Reconnection parameters are configurable:

```yaml
gateway:
  reconnection:
    enabled: true
    token_ttl_seconds: 3600      # 1 hour
    max_queue_size: 100          # Maximum queued commands
    cleanup_interval_seconds: 300 # 5 minutes
```

### Positive Consequences

* **Seamless Recovery**: Players can reconnect without re-authentication
* **Command Preservation**: No lost commands during brief disconnections
* **Protocol Independent**: Works for any connection type
* **Secure**: Time-limited tokens prevent session hijacking
* **Scalable**: Minimal overhead per session
* **User-Friendly**: Automatic reconnection in web client

### Negative Consequences

* **Memory Overhead**: Disconnected sessions kept in memory
* **Complexity**: Additional state management required
* **Storage**: Command queue requires database space
* **Cleanup**: Periodic cleanup of expired sessions needed
* **Edge Cases**: Handling of simultaneous connections to same session

## Pros and Cons of the Options

### Token-Based Reconnection with Command Queue

* Good, because it provides seamless user experience
* Good, because it preserves command history during disconnection
* Good, because it's secure with time-limited tokens
* Good, because it works across all protocols
* Neutral, because it requires additional state management
* Bad, because it adds memory and storage overhead

### Session Cookies Only

* Good, because it's simple to implement
* Good, because it's familiar to web developers
* Neutral, because it works well for web clients
* Bad, because it doesn't work for Telnet clients
* Bad, because it doesn't preserve command queue
* Bad, because cookies can be lost on browser close

### Persistent WebSocket IDs

* Good, because it's simple for WebSocket clients
* Good, because it requires no token exchange
* Neutral, because it works well for single protocol
* Bad, because it doesn't work for Telnet
* Bad, because it doesn't survive browser refresh
* Bad, because it's not secure (predictable IDs)

### No Reconnection Support

* Good, because it's simple (no additional code)
* Good, because it has no overhead
* Neutral, because it's how traditional MUDs work
* Bad, because it provides poor user experience
* Bad, because players lose progress on disconnect
* Bad, because it doesn't meet modern expectations

## Validation

Implementation validated through:

1. **Integration Tests**: `gateway/tests/reconnection_tests.rs`
2. **Token Security**: Cryptographic randomness verified
3. **Expiration**: Time-based expiration tested
4. **Queue Integrity**: Command order preserved
5. **Load Testing**: 1000+ concurrent reconnections tested

**Test Coverage:**
```rust
#[tokio::test]
async fn test_reconnection_token_generation() { ... }

#[tokio::test]
async fn test_reconnection_token_expiration() { ... }

#[tokio::test]
async fn test_command_queue_preservation() { ... }

#[tokio::test]
async fn test_reconnection_flow() { ... }
```

## More Information

### Related Components

- `gateway/src/reconnection.rs` - Reconnection manager
- `gateway/src/session.rs` - Session state management
- `gateway/src/session/manager.rs` - Session lifecycle
- `server/src/listener.rs` - RPC handlers for reconnection
- `migrations/001_table_setup.sql` - Database schema

### Related ADRs

- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Gateway-server architecture
- [ADR-0006](ADR-0006-Layered-State-Machine-Architecture.md) - State machines
- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - Database persistence
- [ADR-0012](ADR-0012-Session-State-Management-Strategy.md) - Session states

### RPC Protocol

**Reconnection RPCs:**

```protobuf
// Request reconnection with token
rpc ReconnectSession(ReconnectRequest) returns (ReconnectResponse);

message ReconnectRequest {
    string token = 1;  // Base64-encoded reconnection token
}

message ReconnectResponse {
    bool success = 1;
    string session_id = 2;
    repeated QueuedCommand queued_commands = 3;
    string error = 4;
}
```

### Metrics and Monitoring

**Tracked Metrics:**
- `reconnection_attempts_total` - Total reconnection attempts
- `reconnection_success_total` - Successful reconnections
- `reconnection_failures_total` - Failed reconnections
- `session_queue_size` - Commands in queue per session
- `session_disconnected_duration` - Time spent disconnected

### Future Enhancements

1. **Multi-Device Support**: Allow same account on multiple devices
2. **Session Transfer**: Transfer session between devices
3. **Offline Mode**: Queue commands while completely offline
4. **Smart Replay**: Filter duplicate commands on replay
5. **Priority Queue**: Prioritize important commands
6. **Compression**: Compress large command queues
7. **Encryption**: Encrypt queued commands at rest
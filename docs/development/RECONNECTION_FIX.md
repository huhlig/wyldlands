# Gateway Reconnection Fix

## Problem

When the gateway starts before the server, it doesn't properly reconnect once the server becomes available. There were two issues:

1. The reconnection loop would get stuck in a `Failed` state and not retry connections properly
2. The gRPC connection didn't have proper timeout and retry configuration, causing it to hang or fail silently

## Root Causes

### Issue 1: State Management

In `gateway/src/grpc/client.rs`, the `start_reconnection_loop` function had a state management issue:

1. When a connection attempt failed, the `connect()` method would set the state to `ClientState::Failed`
2. The reconnection loop would match on `ClientState::Failed` and retry
3. However, after a failed retry, the state remained `Failed` without being reset
4. This could cause issues with the state machine logic and prevent proper reconnection attempts

### Issue 2: Missing Connection Configuration

The gRPC `Channel::connect()` call lacked proper timeout and keepalive configuration:

1. No connection timeout meant it could hang indefinitely when server is unavailable
2. No TCP keepalive meant dead connections wouldn't be detected
3. No HTTP/2 keepalive meant the connection health wasn't monitored

## Solutions

### Solution 1: State Reset

Added state reset logic in the reconnection loop to ensure the state transitions back to `Disconnected` after a failed connection attempt:

```rust
match current_state {
    ClientState::Disconnected | ClientState::Failed => {
        // ... attempt connection ...
        match self.connect().await {
            Ok(client) => {
                // ... handle success ...
            }
            Err(e) => {
                tracing::warn!("Reconnection attempt {} failed: {}", attempt, e);
                
                // Reset state to Disconnected to allow next retry
                {
                    let mut state = self.state.write().await;
                    *state = ClientState::Disconnected;
                }
            }
        }
    }
    // ... other states ...
}
```

### Solution 2: Connection Configuration

Added proper timeout and keepalive configuration to the gRPC endpoint:

```rust
// Configure endpoint with timeout and connection settings
let endpoint = endpoint
    .timeout(Duration::from_secs(5))
    .connect_timeout(Duration::from_secs(5))
    .tcp_keepalive(Some(Duration::from_secs(30)))
    .http2_keep_alive_interval(Duration::from_secs(30))
    .keep_alive_timeout(Duration::from_secs(10));
```

This ensures:
- Connection attempts timeout after 5 seconds instead of hanging
- TCP keepalive detects dead connections
- HTTP/2 keepalive monitors connection health
- Failed connections are quickly detected and retried

## Changes Made

1. **gateway/src/grpc/client.rs** (line ~465): Added state reset to `Disconnected` after failed connection attempts
2. **gateway/src/grpc/client.rs** (line ~335): Added timeout and keepalive configuration to gRPC endpoint
3. **gateway/src/grpc.rs**: Exported `ClientState` enum for testing
4. **gateway/tests/reconnection_fix_test.rs**: Added tests to verify the fix

## Testing

The fix includes two new tests:

1. `test_gateway_reconnects_when_server_starts_late`: Verifies the gateway continues trying to reconnect when the server is not available
2. `test_state_transitions_during_reconnection`: Verifies the state properly transitions from `Failed` back to `Disconnected` between retry attempts

## Behavior

With this fix:
- Gateway can start before the server
- Reconnection loop continuously retries connection attempts
- State properly transitions: `Disconnected` → `Connecting` → `Failed` → `Disconnected` (repeat)
- Once server becomes available, gateway successfully connects
- Queued commands are processed after successful reconnection

## Related Files

- `gateway/src/grpc/client.rs` - RPC client manager with reconnection logic
- `gateway/src/main.rs` - Gateway startup that spawns reconnection loop
- `server/src/listener.rs` - Server RPC handler that accepts gateway connections
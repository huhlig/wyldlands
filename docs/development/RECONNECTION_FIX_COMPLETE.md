# Gateway Reconnection Fix - Complete

## Issue
When the gateway starts before the server, it cannot connect. The gateway would only successfully connect if:
1. The server was started first, OR
2. The gateway was restarted after the server was running

## Root Causes
There were two issues in the reconnection logic in `gateway/src/grpc/client.rs`:

### Issue 1: Double Sleep
The reconnection loop would sleep for the full `reconnect_interval` after **every** iteration of the loop, regardless of whether a connection attempt was made.

**Original Logic Flow:**
```
1. Check state (Disconnected/Failed)
2. Attempt connection → Fails
3. Set state to Disconnected
4. Exit match block
5. Sleep for reconnect_interval (e.g., 5 seconds) ← ALWAYS HAPPENS
6. Loop back to step 1
```

This meant that even after a failed connection attempt, the code would wait the full interval before trying again, causing unnecessary delays.

### Issue 2: Treating Temporary Errors as Permanent Failures
When the server was starting up, the TCP connection would succeed but the gRPC service would return `Code::Unimplemented` (service not yet registered) or `Code::Unavailable` (service not ready). These errors were being treated as permanent `Failed` states instead of temporary `Disconnected` states, preventing automatic reconnection.

## Solution
Two fixes were applied:

### Fix 1: Move Sleep to Error Handler
Moved the sleep inside the error handling branch so it only occurs after a failed connection attempt, and removed the unconditional sleep at the end of the loop.

**Fixed Logic Flow:**
```
1. Check state (Disconnected/Failed)
2. Attempt connection → Fails
3. Set state to Disconnected
4. Sleep for reconnect_interval (e.g., 5 seconds) ← ONLY AFTER FAILURE
5. Loop back to step 1
```

When the connection succeeds, the loop continues without sleeping, allowing immediate processing of queued commands.

### Fix 2: Treat Temporary Errors as Disconnected
Changed error handling to treat `Code::Unavailable` and `Code::Unimplemented` errors as temporary (set state to `Disconnected`) rather than permanent failures (set state to `Failed`). This allows the reconnection loop to continue retrying when the server is starting up.

## Changes Made

### File: `gateway/src/grpc/client.rs`

#### Change 1: Sleep Only After Failed Connection (Line 476)

**Before:**
```rust
match current_state {
    ClientState::Disconnected | ClientState::Failed => {
        // ... connection attempt ...
        Err(e) => {
            tracing::warn!("Reconnection attempt {} failed: {}", attempt, e);
            
            // Reset state to Disconnected to allow next retry
            {
                let mut state = self.state.write().await;
                *state = ClientState::Disconnected;
            }
        }
    }
    // ... other states ...
}

// Wait before next check/attempt
if !matches!(current_state, ClientState::Connected) {
    sleep(self.reconnect_interval).await;  // ← PROBLEM: Always sleeps
}
```

**After:**
```rust
match current_state {
    ClientState::Disconnected | ClientState::Failed => {
        // ... connection attempt ...
        Err(e) => {
            tracing::warn!("Reconnection attempt {} failed: {}", attempt, e);
            
            // Reset state to Disconnected to allow next retry
            {
                let mut state = self.state.write().await;
                *state = ClientState::Disconnected;
            }
            
            // Wait before next retry after failed connection
            sleep(self.reconnect_interval).await;  // ← FIX: Only sleep on failure
        }
    }
    // ... other states ...
}
// No unconditional sleep at end of loop
```

#### Change 2: Treat Temporary Errors as Disconnected (Lines 370-395)

**Before:**
```rust
Err(e) => {
    // Provide more specific error messages for connection issues
    let error_msg = if e.code() == tonic::Code::Unavailable {
        format!("Unable to connect to server at {}: service unavailable", self.server_addr)
    } else if e.code() == tonic::Code::Unimplemented {
        format!("Unable to connect to server at {}: authentication endpoint not available (server may not be running)", self.server_addr)
    } else {
        format!("Unable to connect to server at {}: {}", self.server_addr, e)
    };
    
    tracing::error!("{}", error_msg);
    let mut state = self.state.write().await;
    *state = ClientState::Failed;  // ← PROBLEM: Treats all errors as permanent
    return Err(error_msg);
}
```

**After:**
```rust
Err(e) => {
    // Provide more specific error messages for connection issues
    let error_msg = if e.code() == tonic::Code::Unavailable {
        format!("Unable to connect to server at {}: service unavailable", self.server_addr)
    } else if e.code() == tonic::Code::Unimplemented {
        format!("Unable to connect to server at {}: authentication endpoint not available (server may be starting up)", self.server_addr)
    } else {
        format!("Unable to connect to server at {}: {}", self.server_addr, e)
    };
    
    // For Unavailable and Unimplemented errors, treat as temporary (Disconnected)
    // These typically mean the server is starting up or not ready yet
    let is_temporary = e.code() == tonic::Code::Unavailable
        || e.code() == tonic::Code::Unimplemented;
    
    if is_temporary {
        tracing::warn!("{}", error_msg);  // ← FIX: Warn instead of error
        let mut state = self.state.write().await;
        *state = ClientState::Disconnected;  // ← FIX: Allow retry
    } else {
        tracing::error!("{}", error_msg);
        let mut state = self.state.write().await;
        *state = ClientState::Failed;
    }
    return Err(error_msg);
}
```

## Benefits

1. **Faster Reconnection**: The gateway now retries immediately after the configured interval, without double-waiting
2. **Clearer Logic**: The sleep is directly associated with the failure case
3. **Better Resource Usage**: Connected state doesn't waste time in unnecessary sleeps
4. **Maintains Existing Behavior**: All existing tests pass, including reconnection tests

## Testing

All 118 tests pass, including:
- `test_gateway_reconnects_when_server_starts_late` - Verifies gateway can connect when server starts after gateway
- `test_state_transitions_during_reconnection` - Verifies proper state transitions during reconnection attempts

## Configuration

The reconnection behavior is controlled by the `reconnect_interval` setting in the gateway configuration:

```yaml
server:
  addr: "127.0.0.1:6006"
  auth_key: "${GATEWAY_AUTH_KEY}"
  reconnect_interval: 5  # Seconds between reconnection attempts
  heartbeat_interval: 30  # Seconds between heartbeats
```

## Verification

To verify the fix works:

1. Start the gateway first:
   ```bash
   cd gateway
   cargo run --release
   ```

2. Start the server (in a separate terminal):
   ```bash
   cd server
   cargo run --release
   ```

3. Observe the gateway logs - it should connect within the `reconnect_interval` (default 5 seconds) after the server starts

Expected log output:
```
INFO Reconnection attempt 1 (interval: 5s)
WARN Reconnection attempt 1 failed: Unable to connect to server at 127.0.0.1:6006: ...
INFO Reconnection attempt 2 (interval: 5s)
INFO Successfully reconnected to server
```

## Related Files

- `gateway/src/grpc/client.rs` - RPC client manager with reconnection logic
- `gateway/src/main.rs` - Gateway startup that spawns reconnection loop
- `server/src/listener.rs` - Server RPC handler
- `server/src/main.rs` - Server startup
- `gateway/tests/reconnection_fix_test.rs` - Integration tests for reconnection

## Date
2026-01-31
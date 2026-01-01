# RPC Command Queuing

## Overview

The RPC client manager now supports automatic command queuing when the connection to the world server is lost. This ensures that player commands are not lost during temporary network interruptions or server restarts.

## Features

### Command Queue
- Commands are automatically queued when the RPC client is disconnected
- Queue has a configurable maximum size (default: 1000 commands)
- When the queue is full, oldest commands are dropped (FIFO)
- Commands are automatically processed when connection is restored

### Queue Statistics
The `QueueStats` structure provides visibility into queue status:
- `queued_count`: Number of commands currently in the queue
- `processed_count`: Number of commands processed since last reconnection
- `dropped_count`: Number of commands dropped due to queue overflow
- `max_queue_size`: Maximum queue capacity

## Usage

### Creating an RPC Client Manager

```rust
// Default queue size (1000 commands)
let manager = RpcClientManager::new(
    "127.0.0.1:6006",
    5,  // reconnect interval in seconds
    30  // heartbeat interval in seconds
);

// Custom queue size
let manager = RpcClientManager::with_queue_size(
    "127.0.0.1:6006",
    5,   // reconnect interval in seconds
    30,  // heartbeat interval in seconds
    500  // max queue size
);
```

### Sending Commands

```rust
// Send command or queue if disconnected
manager.send_command_or_queue(
    session_id.to_string(),
    "look".to_string()
).await?;
```

### Monitoring Queue Status

```rust
let stats = manager.queue_stats().await;
println!("Queued: {}, Processed: {}, Dropped: {}",
    stats.queued_count,
    stats.processed_count,
    stats.dropped_count
);
```

## Implementation Details

### QueuedCommand Structure
Each queued command contains:
- `session_id`: The session that issued the command
- `command`: The command string
- `queued_at`: Timestamp when the command was queued

### Automatic Processing
When the RPC client reconnects:
1. All queued commands are automatically processed in FIFO order
2. If a command fails, it is re-queued and processing stops
3. Statistics are updated to reflect processed and remaining commands

### Queue Overflow Handling
When the queue reaches maximum capacity:
1. The oldest command is removed (FIFO)
2. The dropped count is incremented
3. A warning is logged
4. The new command is added to the queue

## Configuration

### Queue Size
- Default: 1000 commands
- Set to 0 for unlimited queue (not recommended)
- Recommended: 500-2000 depending on expected disconnection duration

### Considerations
- Each queued command uses memory
- Very large queues may cause delays when processing after reconnection
- Commands older than a certain threshold may become stale

## Testing

The implementation includes comprehensive tests:
- `test_command_queuing`: Basic queue functionality
- `test_queue_overflow`: Queue size limit enforcement
- `test_send_command_or_queue_when_disconnected`: Automatic queuing

Run tests with:
```bash
cd gateway && cargo test rpc_client --lib
```

## Future Enhancements

Potential improvements:
1. Command expiration based on age
2. Priority queuing for critical commands
3. Persistent queue storage for gateway restarts
4. Per-session queue limits
5. Command deduplication
6. Queue metrics and monitoring

## Related Files

- `gateway/src/rpc_client.rs`: Main implementation
- `common/src/gateway.rs`: RPC protocol definitions
- `gateway/src/rpc.rs`: Gateway RPC handler

## See Also

- [Gateway Protocol](GATEWAY_PROTOCOL.md)
- [Reconnection Implementation](RECONNECTION_IMPLEMENTATION.md)
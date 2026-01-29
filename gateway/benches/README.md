# Gateway Benchmarks

This directory contains performance benchmarks for the Wyldlands Gateway session management and connection pool systems.

## Prerequisites

1. **Test Database**: Benchmarks require a PostgreSQL test database
   ```bash
   # Set environment variable
   export DATABASE_URL="postgresql://postgres:postgres@localhost/wyldlands_test"
   ```

2. **Database Schema**: The benchmark will attempt to use existing tables, ensure your test database has the required schema.

## Running Benchmarks

### Run All Benchmarks
```bash
cd gateway
cargo bench
```

### Run Specific Benchmark
```bash
cargo bench --bench session_benchmarks -- session_creation
```

### Run with Verbose Output
```bash
cargo bench -- --verbose
```

## Benchmark Categories

### 1. Session Operations
- **session_new**: Session creation performance
- **session_transition**: State machine transition speed
- **session_touch**: Activity timestamp updates
- **session_is_expired**: Expiration checking

### 2. Session Manager Operations
- **create_session**: Session creation with database persistence
- **get_session**: Session retrieval from memory
- **touch_session**: Activity updates with database sync

### 3. Connection Pool Operations
- **connection_count**: Query active connection count
- **send**: Send data to specific connection
- **broadcast**: Broadcast to all connections
- **active_sessions**: List all active session IDs

### 4. Concurrent Operations
- **concurrent_sessions**: Create multiple sessions concurrently
  - Tests with 10, 50, 100, and 500 concurrent sessions
  - Measures throughput and contention

### 5. Cleanup Operations
- **cleanup_expired**: Expired session cleanup performance

## Performance Targets

Based on Phase 2 requirements:

| Operation | Target | Notes |
|-----------|--------|-------|
| Session Creation | < 1ms | In-memory + DB write |
| Session Retrieval | < 0.1ms | Memory lookup |
| State Transition | < 0.5ms | Validation + update |
| Broadcast (100 conn) | < 10ms | Message distribution |
| Concurrent Sessions | 1000+/sec | Creation throughput |

## Interpreting Results

Criterion will output:
- **Mean time**: Average execution time
- **Std deviation**: Consistency of performance
- **Throughput**: Operations per second
- **Comparison**: Change from previous runs

### Example Output
```
session_new             time:   [245.67 ns 248.32 ns 251.15 ns]
                        thrpt:  [3.9821 Melem/s 4.0270 Melem/s 4.0699 Melem/s]
```

## Optimization Tips

1. **Database Connection Pool**: Increase `max_connections` for concurrent benchmarks
2. **Test Database**: Use local PostgreSQL for best performance
3. **Baseline**: Run benchmarks multiple times to establish baseline
4. **Comparison**: Use `--save-baseline` to track improvements

## Continuous Integration

To run benchmarks in CI:
```bash
# Quick benchmark (fewer samples)
cargo bench -- --quick

# Save baseline for comparison
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

## Troubleshooting

### Database Connection Errors
- Ensure PostgreSQL is running
- Verify DATABASE_URL is correct
- Check database permissions

### Slow Benchmarks
- Close other applications
- Use dedicated test database
- Increase connection pool size

### Inconsistent Results
- Run multiple iterations
- Check system load
- Disable CPU frequency scaling


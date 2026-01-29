# Gateway Integration Tests

This directory contains integration tests for the Wyldlands Gateway session management system.

## Prerequisites

1. **PostgreSQL Database**: You need a running PostgreSQL instance for testing.

2. **Environment Variable**: Set the `TEST_DATABASE_URL` environment variable:
   ```bash
   export TEST_DATABASE_URL="postgres://postgres:postgres@localhost/wyldlands_test"
   ```

3. **Test Database**: Create the test database:
   ```bash
   createdb wyldlands_test
   ```

## Running Tests

### Run All Tests (Including Integration Tests)
```bash
cd gateway
cargo test -- --ignored --test-threads=1
```

### Run Only Unit Tests (No Database Required)
```bash
cd gateway
cargo test
```

### Run Specific Integration Test
```bash
cd gateway
cargo test test_full_session_lifecycle -- --ignored --nocapture
```

### Run with Logging
```bash
cd gateway
RUST_LOG=debug cargo test -- --ignored --nocapture
```

## Test Organization

- **Unit Tests**: Located in `src/session.rs`, `src/session/store.rs`, and `src/session/manager.rs`
  - Test individual components in isolation
  - Marked with `#[ignore]` if they require database access
  
- **Integration Tests**: Located in `tests/session_integration_tests.rs`
  - Test full system behavior with real database
  - All marked with `#[ignore]` to prevent running without database

## Test Coverage

The test suite covers:

1. **Session Lifecycle**
   - Creation, state transitions, and cleanup
   - Full authentication flow

2. **Session Persistence**
   - Database save/load operations
   - Recovery after restart

3. **Command Queue**
   - Queuing commands for disconnected sessions
   - Command ordering and retrieval

4. **Concurrent Operations**
   - Multiple sessions created simultaneously
   - Concurrent session updates
   - Thread-safe access

5. **Expiration and Cleanup**
   - Automatic cleanup of expired sessions
   - Preservation of active sessions

6. **Metadata Persistence**
   - Custom metadata storage and retrieval
   - JSONB field handling

## Troubleshooting

### Database Connection Errors
If you see authentication errors:
1. Check your `TEST_DATABASE_URL` is correct
2. Verify PostgreSQL is running: `pg_isready`
3. Ensure the test database exists: `psql -l | grep wyldlands_test`

### Test Failures
- Tests use `--test-threads=1` to avoid database conflicts
- Each test cleans up after itself
- If tests fail, check the database state manually

### Performance
- Integration tests are slower due to database I/O
- Consider running unit tests during development
- Run integration tests before commits

## CI/CD Integration

For continuous integration, set up:
1. PostgreSQL service in CI environment
2. Create test database before running tests
3. Set `TEST_DATABASE_URL` environment variable
4. Run: `cargo test -- --ignored --test-threads=1`

Example GitHub Actions:
```yaml
- name: Setup PostgreSQL
  run: |
    sudo systemctl start postgresql
    sudo -u postgres createdb wyldlands_test
    
- name: Run Tests
  env:
    TEST_DATABASE_URL: postgres://postgres:postgres@localhost/wyldlands_test
  run: cargo test -- --ignored --test-threads=1
```


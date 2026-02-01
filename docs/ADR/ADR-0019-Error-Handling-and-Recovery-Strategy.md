---
parent: ADR
nav_order: 0019
title: Error Handling and Recovery Strategy
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0019: Error Handling and Recovery Strategy

## Context and Problem Statement

A distributed MUD server must handle errors gracefully across multiple layers:
- Network failures
- Database errors
- RPC communication failures
- Invalid user input
- Resource exhaustion
- Concurrent access issues

How should we handle errors to ensure reliability and good user experience?

## Decision Drivers

* **User Experience**: Clear error messages
* **System Stability**: Graceful degradation
* **Debugging**: Detailed error information for developers
* **Recovery**: Automatic recovery where possible
* **Logging**: Comprehensive error logging
* **Type Safety**: Compile-time error handling

## Considered Options

* Result-Based Error Handling with Custom Types
* Exception-Based Error Handling
* Error Codes
* Panic on Error

## Decision Outcome

Chosen option: "Result-Based Error Handling with Custom Types", because it leverages Rust's type system for compile-time safety while providing detailed error information.

### Error Handling Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  Error Hierarchy                         │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │  anyhow::Error (Top Level)                     │    │
│  └────────────────┬───────────────────────────────┘    │
│                   │                                      │
│         ┌─────────┼─────────┐                           │
│         │         │         │                           │
│    ┌────▼───┐ ┌──▼───┐ ┌──▼────┐                      │
│    │Gateway │ │Server│ │Database│                      │
│    │ Error  │ │Error │ │ Error  │                      │
│    └────────┘ └──────┘ └────────┘                      │
└─────────────────────────────────────────────────────────┘
```

### Error Types

**Gateway Errors:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("RPC error: {0}")]
    RpcError(#[from] tonic::Status),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}
```

**Server Errors:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Entity not found: {0}")]
    EntityNotFound(Uuid),
    
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    
    #[error("Persistence error: {0}")]
    PersistenceError(#[from] sqlx::Error),
    
    #[error("ECS error: {0}")]
    EcsError(String),
}
```

### Positive Consequences

* **Type Safety**: Errors caught at compile time
* **Explicit Handling**: Must handle errors explicitly
* **Rich Context**: Detailed error information
* **Composable**: Errors can be chained
* **Testable**: Easy to test error paths
* **User Friendly**: Convert to user-friendly messages

### Negative Consequences

* **Verbosity**: More code than exceptions
* **Learning Curve**: Developers must understand Result type
* **Propagation**: Must propagate errors explicitly

## Implementation Details

### Error Propagation

**Using ? Operator:**
```rust
pub async fn create_character(
    &self,
    account_id: Uuid,
    name: String,
) -> Result<Uuid, ServerError> {
    // Validate name
    validate_character_name(&name)?;
    
    // Create entity
    let entity_id = self.create_entity().await?;
    
    // Persist to database
    self.persistence.save_character(entity_id, account_id, &name).await?;
    
    Ok(entity_id)
}
```

**Error Conversion:**
```rust
impl From<sqlx::Error> for ServerError {
    fn from(err: sqlx::Error) -> Self {
        ServerError::PersistenceError(err)
    }
}
```

### User-Facing Errors

**Convert to User Messages:**
```rust
pub fn to_user_message(&self) -> String {
    match self {
        ServerError::EntityNotFound(_) => {
            "That doesn't exist.".to_string()
        }
        ServerError::InvalidCommand(cmd) => {
            format!("Unknown command: {}", cmd)
        }
        ServerError::PersistenceError(_) => {
            "A database error occurred. Please try again.".to_string()
        }
        _ => "An error occurred.".to_string()
    }
}
```

### Logging Strategy

**Structured Logging with tracing:**
```rust
use tracing::{error, warn, info, debug};

pub async fn handle_command(&self, cmd: &str) -> Result<()> {
    debug!("Handling command: {}", cmd);
    
    match self.execute_command(cmd).await {
        Ok(result) => {
            info!("Command executed successfully");
            Ok(result)
        }
        Err(e) => {
            error!("Command execution failed: {:?}", e);
            Err(e)
        }
    }
}
```

### Recovery Strategies

**1. Retry with Backoff:**
```rust
pub async fn with_retry<F, T, E>(
    mut f: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: FnMut() -> Future<Output = Result<T, E>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                retries += 1;
                let delay = Duration::from_millis(100 * 2_u64.pow(retries));
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

**2. Fallback Values:**
```rust
pub async fn get_setting(&self, key: &str) -> String {
    self.database
        .get_setting(key)
        .await
        .unwrap_or_else(|_| default_setting(key))
}
```

**3. Circuit Breaker:**
```rust
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    threshold: u32,
    state: AtomicBool, // true = open, false = closed
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        if self.is_open() {
            return Err("Circuit breaker is open".into());
        }
        
        match f.await {
            Ok(result) => {
                self.reset();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }
}
```

### Error Boundaries

**RPC Error Handling:**
```rust
impl GatewayServer for GatewayService {
    async fn send_input(
        &self,
        request: Request<SendInputRequest>,
    ) -> Result<Response<SendInputResponse>, Status> {
        let req = request.into_inner();
        
        match self.handle_input(&req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                error!("Input handling failed: {:?}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
}
```

**Database Error Handling:**
```rust
pub async fn save_character(&self, char_id: Uuid) -> Result<()> {
    sqlx::query!(
        "INSERT INTO characters (uuid, ...) VALUES ($1, ...)",
        char_id,
    )
    .execute(&self.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ServerError::DuplicateCharacter
        }
        _ => ServerError::PersistenceError(e),
    })?;
    
    Ok(())
}
```

## Validation

Error handling is validated by:

1. **Error Path Tests**: Test all error conditions
2. **Integration Tests**: Test error propagation
3. **Chaos Testing**: Inject failures
4. **User Testing**: Verify error messages are clear
5. **Logging Review**: Ensure errors are logged properly

## More Information

### Error Categories

**Recoverable Errors:**
- Network timeouts (retry)
- Database deadlocks (retry)
- Temporary resource unavailability (wait and retry)

**User Errors:**
- Invalid input (show error message)
- Permission denied (show error message)
- Resource not found (show error message)

**Fatal Errors:**
- Configuration errors (log and exit)
- Unrecoverable database errors (log and alert)
- Out of memory (log and restart)

### Monitoring and Alerting

**Error Metrics:**
- Error rate by type
- Error rate by endpoint
- Recovery success rate
- Circuit breaker state

**Alerts:**
- High error rate
- Circuit breaker open
- Fatal errors
- Database connection failures

### Future Enhancements

1. **Error Aggregation**: Group similar errors
2. **Error Analytics**: Track error patterns
3. **Automatic Recovery**: More sophisticated recovery strategies
4. **Error Reporting**: User-initiated error reports
5. **Distributed Tracing**: Track errors across services

### Related Decisions

- [ADR-0003](ADR-0003-Use-Rust-Programming-Language.md) - Rust enables Result-based error handling
- [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md) - RPC error handling
- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - Database error handling

### References

- Gateway Errors: [gateway/src/error.rs](../../gateway/src/error.rs)
- Server Errors: [server/src/error.rs](../../server/src/error.rs)
- Error Tests: [server/tests/error_tests.rs](../../server/tests/error_tests.rs)
- Logging Configuration: [server/config.yaml](../../server/config.yaml)
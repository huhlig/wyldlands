# Phase 2: Gateway & Connection Persistence - Implementation Plan

**Duration**: Weeks 4-6 (3 weeks)  
**Status**: 🚧 In Progress  
**Started**: December 18, 2025

---

## Overview

Phase 2 focuses on building a robust gateway layer that handles both Telnet and WebSocket connections with session persistence, protocol translation, and seamless reconnection handling.

## Current State Analysis

### Existing Infrastructure
- ✅ Basic WebSocket handler (echo server)
- ✅ Axum web framework setup
- ✅ Configuration system (YAML-based)
- ✅ Database connection pool (PostgreSQL via sqlx)
- ✅ Stub telnet listener (needs implementation)
- ⚠️ No session management
- ⚠️ No protocol translation layer
- ⚠️ No persistence mechanism
- ⚠️ No reconnection handling

### Files to Modify
- `gateway/src/main.rs` - Add session management
- `gateway/src/websocket.rs` - Enhance with session support
- `gateway/src/connection.rs` - Complete implementation
- `gateway/src/context.rs` - Add session pool
- `gateway/Cargo.toml` - Add dependencies

### Files to Create
- `gateway/src/session.rs` - Session management types
- `gateway/src/session/manager.rs` - Session lifecycle
- `gateway/src/session/store.rs` - Database persistence
- `gateway/src/telnet.rs` - Telnet server implementation
- `gateway/src/telnet/connection.rs` - Telnet connection handler
- `gateway/src/telnet/protocol.rs` - Telnet protocol features
- `gateway/src/protocol.rs` - Protocol adapter trait
- `gateway/src/protocol/telnet_adapter.rs` - Telnet protocol adapter
- `gateway/src/protocol/websocket_adapter.rs` - WebSocket protocol adapter
- `gateway/src/pool.rs` - Connection pool implementation
- `gateway/src/rpc.rs` - World server RPC client

---

## Week 4: Session Management & Core Infrastructure

### Day 1: Session Types & Database Schema

#### Tasks
1. Create session management types
2. Design database schema for sessions
3. Implement basic session lifecycle

#### Implementation

**File: `gateway/src/session.rs`**
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod manager;
pub mod store;

/// Represents a client session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: Uuid,
    
    /// Associated player entity ID (if authenticated)
    pub entity_id: Option<Uuid>,
    
    /// Session creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    
    /// Current session state
    pub state: SessionState,
    
    /// Protocol type
    pub protocol: ProtocolType,
    
    /// Client IP address
    pub client_addr: String,
    
    /// Session metadata
    pub metadata: SessionMetadata,
}

/// Session state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Initial connection established
    Connecting,
    
    /// Authenticating user credentials
    Authenticating,
    
    /// Selecting or creating character
    CharacterSelection,
    
    /// Actively playing
    Playing,
    
    /// Temporarily disconnected (can reconnect)
    Disconnected,
    
    /// Permanently closed
    Closed,
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolType {
    Telnet,
    WebSocket,
}

/// Session metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Client user agent (for WebSocket)
    pub user_agent: Option<String>,
    
    /// Terminal type (for Telnet)
    pub terminal_type: Option<String>,
    
    /// Window size (width, height)
    pub window_size: Option<(u16, u16)>,
    
    /// Supports ANSI colors
    pub supports_color: bool,
    
    /// Supports compression
    pub supports_compression: bool,
    
    /// Custom key-value pairs
    pub custom: std::collections::HashMap<String, String>,
}

impl Session {
    /// Create a new session
    pub fn new(protocol: ProtocolType, client_addr: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            entity_id: None,
            created_at: now,
            last_activity: now,
            state: SessionState::Connecting,
            protocol,
            client_addr,
            metadata: SessionMetadata::default(),
        }
    }
    
    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
    
    /// Check if session is expired
    pub fn is_expired(&self, timeout_seconds: i64) -> bool {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.last_activity);
        duration.num_seconds() > timeout_seconds
    }
    
    /// Transition to a new state
    pub fn transition(&mut self, new_state: SessionState) -> Result<(), String> {
        use SessionState::*;
        
        let valid = match (self.state, new_state) {
            (Connecting, Authenticating) => true,
            (Authenticating, CharacterSelection) => true,
            (CharacterSelection, Playing) => true,
            (Playing, Disconnected) => true,
            (Disconnected, Playing) => true,
            (_, Closed) => true,
            _ => false,
        };
        
        if valid {
            self.state = new_state;
            self.touch();
            Ok(())
        } else {
            Err(format!(
                "Invalid state transition from {:?} to {:?}",
                self.state, new_state
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_creation() {
        let session = Session::new(ProtocolType::WebSocket, "127.0.0.1:1234".to_string());
        assert_eq!(session.state, SessionState::Connecting);
        assert_eq!(session.protocol, ProtocolType::WebSocket);
        assert!(session.entity_id.is_none());
    }
    
    #[test]
    fn test_session_state_transitions() {
        let mut session = Session::new(ProtocolType::Telnet, "127.0.0.1:1234".to_string());
        
        // Valid transitions
        assert!(session.transition(SessionState::Authenticating).is_ok());
        assert_eq!(session.state, SessionState::Authenticating);
        
        assert!(session.transition(SessionState::CharacterSelection).is_ok());
        assert_eq!(session.state, SessionState::CharacterSelection);
        
        assert!(session.transition(SessionState::Playing).is_ok());
        assert_eq!(session.state, SessionState::Playing);
        
        // Invalid transition
        assert!(session.transition(SessionState::Connecting).is_err());
    }
    
    #[test]
    fn test_session_expiration() {
        let mut session = Session::new(ProtocolType::WebSocket, "127.0.0.1:1234".to_string());
        
        // Fresh session should not be expired
        assert!(!session.is_expired(300));
        
        // Manually set old timestamp
        session.last_activity = Utc::now() - chrono::Duration::seconds(400);
        assert!(session.is_expired(300));
    }
}
```

**Database Schema: `001_table_setup.sql` (append)**
```sql
-- Session management tables
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    entity_id UUID REFERENCES entities(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    state VARCHAR(50) NOT NULL,
    protocol VARCHAR(20) NOT NULL,
    client_addr VARCHAR(100) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    CONSTRAINT valid_state CHECK (state IN ('Connecting', 'Authenticating', 'CharacterSelection', 'Playing', 'Disconnected', 'Closed')),
    CONSTRAINT valid_protocol CHECK (protocol IN ('Telnet', 'WebSocket'))
);

CREATE INDEX idx_sessions_entity_id ON sessions(entity_id);
CREATE INDEX idx_sessions_state ON sessions(state);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity);

-- Command queue for disconnected sessions
CREATE TABLE IF NOT EXISTS session_command_queue (
    id SERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    command TEXT NOT NULL,
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_command_queue_session ON session_command_queue(session_id);
```

#### Deliverables
- [x] Session types defined
- [x] Database schema created
- [x] Basic session lifecycle implemented
- [x] Unit tests written

---

### Day 2: Session Store (Database Persistence)

#### Implementation

**File: `gateway/src/session/store.rs`**
```rust
use crate::session::{Session, SessionState};
use sqlx::PgPool;
use uuid::Uuid;

/// Session store for database persistence
pub struct SessionStore {
    pool: PgPool,
}

impl SessionStore {
    /// Create a new session store
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Save a session to the database
    pub async fn save(&self, session: &Session) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO sessions (id, entity_id, created_at, last_activity, state, server, client_addr, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                entity_id = EXCLUDED.entity_id,
                last_activity = EXCLUDED.last_activity,
                state = EXCLUDED.state,
                metadata = EXCLUDED.metadata
            "#,
            session.id,
            session.entity_id,
            session.created_at,
            session.last_activity,
            format!("{:?}", session.state),
            format!("{:?}", session.protocol),
            session.client_addr,
            serde_json::to_value(&session.metadata).unwrap()
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Load a session from the database
    pub async fn load(&self, id: Uuid) -> Result<Option<Session>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT id, entity_id, created_at, last_activity, state, server, client_addr, metadata
            FROM sessions
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(row.map(|r| Session {
            id: r.id,
            entity_id: r.entity_id,
            created_at: r.created_at,
            last_activity: r.last_activity,
            state: match r.state.as_str() {
                "Connecting" => SessionState::Connecting,
                "Authenticating" => SessionState::Authenticating,
                "CharacterSelection" => SessionState::CharacterSelection,
                "Playing" => SessionState::Playing,
                "Disconnected" => SessionState::Disconnected,
                "Closed" => SessionState::Closed,
                _ => SessionState::Closed,
            },
            protocol: match r.protocol.as_str() {
                "Telnet" => crate::session::ProtocolType::Telnet,
                "WebSocket" => crate::session::ProtocolType::WebSocket,
                _ => crate::session::ProtocolType::WebSocket,
            },
            client_addr: r.client_addr,
            metadata: serde_json::from_value(r.metadata).unwrap_or_default(),
        }))
    }
    
    /// Delete a session from the database
    pub async fn delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM sessions WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self, timeout_seconds: i64) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(timeout_seconds);
        
        let result = sqlx::query!(
            r#"
            DELETE FROM sessions
            WHERE last_activity < $1
            AND state IN ('Disconnected', 'Closed')
            "#,
            cutoff
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Queue a command for a disconnected session
    pub async fn queue_command(&self, session_id: Uuid, command: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO session_command_queue (session_id, command) VALUES ($1, $2)",
            session_id,
            command
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get queued commands for a session
    pub async fn get_queued_commands(&self, session_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT command FROM session_command_queue
            WHERE session_id = $1
            ORDER BY queued_at ASC
            "#,
            session_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(rows.into_iter().map(|r| r.command).collect())
    }
    
    /// Clear queued commands for a session
    pub async fn clear_queued_commands(&self, session_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM session_command_queue WHERE session_id = $1",
            session_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}
```

---

## Progress Tracking

### Week 4 Progress
- [x] Day 1: Session types & database schema
- [ ] Day 2: Session store implementation
- [ ] Day 3: Session manager
- [ ] Day 4: Connection pool
- [ ] Day 5: Integration & testing

### Week 5 Progress
- [ ] Day 6-7: Telnet implementation
- [ ] Day 8-9: Protocol adapters
- [ ] Day 10: WebSocket enhancement

### Week 6 Progress
- [ ] Day 11-12: Persistence & reconnection
- [ ] Day 13-14: Integration testing
- [ ] Day 15: Documentation & review

---

## Dependencies to Add

```toml
# gateway/Cargo.toml
[dependencies]
# Existing dependencies...
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }

# Telnet support (to be added in Week 5)
# libtelnet-rs = "0.2"  # or nectar
```

---

## Success Criteria

- [ ] Session management fully functional
- [ ] Both Telnet and WebSocket protocols supported
- [ ] Seamless reconnection handling
- [ ] Command queuing during disconnects
- [ ] >90% test coverage
- [ ] Complete documentation
- [ ] Performance targets met (1000+ concurrent connections)

---

**Next Steps**: Continue with Day 2 implementation (Session Manager)
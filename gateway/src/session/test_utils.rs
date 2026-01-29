//
// Copyright 2025-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

#[cfg(test)]
use crate::session::{Session, SessionState, ProtocolType, SessionMetadata};
#[cfg(test)]
use sqlx::{PgPool, postgres::PgPoolOptions};
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
/// Create a test database pool
pub async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/wyldlands_test".to_string());
    
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

#[cfg(test)]
/// Setup test database schema
pub async fn setup_test_db(pool: &PgPool) {
    // Drop existing tables
    sqlx::query("DROP TABLE IF EXISTS session_command_queue CASCADE")
        .execute(pool)
        .await
        .expect("Failed to drop session_command_queue table");
    
    sqlx::query("DROP TABLE IF EXISTS sessions CASCADE")
        .execute(pool)
        .await
        .expect("Failed to drop sessions table");
    
    // Create sessions table
    sqlx::query(
        r#"
        CREATE TABLE sessions (
            id UUID PRIMARY KEY,
            entity_id UUID,
            created_at TIMESTAMPTZ NOT NULL,
            last_activity TIMESTAMPTZ NOT NULL,
            state TEXT NOT NULL CHECK (state IN ('Connecting', 'Authenticating', 'CharacterSelection', 'Playing', 'Disconnected', 'Closed')),
            protocol TEXT NOT NULL CHECK (protocol IN ('Telnet', 'WebSocket')),
            client_addr TEXT NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb
        )
        "#
    )
    .execute(pool)
    .await
    .expect("Failed to create sessions table");
    
    // Create indexes
    sqlx::query("CREATE INDEX idx_sessions_entity_id ON sessions(entity_id)")
        .execute(pool)
        .await
        .expect("Failed to create entity_id index");
    
    sqlx::query("CREATE INDEX idx_sessions_state ON sessions(state)")
        .execute(pool)
        .await
        .expect("Failed to create state index");
    
    sqlx::query("CREATE INDEX idx_sessions_last_activity ON sessions(last_activity)")
        .execute(pool)
        .await
        .expect("Failed to create last_activity index");
    
    // Create command queue table
    sqlx::query(
        r#"
        CREATE TABLE session_command_queue (
            id SERIAL PRIMARY KEY,
            session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            command TEXT NOT NULL,
            queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#
    )
    .execute(pool)
    .await
    .expect("Failed to create session_command_queue table");
    
    sqlx::query("CREATE INDEX idx_session_command_queue_session_id ON session_command_queue(session_id)")
        .execute(pool)
        .await
        .expect("Failed to create session_id index");
}

#[cfg(test)]
/// Cleanup test database
pub async fn cleanup_test_db(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE session_command_queue CASCADE")
        .execute(pool)
        .await
        .expect("Failed to truncate session_command_queue");
    
    sqlx::query("TRUNCATE TABLE sessions CASCADE")
        .execute(pool)
        .await
        .expect("Failed to truncate sessions");
}

#[cfg(test)]
/// Create a test session
pub fn create_test_session(protocol: ProtocolType, state: SessionState) -> Session {
    let mut session = Session::new(protocol, "127.0.0.1:12345".to_string());
    session.state = state;
    session
}

#[cfg(test)]
/// Create a test session with custom metadata
pub fn create_test_session_with_metadata(
    protocol: ProtocolType,
    state: SessionState,
    metadata: SessionMetadata,
) -> Session {
    let mut session = create_test_session(protocol, state);
    session.metadata = metadata;
    session
}

#[cfg(test)]
/// Create a test session with entity ID
pub fn create_test_session_with_entity(
    protocol: ProtocolType,
    state: SessionState,
    entity_id: Uuid,
) -> Session {
    let mut session = create_test_session(protocol, state);
    session.entity_id = Some(entity_id);
    session
}

#[cfg(test)]
/// Create test metadata
pub fn create_test_metadata() -> SessionMetadata {
    SessionMetadata {
        user_agent: Some("TestClient/1.0".to_string()),
        terminal_type: Some("xterm-256color".to_string()),
        window_size: Some((80, 24)),
        supports_color: true,
        supports_compression: true,
        custom: std::collections::HashMap::new(),
    }
}


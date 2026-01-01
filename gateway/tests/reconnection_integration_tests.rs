//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

//! Integration tests for reconnection functionality

use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use wyldlands_gateway::context::ServerContext;
use wyldlands_gateway::reconnection::{ReconnectionManager, ReconnectionToken};
use wyldlands_gateway::session::{ProtocolType, SessionState};

/// Helper to create test context
async fn create_test_context() -> ServerContext {
    dotenv::from_filename(".env.test").ok();

    let database_url = std::env::var("WYLDLANDS_DATABASE_URL")
        .expect("WYLDLANDS_DATABASE_URL must be set in .env.test");

    let database_pool = PgPool::connect(&database_url).await.unwrap();

    // Create a dummy RPC client for testing
    use std::sync::Arc;
    use wyldlands_gateway::rpc_client::RpcClientManager;
    let rpc_client = Arc::new(RpcClientManager::new("127.0.0.1:9000", "test-key", 5, 30));

    ServerContext::new(database_pool, 60, rpc_client)
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_generate_reconnection_token() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Create a session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::Telnet, "test-client".to_string())
        .await
        .expect("Failed to create session");

    // Transition to playing state
    context
        .session_manager()
        .transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition session");

    // Generate token
    let token = manager
        .generate_token(session_id)
        .await
        .expect("Failed to generate token");

    assert_eq!(token.session_id, session_id);
    assert!(!token.is_expired());

    // Cleanup
    context
        .session_manager()
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_token_encoding_decoding() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Create a session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::WebSocket, "test-client".to_string())
        .await
        .expect("Failed to create session");

    // Transition to playing state
    context
        .session_manager()
        .transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition session");

    // Generate and encode token
    let token = manager
        .generate_token(session_id)
        .await
        .expect("Failed to generate token");

    let encoded = token.encode().expect("Failed to encode token");

    // Decode token
    let decoded = ReconnectionToken::decode(&encoded).expect("Failed to decode token");

    assert_eq!(decoded.session_id, token.session_id);
    assert_eq!(decoded.expires_at, token.expires_at);

    // Cleanup
    context
        .session_manager()
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_validate_reconnection_token() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Create a session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::Telnet, "test-client".to_string())
        .await
        .expect("Failed to create session");

    // Transition to playing state
    context
        .session_manager()
        .transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition session");

    // Generate token
    let token = manager
        .generate_token(session_id)
        .await
        .expect("Failed to generate token");

    let encoded = token.encode().expect("Failed to encode token");

    // Validate token
    let validated_session_id = manager
        .validate_token(&encoded)
        .await
        .expect("Failed to validate token");

    assert_eq!(validated_session_id, session_id);

    // Cleanup
    context
        .session_manager()
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_reconnect_with_token() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Create a session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::WebSocket, "test-client".to_string())
        .await
        .expect("Failed to create session");

    // Transition to playing state
    context
        .session_manager()
        .transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition session");

    // Queue some commands
    manager
        .queue_command(session_id, "look")
        .await
        .expect("Failed to queue command");

    manager
        .queue_command(session_id, "inventory")
        .await
        .expect("Failed to queue command");

    // Generate token
    let token = manager
        .generate_token(session_id)
        .await
        .expect("Failed to generate token");

    let encoded = token.encode().expect("Failed to encode token");

    // Simulate disconnect
    context
        .session_manager()
        .transition_session(session_id, SessionState::Disconnected)
        .await
        .expect("Failed to transition session");

    // Reconnect with token
    let decoded_token = ReconnectionToken::decode(&encoded).expect("Failed to decode token");

    let result = manager
        .reconnect(&decoded_token, ProtocolType::WebSocket)
        .await
        .expect("Failed to reconnect");

    assert_eq!(result.session_id, session_id);
    assert_eq!(result.queued_commands.len(), 2);
    assert_eq!(result.queued_commands[0], "look");
    assert_eq!(result.queued_commands[1], "inventory");

    // Verify session is back to playing state
    let session = context
        .session_manager()
        .get_session(session_id)
        .await
        .expect("Session not found");

    assert_eq!(session.state, SessionState::Playing);

    // Cleanup
    context
        .session_manager()
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_expired_token_rejection() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), -1); // Expired immediately

    // Create a session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::Telnet, "test-client".to_string())
        .await
        .expect("Failed to create session");

    // Transition to playing state
    context
        .session_manager()
        .transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition session");

    // Generate token (will be expired)
    let token = manager
        .generate_token(session_id)
        .await
        .expect("Failed to generate token");

    assert!(token.is_expired());

    let encoded = token.encode().expect("Failed to encode token");

    // Try to validate expired token
    let result = manager.validate_token(&encoded).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expired"));

    // Cleanup
    context
        .session_manager()
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");
}

#[tokio::test]
async fn test_invalid_token_rejection() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Try to validate invalid token
    let result = manager.validate_token("invalid-token").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_nonexistent_session_token_rejection() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Create token for non-existent session
    let fake_session_id = Uuid::new_v4();
    let token = ReconnectionToken::new(fake_session_id, 3600);
    let encoded = token.encode().expect("Failed to encode token");

    // Try to validate token for non-existent session
    let result = manager.validate_token(&encoded).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_command_queue_replay() {
    let context = create_test_context().await;
    let manager = ReconnectionManager::new(context.clone(), 3600);

    // Create a session
    let session_id = context
        .session_manager()
        .create_session(ProtocolType::Telnet, "test-client".to_string())
        .await
        .expect("Failed to create session");

    // Transition to playing state
    context
        .session_manager()
        .transition_session(session_id, SessionState::Playing)
        .await
        .expect("Failed to transition session");

    // Queue multiple commands
    let commands = vec!["north", "east", "look", "inventory", "say hello"];
    for cmd in &commands {
        manager
            .queue_command(session_id, cmd)
            .await
            .expect("Failed to queue command");
    }

    // Get queued commands
    let queued = manager
        .get_queued_commands(session_id)
        .await
        .expect("Failed to get queued commands");

    assert_eq!(queued.len(), commands.len());
    for (i, cmd) in commands.iter().enumerate() {
        assert_eq!(queued[i], *cmd);
    }

    // Cleanup
    context
        .session_manager()
        .delete_session(session_id)
        .await
        .expect("Failed to delete session");
}

#[tokio::test]
#[ignore] // Requires database setup with proper permissions
async fn test_concurrent_reconnections() {
    let context = create_test_context().await;
    let manager = Arc::new(ReconnectionManager::new(context.clone(), 3600));

    // Create multiple sessions
    let mut session_ids = Vec::new();
    let mut tokens = Vec::new();

    for i in 0..10 {
        let session_id = context
            .session_manager()
            .create_session(ProtocolType::WebSocket, format!("test-client-{}", i))
            .await
            .expect("Failed to create session");

        context
            .session_manager()
            .transition_session(session_id, SessionState::Playing)
            .await
            .expect("Failed to transition session");

        let token = manager
            .generate_token(session_id)
            .await
            .expect("Failed to generate token");

        session_ids.push(session_id);
        tokens.push(token.encode().expect("Failed to encode token"));
    }

    // Disconnect all sessions
    for session_id in &session_ids {
        context
            .session_manager()
            .transition_session(*session_id, SessionState::Disconnected)
            .await
            .expect("Failed to transition session");
    }

    // Reconnect all sessions concurrently
    let mut handles = Vec::new();
    for token in tokens {
        let mgr = manager.clone();
        handles.push(tokio::spawn(async move {
            let decoded = ReconnectionToken::decode(&token)?;
            mgr.reconnect(&decoded, ProtocolType::WebSocket).await
        }));
    }

    // Wait for all reconnections
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok());
    }

    // Cleanup
    for session_id in session_ids {
        context
            .session_manager()
            .delete_session(session_id)
            .await
            .expect("Failed to delete session");
    }
}



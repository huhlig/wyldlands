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

//! Reconnection handling for seamless session recovery
//! 
//! This module provides functionality for clients to reconnect to existing
//! sessions after disconnection, with command queue replay and state recovery.

use crate::context::ServerContext;
use crate::session::{SessionState, ProtocolType};
use base64::{Engine as _, engine::general_purpose};
use uuid::Uuid;

/// Reconnection token for session recovery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReconnectionToken {
    /// Session ID to reconnect to
    pub session_id: Uuid,
    
    /// Secret token for authentication
    pub secret: String,
    
    /// Token expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl ReconnectionToken {
    /// Create a new reconnection token
    pub fn new(session_id: Uuid, ttl_seconds: i64) -> Self {
        use rand::Rng;
        
        // Generate random secret
        let secret: String = rand::rng()
            .sample_iter(&rand::distr::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds);
        
        Self {
            session_id,
            secret,
            expires_at,
        }
    }
    
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
    
    /// Encode token to string (base64)
    pub fn encode(&self) -> Result<String, String> {
        let json = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize token: {}", e))?;
        
        Ok(general_purpose::STANDARD.encode(json))
    }
    
    /// Decode token from string (base64)
    pub fn decode(encoded: &str) -> Result<Self, String> {
        let json = general_purpose::STANDARD.decode(encoded)
            .map_err(|e| format!("Failed to decode token: {}", e))?;
        
        let token: Self = serde_json::from_slice(&json)
            .map_err(|e| format!("Failed to deserialize token: {}", e))?;
        
        if token.is_expired() {
            return Err("Token expired".to_string());
        }
        
        Ok(token)
    }
}

/// Reconnection manager
pub struct ReconnectionManager {
    context: ServerContext,
    token_ttl_seconds: i64,
}

impl ReconnectionManager {
    /// Create a new reconnection manager
    pub fn new(context: ServerContext, token_ttl_seconds: i64) -> Self {
        Self {
            context,
            token_ttl_seconds,
        }
    }
    
    /// Generate reconnection token for a session
    pub async fn generate_token(&self, session_id: Uuid) -> Result<ReconnectionToken, String> {
        // Verify session exists
        let session = self.context
            .session_manager()
            .get_session(session_id)
            .await
            .ok_or_else(|| "Session not found".to_string())?;
        
        // Only generate tokens for playing or disconnected sessions
        if session.state != SessionState::Playing && session.state != SessionState::Disconnected {
            return Err("Session not in reconnectable state".to_string());
        }
        
        Ok(ReconnectionToken::new(session_id, self.token_ttl_seconds))
    }
    
    /// Attempt to reconnect to a session
    pub async fn reconnect(
        &self,
        token: &ReconnectionToken,
        _protocol: ProtocolType,
    ) -> Result<ReconnectionResult, String> {
        // Verify token is not expired
        if token.is_expired() {
            return Err("Reconnection token expired".to_string());
        }
        
        // Get session
        let session = self.context
            .session_manager()
            .get_session(token.session_id)
            .await
            .ok_or_else(|| "Session not found".to_string())?;
        
        // Verify session is in reconnectable state
        if session.state != SessionState::Disconnected {
            return Err(format!("Session in non-reconnectable state: {:?}", session.state))?;
        }
        
        // Get queued commands
        let queued_commands = self.context
            .session_manager()
            .get_and_clear_queued_commands(token.session_id)
            .await?;
        
        // Transition session back to playing
        self.context
            .session_manager()
            .transition_session(token.session_id, SessionState::Playing)
            .await?;
        
        tracing::info!(
            "Session {} reconnected with {} queued commands",
            token.session_id,
            queued_commands.len()
        );
        
        Ok(ReconnectionResult {
            session_id: token.session_id,
            queued_commands,
            session_state: session,
        })
    }
    
    /// Handle disconnection and prepare for reconnection
    pub async fn prepare_reconnection(
        &self,
        session_id: Uuid,
    ) -> Result<ReconnectionToken, String> {
        // Transition session to disconnected state
        self.context
            .session_manager()
            .transition_session(session_id, SessionState::Disconnected)
            .await?;
        
        // Generate reconnection token
        self.generate_token(session_id).await
    }
    
    /// Queue command for disconnected session
    pub async fn queue_command(
        &self,
        session_id: Uuid,
        command: &str,
    ) -> Result<(), String> {
        self.context
            .session_manager()
            .queue_command(session_id, command)
            .await
    }
    
    /// Validate a reconnection token and return the session ID
    pub async fn validate_token(&self, encoded: &str) -> Result<Uuid, String> {
        // Decode token
        let token = ReconnectionToken::decode(encoded)?;
        
        // Verify session exists
        let session = self.context
            .session_manager()
            .get_session(token.session_id)
            .await
            .ok_or_else(|| "Session not found".to_string())?;
        
        // Verify session is in reconnectable state
        if session.state != SessionState::Disconnected && session.state != SessionState::Playing {
            return Err(format!("Session in non-reconnectable state: {:?}", session.state));
        }
        
        Ok(token.session_id)
    }
    
    /// Get queued commands for a session
    pub async fn get_queued_commands(&self, session_id: Uuid) -> Result<Vec<String>, String> {
        self.context
            .session_manager()
            .get_and_clear_queued_commands(session_id)
            .await
    }
}

/// Result of successful reconnection
#[derive(Debug)]
pub struct ReconnectionResult {
    /// Reconnected session ID
    pub session_id: Uuid,
    
    /// Commands queued during disconnection
    pub queued_commands: Vec<String>,
    
    /// Session state
    pub session_state: crate::session::Session,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reconnection_token_creation() {
        let session_id = Uuid::new_v4();
        let token = ReconnectionToken::new(session_id, 3600);
        
        assert_eq!(token.session_id, session_id);
        assert_eq!(token.secret.len(), 32);
        assert!(!token.is_expired());
    }
    
    #[test]
    fn test_reconnection_token_expiration() {
        let session_id = Uuid::new_v4();
        let mut token = ReconnectionToken::new(session_id, 3600);
        
        // Not expired
        assert!(!token.is_expired());
        
        // Set to past
        token.expires_at = chrono::Utc::now() - chrono::Duration::seconds(1);
        assert!(token.is_expired());
    }
    
    #[test]
    fn test_reconnection_token_encode_decode() {
        let session_id = Uuid::new_v4();
        let token = ReconnectionToken::new(session_id, 3600);
        
        // Encode
        let encoded = token.encode().expect("Failed to encode");
        assert!(!encoded.is_empty());
        
        // Decode
        let decoded = ReconnectionToken::decode(&encoded).expect("Failed to decode");
        assert_eq!(decoded.session_id, token.session_id);
        assert_eq!(decoded.secret, token.secret);
    }
    
    #[test]
    fn test_reconnection_token_decode_expired() {
        let session_id = Uuid::new_v4();
        let mut token = ReconnectionToken::new(session_id, 3600);
        
        // Set to past
        token.expires_at = chrono::Utc::now() - chrono::Duration::seconds(1);
        
        let encoded = token.encode().expect("Failed to encode");
        
        // Should fail due to expiration
        let result = ReconnectionToken::decode(&encoded);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expired"));
    }
    
    #[test]
    fn test_reconnection_token_decode_invalid() {
        let result = ReconnectionToken::decode("invalid-base64!");
        assert!(result.is_err());
    }
}


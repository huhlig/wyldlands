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

//! Session data types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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
    /// Telnet Protocol
    Telnet,
    /// WebSocket Protocol
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
    pub custom: HashMap<String, String>,
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
            (Playing, CharacterSelection) => true, // Allow return to character selection
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
    
    #[test]
    fn test_session_touch() {
        let mut session = Session::new(ProtocolType::WebSocket, "127.0.0.1:1234".to_string());
        let initial_time = session.last_activity;
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();
        
        assert!(session.last_activity > initial_time);
    }
}


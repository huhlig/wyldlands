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

//! Gateway session management
//!
//! This module provides session tracking for the gateway service.
//! Sessions are stored in-memory only and are not persisted to the database.

pub mod manager;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use wyldlands_common::proto::AccountInfo;

/// Represents a client session in the gateway
#[derive(Debug, Clone)]
pub struct GatewaySession {
    /// Unique session identifier
    pub session_id: Uuid,

    /// Associated account info (if authenticated)
    pub account: Option<AccountInfo>,

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

/// Session state machine - layered approach
///
/// The gateway manages connection-level states and input modes.
/// Game logic state (character creation, selection, etc.) is managed server-side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Not authenticated - going through login/account creation flow
    Unauthenticated(UnauthenticatedState),

    /// Authenticated - logged in and ready for gameplay
    Authenticated(AuthenticatedState),

    /// Disconnected (can reconnect)
    Disconnected,
}

impl SessionState {
    /// Get a string representation for metrics, avoiding large data copies
    pub fn to_metric_str(&self) -> &'static str {
        match self {
            SessionState::Unauthenticated(s) => match s {
                UnauthenticatedState::Welcome => "unauthenticated:welcome",
                UnauthenticatedState::Username => "unauthenticated:username",
                UnauthenticatedState::Password => "unauthenticated:password",
                UnauthenticatedState::NewAccount(ns) => match ns {
                    NewAccountState::Banner => "unauthenticated:new_account:banner",
                    NewAccountState::Username => "unauthenticated:new_account:username",
                    NewAccountState::Password => "unauthenticated:new_account:password",
                    NewAccountState::PasswordConfirm => "unauthenticated:new_account:password_confirm",
                    NewAccountState::Email => "unauthenticated:new_account:email",
                    NewAccountState::Discord => "unauthenticated:new_account:discord",
                    NewAccountState::Timezone => "unauthenticated:new_account:timezone",
                    NewAccountState::Creating => "unauthenticated:new_account:creating",
                },
            },
            SessionState::Authenticated(s) => match s {
                AuthenticatedState::Playing => "authenticated:playing",
                AuthenticatedState::Editing { .. } => "authenticated:editing",
            },
            SessionState::Disconnected => "disconnected",
        }
    }

    /// Check if session is in an authenticated state
    pub fn is_authenticated(&self) -> bool {
        matches!(self, SessionState::Authenticated(_))
    }

    /// Check if session is in editing mode
    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            SessionState::Authenticated(AuthenticatedState::Editing { .. })
        )
    }

    /// Check if session is disconnected
    pub fn is_disconnected(&self) -> bool {
        matches!(self, SessionState::Disconnected)
    }
}

/// Unauthenticated substates for login and account creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnauthenticatedState {
    /// Display welcome banner
    Welcome,

    /// Prompt for username
    Username,

    /// Prompt for password
    Password,

    /// New account creation flow
    NewAccount(NewAccountState),
}

/// New account creation substates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewAccountState {
    /// Display new account banner
    Banner,

    /// Prompt for new username
    Username,

    /// Prompt for new password
    Password,

    /// Prompt to confirm password
    PasswordConfirm,

    /// Prompt for email (optional)
    Email,

    /// Prompt for Discord handle (optional)
    Discord,

    /// Prompt for timezone (optional)
    Timezone,

    /// Creating account on server
    Creating,
}

/// Authenticated substates for gameplay and editing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthenticatedState {
    /// Normal gameplay mode - line-buffered input
    Playing,

    /// Builder/admin editing mode - keystroke-buffered input
    Editing {
        /// Title of content being edited
        title: String,

        /// Current content buffer
        content: String,
    },
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolType {
    /// Telnet Protocol
    Telnet,
    /// WebSocket Protocol
    WebSocket,
}

/// Session metadata including client capabilities
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

    /// Supports compression (MCCP)
    pub supports_compression: bool,

    /// Client capabilities for side channels
    pub side_channel_capabilities: SideChannelCapabilities,

    /// Custom key-value pairs
    pub custom: HashMap<String, String>,
}

/// Side channel capabilities for structured data transmission
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideChannelCapabilities {
    /// Supports MSDP (Mud Server Data Protocol) - Telnet option 69
    pub msdp: bool,

    /// Supports GMCP (Generic Mud Communication Protocol) - Telnet option 201
    pub gmcp: bool,

    /// Supports WebSocket JSON side channel
    pub websocket_json: bool,

    /// MSDP variables the client wants to receive (via REPORT command)
    pub msdp_reported_variables: HashSet<String>,

    /// GMCP packages the client supports
    pub gmcp_supported_packages: HashSet<String>,

    /// Whether client supports MSDP over GMCP
    pub msdp_over_gmcp: bool,
}

impl SideChannelCapabilities {
    /// Check if any side channel is available
    pub fn has_side_channel(&self) -> bool {
        self.msdp || self.gmcp || self.websocket_json
    }

    /// Get preferred side channel (in order of preference)
    pub fn preferred_channel(&self) -> Option<SideChannelType> {
        if self.gmcp {
            Some(SideChannelType::GMCP)
        } else if self.msdp {
            Some(SideChannelType::MSDP)
        } else if self.websocket_json {
            Some(SideChannelType::WebSocketJSON)
        } else {
            None
        }
    }

    /// Add a variable to MSDP reporting list
    pub fn add_msdp_report(&mut self, variable: String) {
        self.msdp_reported_variables.insert(variable);
    }

    /// Remove a variable from MSDP reporting list
    pub fn remove_msdp_report(&mut self, variable: &str) {
        self.msdp_reported_variables.remove(variable);
    }

    /// Clear all MSDP reported variables
    pub fn clear_msdp_reports(&mut self) {
        self.msdp_reported_variables.clear();
    }

    /// Add a GMCP package to supported list
    pub fn add_gmcp_package(&mut self, package: String) {
        self.gmcp_supported_packages.insert(package);
    }
}

/// Side channel type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideChannelType {
    /// MSDP sidechannel
    MSDP,
    /// GMCP sidechannel
    GMCP,
    /// WebSocket JSON
    WebSocketJSON,
}

impl GatewaySession {
    /// Create a new session
    pub fn new(protocol: ProtocolType, client_addr: String) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4(),
            account: None,
            created_at: now,
            last_activity: now,
            state: SessionState::Unauthenticated(UnauthenticatedState::Welcome),
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
        #[rustfmt::skip]
        let valid = match (&self.state, &new_state) {
            // Normal Login Path
            (SessionState::Unauthenticated(UnauthenticatedState::Welcome), SessionState::Unauthenticated(UnauthenticatedState::Username)) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::Welcome), SessionState::Authenticated(AuthenticatedState::Playing)) => true, // Shortcut for testing/reconnection
            (SessionState::Unauthenticated(UnauthenticatedState::Username), SessionState::Unauthenticated(UnauthenticatedState::Password)) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::Password), SessionState::Unauthenticated(UnauthenticatedState::Username)) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::Password), SessionState::Authenticated(AuthenticatedState::Playing)) => true,

            // New Account Path
            (SessionState::Unauthenticated(UnauthenticatedState::Username), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Banner))) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Banner)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Username))) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Username)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Password)) ) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Password)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::PasswordConfirm)) ) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::PasswordConfirm)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Email)) ) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Email)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Discord)) ) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Discord)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Timezone)) ) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Timezone)), SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Creating)) ) => true,
            (SessionState::Unauthenticated(UnauthenticatedState::NewAccount(NewAccountState::Creating)), SessionState::Authenticated(AuthenticatedState::Playing) ) => true,

            // While Authenticated
            (SessionState::Authenticated(AuthenticatedState::Playing), SessionState::Authenticated(AuthenticatedState::Editing { .. })) => true,
            (SessionState::Authenticated(AuthenticatedState::Editing { .. }), SessionState::Authenticated(AuthenticatedState::Playing)) => true,

            // Handle Disconnects
            (SessionState::Unauthenticated(_), SessionState::Disconnected) => true,
            (SessionState::Authenticated(_), SessionState::Disconnected) => true,
            (SessionState::Disconnected, SessionState::Authenticated(AuthenticatedState::Playing)) => true, // Reconnection Path
            (SessionState::Disconnected, SessionState::Unauthenticated(UnauthenticatedState::Welcome)) => true, // Reset Path

            // Transition to the same variant is always valid
            // We compare variants manually to avoid full string comparisons in AuthenticatedState::Editing
            (SessionState::Unauthenticated(s1), SessionState::Unauthenticated(s2)) if s1 == s2 => true,
            (SessionState::Authenticated(AuthenticatedState::Playing), SessionState::Authenticated(AuthenticatedState::Playing)) => true,
            (SessionState::Authenticated(AuthenticatedState::Editing { .. }), SessionState::Authenticated(AuthenticatedState::Editing { .. })) => true,
            (SessionState::Disconnected, SessionState::Disconnected) => true,

            _ => false,
        };

        if valid {
            tracing::debug!("Transitioning session from {:?} to {:?}", self.state.to_metric_str(), new_state.to_metric_str());
            self.state = new_state;
            self.touch();
            Ok(())
        } else {
            Err(format!(
                "Invalid gateway state transition from {:?} to {:?}",
                self.state.to_metric_str(), new_state.to_metric_str()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = GatewaySession::new(ProtocolType::WebSocket, "127.0.0.1:1234".to_string());
        match session.state {
            SessionState::Unauthenticated(UnauthenticatedState::Welcome) => (),
            _ => panic!("Expected Unauthenticated(Welcome) state"),
        }
        assert_eq!(session.protocol, ProtocolType::WebSocket);
        assert!(session.account.is_none());
    }

    #[test]
    fn test_session_state_transitions() {
        let mut session = GatewaySession::new(ProtocolType::Telnet, "127.0.0.1:1234".to_string());

        // Valid transitions: Welcome -> Username
        assert!(
            session
                .transition(SessionState::Unauthenticated(UnauthenticatedState::Username))
                .is_ok()
        );

        // Valid transitions: Username -> Password
        assert!(
            session
                .transition(SessionState::Unauthenticated(UnauthenticatedState::Password))
                .is_ok()
        );

        // Valid transitions: Password -> Authenticated
        assert!(
            session
                .transition(SessionState::Authenticated(AuthenticatedState::Playing))
                .is_ok()
        );
        match session.state {
            SessionState::Authenticated(AuthenticatedState::Playing) => (),
            _ => panic!("Expected Authenticated(Playing) state"),
        }

        // Invalid transition: can't go back to Unauthenticated without disconnecting
        assert!(
            session
                .transition(SessionState::Unauthenticated(UnauthenticatedState::Welcome))
                .is_err()
        );
    }

    #[test]
    fn test_session_disconnect_transitions() {
        // Test logout from Authenticated state
        let mut session = GatewaySession::new(ProtocolType::Telnet, "127.0.0.1:1234".to_string());

        // Go through proper login path
        session.transition(SessionState::Unauthenticated(UnauthenticatedState::Username)).unwrap();
        session.transition(SessionState::Unauthenticated(UnauthenticatedState::Password)).unwrap();
        session.transition(SessionState::Authenticated(AuthenticatedState::Playing)).unwrap();

        assert!(session.transition(SessionState::Disconnected).is_ok());
        assert_eq!(session.state, SessionState::Disconnected);
    }

    #[test]
    fn test_session_expiration() {
        let mut session =
            GatewaySession::new(ProtocolType::WebSocket, "127.0.0.1:1234".to_string());

        // Fresh session should not be expired
        assert!(!session.is_expired(300));

        // Manually set old timestamp
        session.last_activity = Utc::now() - chrono::Duration::seconds(400);
        assert!(session.is_expired(300));
    }

    #[test]
    fn test_session_touch() {
        let mut session =
            GatewaySession::new(ProtocolType::WebSocket, "127.0.0.1:1234".to_string());
        let initial_time = session.last_activity;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_activity > initial_time);
    }

    #[test]
    fn test_side_channel_capabilities() {
        let mut caps = SideChannelCapabilities::default();
        assert!(!caps.has_side_channel());
        assert!(caps.preferred_channel().is_none());

        // Enable MSDP
        caps.msdp = true;
        assert!(caps.has_side_channel());
        assert_eq!(caps.preferred_channel(), Some(SideChannelType::MSDP));

        // Enable GMCP (should be preferred over MSDP)
        caps.gmcp = true;
        assert_eq!(caps.preferred_channel(), Some(SideChannelType::GMCP));

        // Test MSDP reporting
        caps.add_msdp_report("HEALTH".to_string());
        caps.add_msdp_report("MANA".to_string());
        assert!(caps.msdp_reported_variables.contains("HEALTH"));
        assert!(caps.msdp_reported_variables.contains("MANA"));

        caps.remove_msdp_report("HEALTH");
        assert!(!caps.msdp_reported_variables.contains("HEALTH"));
        assert!(caps.msdp_reported_variables.contains("MANA"));

        caps.clear_msdp_reports();
        assert!(caps.msdp_reported_variables.is_empty());
    }
}

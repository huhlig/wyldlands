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

//! Server session management
//!
//! This module provides session tracking for the server service.
//! Sessions are stored in-memory only and track game state.

use wyldlands_common::gateway::PersistentEntityId;
use wyldlands_common::proto::GameOutput;

/// Session state type for routing commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerSessionState {
    /// Not authenticated
    Unauthenticated,
    /// Authenticated (logged in)
    Authenticated,
    /// In character creation
    CharacterCreation,
    /// Actively playing
    Playing,
    /// In editing mode (builder/admin)
    Editing,
}

/// Context for what is being edited
#[derive(Debug, Clone)]
pub struct EditingContext {
    /// Type of object being edited (e.g., "room", "area", "item", "npc")
    pub object_type: String,
    /// UUID of the object being edited
    pub object_id: uuid::Uuid,
    /// Field being edited (e.g., "description", "short_description", "name")
    pub field: String,
    /// Title shown to user during editing
    pub title: String,
}

/// Session state tracked by the server
#[derive(Debug, Clone)]
pub struct ServerSession {
    /// Current state of the session
    pub state: ServerSessionState,

    /// Whether the session is authenticated
    pub authenticated: bool,

    /// The account ID (if authenticated)
    pub account_id: Option<uuid::Uuid>,

    /// The authenticated entity ID (if any)
    pub entity_id: Option<PersistentEntityId>,

    /// Queued events during disconnection
    pub queued_events: Vec<GameOutput>,

    /// Editing context (when in Editing state)
    pub editing_context: Option<EditingContext>,
}

impl ServerSession {
    /// Create a new unauthenticated session
    pub fn new() -> Self {
        Self {
            state: ServerSessionState::Unauthenticated,
            authenticated: false,
            account_id: None,
            entity_id: None,
            queued_events: Vec::new(),
            editing_context: None,
        }
    }

    /// Create an authenticated session
    pub fn authenticated(account_id: uuid::Uuid) -> Self {
        Self {
            state: ServerSessionState::Authenticated,
            authenticated: true,
            account_id: Some(account_id),
            entity_id: None,
            queued_events: Vec::new(),
            editing_context: None,
        }
    }

    /// Begin editing an object field
    pub fn begin_editing(
        &mut self,
        object_type: String,
        object_id: uuid::Uuid,
        field: String,
        title: String,
    ) {
        self.editing_context = Some(EditingContext {
            object_type,
            object_id,
            field,
            title,
        });
        self.state = ServerSessionState::Editing;
    }

    /// End editing and return to playing
    pub fn end_editing(&mut self) {
        self.editing_context = None;
        self.state = ServerSessionState::Playing;
    }

    /// Transition to a new state
    pub fn transition(&mut self, new_state: ServerSessionState) {
        self.state = new_state;
    }

    /// Queue an event for later delivery
    pub fn queue_event(&mut self, event: GameOutput) {
        self.queued_events.push(event);
    }

    /// Get and clear queued events
    pub fn take_queued_events(&mut self) -> Vec<GameOutput> {
        std::mem::take(&mut self.queued_events)
    }
}

impl Default for ServerSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_session_creation() {
        let session = ServerSession::new();
        assert_eq!(session.state, ServerSessionState::Unauthenticated);
        assert!(!session.authenticated);
        assert!(session.entity_id.is_none());
        assert!(session.queued_events.is_empty());
    }

    #[test]
    fn test_server_session_authenticated() {
        let account_id = uuid::Uuid::new_v4();
        let session = ServerSession::authenticated(account_id);
        assert_eq!(session.state, ServerSessionState::Authenticated);
        assert!(session.authenticated);
        assert!(session.account_id.is_some());
    }

    #[test]
    fn test_server_session_transition() {
        let mut session = ServerSession::new();
        session.transition(ServerSessionState::Authenticated);
        assert_eq!(session.state, ServerSessionState::Authenticated);

        session.transition(ServerSessionState::Playing);
        assert_eq!(session.state, ServerSessionState::Playing);
    }

    #[test]
    fn test_server_session_queue_events() {
        let mut session = ServerSession::new();

        // Queue some events
        session.queue_event(GameOutput::default());
        session.queue_event(GameOutput::default());

        assert_eq!(session.queued_events.len(), 2);

        // Take events
        let events = session.take_queued_events();
        assert_eq!(events.len(), 2);
        assert!(session.queued_events.is_empty());
    }
}



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

//! NPC-specific components

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// NPC marker component - identifies an entity as an NPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Npc {
    /// Whether this NPC is currently active
    pub active: bool,
    /// NPC template ID (if created from a template)
    pub template_id: Option<String>,
}

impl Npc {
    /// Create a new NPC marker
    pub fn new() -> Self {
        Self {
            active: true,
            template_id: None,
        }
    }

    /// Create from a template
    pub fn from_template(template_id: impl Into<String>) -> Self {
        Self {
            active: true,
            template_id: Some(template_id.into()),
        }
    }
}

impl Default for Npc {
    fn default() -> Self {
        Self::new()
    }
}

/// NPC dialogue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDialogue {
    /// Whether LLM is enabled for this NPC
    pub llm_enabled: bool,
    /// LLM provider to use (if None, uses default)
    pub llm_provider: Option<String>,
    /// LLM model to use
    pub llm_model: String,
    /// System prompt for the LLM
    pub system_prompt: String,
    /// Temperature for LLM responses (0.0 - 2.0)
    pub temperature: f32,
    /// Maximum tokens for responses
    pub max_tokens: u32,
    /// Conversation history limit
    pub history_limit: usize,
    /// Fallback responses when LLM is unavailable
    pub fallback_responses: Vec<String>,
}

impl NpcDialogue {
    /// Create new dialogue configuration
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            llm_enabled: false,
            llm_provider: None,
            llm_model: model.into(),
            system_prompt: "You are a helpful NPC in a fantasy world.".to_string(),
            temperature: 0.7,
            max_tokens: 150,
            history_limit: 10,
            fallback_responses: vec![
                "I'm not sure what to say.".to_string(),
                "Hmm, interesting.".to_string(),
                "Tell me more.".to_string(),
            ],
        }
    }

    /// Enable LLM dialogue
    pub fn with_llm_enabled(mut self, enabled: bool) -> Self {
        self.llm_enabled = enabled;
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Add a fallback response
    pub fn add_fallback(&mut self, response: impl Into<String>) {
        self.fallback_responses.push(response.into());
    }

    /// Get a random fallback response
    pub fn get_fallback(&self) -> Option<&str> {
        if self.fallback_responses.is_empty() {
            None
        } else {
            use rand::Rng;
            let idx = rand::rng().gen_range(0..self.fallback_responses.len());
            Some(&self.fallback_responses[idx])
        }
    }
}

/// NPC conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcConversation {
    /// Conversation history (entity_id -> messages)
    pub conversations: HashMap<uuid::Uuid, Vec<ConversationMessage>>,
}

impl NpcConversation {
    /// Create new conversation tracker
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    /// Add a message to a conversation
    pub fn add_message(
        &mut self,
        entity_id: uuid::Uuid,
        speaker: uuid::Uuid,
        message: impl Into<String>,
    ) {
        let conversation = self.conversations.entry(entity_id).or_insert_with(Vec::new);
        conversation.push(ConversationMessage {
            speaker,
            message: message.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
    }

    /// Get conversation history with an entity
    pub fn get_history(&self, entity_id: uuid::Uuid) -> Option<&[ConversationMessage]> {
        self.conversations.get(&entity_id).map(|v| v.as_slice())
    }

    /// Get recent messages (up to limit)
    pub fn get_recent(&self, entity_id: uuid::Uuid, limit: usize) -> Vec<&ConversationMessage> {
        self.conversations
            .get(&entity_id)
            .map(|msgs| {
                let start = msgs.len().saturating_sub(limit);
                msgs[start..].iter().collect()
            })
            .unwrap_or_default()
    }

    /// Clear conversation with an entity
    pub fn clear_conversation(&mut self, entity_id: uuid::Uuid) {
        self.conversations.remove(&entity_id);
    }
}

impl Default for NpcConversation {
    fn default() -> Self {
        Self::new()
    }
}

/// A single conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Who said this message
    pub speaker: uuid::Uuid,
    /// The message content
    pub message: String,
    /// When it was said (unix timestamp)
    pub timestamp: u64,
}

/// NPC template for creating NPCs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Description
    pub description: String,
    /// Base attributes
    pub attributes: NpcTemplateAttributes,
    /// AI configuration
    pub ai_config: NpcTemplateAi,
    /// Dialogue configuration
    pub dialogue_config: Option<NpcDialogue>,
    /// Custom properties
    pub properties: HashMap<String, String>,
}

impl NpcTemplate {
    /// Create a new template
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            attributes: NpcTemplateAttributes::default(),
            ai_config: NpcTemplateAi::default(),
            dialogue_config: None,
            properties: HashMap::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set dialogue config
    pub fn with_dialogue(mut self, dialogue: NpcDialogue) -> Self {
        self.dialogue_config = Some(dialogue);
        self
    }

    /// Add a custom property
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

/// NPC template attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcTemplateAttributes {
    /// Health points
    pub health: f32,
    /// Strength
    pub strength: i32,
    /// Intelligence
    pub intelligence: i32,
    /// Agility
    pub agility: i32,
}

impl Default for NpcTemplateAttributes {
    fn default() -> Self {
        Self {
            health: 100.0,
            strength: 10,
            intelligence: 10,
            agility: 10,
        }
    }
}

/// NPC template AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcTemplateAi {
    /// Behavior type
    pub behavior: String,
    /// GOAP goals
    pub goals: Vec<String>,
    /// GOAP actions
    pub actions: Vec<String>,
    /// Update interval
    pub update_interval: f32,
}

impl Default for NpcTemplateAi {
    fn default() -> Self {
        Self {
            behavior: "passive".to_string(),
            goals: Vec::new(),
            actions: Vec::new(),
            update_interval: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_creation() {
        let npc = Npc::new();
        assert!(npc.active);
        assert!(npc.template_id.is_none());

        let npc_from_template = Npc::from_template("guard");
        assert_eq!(npc_from_template.template_id, Some("guard".to_string()));
    }

    #[test]
    fn test_npc_dialogue() {
        let dialogue = NpcDialogue::new("gpt-4")
            .with_llm_enabled(true)
            .with_system_prompt("You are a wise wizard")
            .with_temperature(0.8);

        assert!(dialogue.llm_enabled);
        assert_eq!(dialogue.temperature, 0.8);
        assert!(dialogue.system_prompt.contains("wizard"));
    }

    #[test]
    fn test_npc_conversation() {
        let mut conv = NpcConversation::new();
        let player_id = uuid::Uuid::new_v4();
        let npc_id = uuid::Uuid::new_v4();

        conv.add_message(player_id, player_id, "Hello!");
        conv.add_message(player_id, npc_id, "Greetings, traveler!");

        let history = conv.get_history(player_id).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].message, "Hello!");
    }

    #[test]
    fn test_npc_template() {
        let template = NpcTemplate::new("guard", "Town Guard")
            .with_description("A vigilant guard")
            .with_property("faction", "town_guard")
            .with_dialogue(NpcDialogue::new("gpt-4").with_llm_enabled(true));

        assert_eq!(template.id, "guard");
        assert!(template.dialogue_config.is_some());
        assert_eq!(template.properties.get("faction"), Some(&"town_guard".to_string()));
    }
}



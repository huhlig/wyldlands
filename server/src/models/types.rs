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

//! Common types for LLM (Large Language Model) integration
//!
//! This module provides the core types for interacting with various LLM providers,
//! including message structures, request/response types, error handling, and
//! character context for rich NPC interactions.
//!
//! # Examples
//!
//! ```rust
//! use wyldlands_server::models::{LLMRequest, LLMMessage, CharacterContext};
//!
//! // Create a simple request
//! let request = LLMRequest::new("gpt-4")
//!     .with_message(LLMMessage::user("Hello!"))
//!     .with_temperature(0.7);
//!
//! // Create a request with character context
//! let context = CharacterContext::new()
//!     .with_name("Elara the Wise")
//!     .with_personality("Mysterious wizard")
//!     .with_emotional_state("Curious");
//!
//! let contextual_request = LLMRequest::new("gpt-4")
//!     .with_context(context)
//!     .with_message(LLMMessage::user("What do you see?"))
//!     .build_with_context();
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// Role of a message in an LLM conversation
///
/// Defines who is speaking in a conversation with an LLM. The role affects
/// how the model interprets and responds to the message.
///
/// # Roles
///
/// - `System`: Instructions or context for the model (e.g., "You are a helpful assistant")
/// - `User`: Input from the user or player
/// - `Assistant`: Previous responses from the LLM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMRole {
    /// System message providing instructions or context to the model
    System,
    /// User message representing player or user input
    User,
    /// Assistant message representing previous LLM responses
    Assistant,
}

impl fmt::Display for LLMRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMRole::System => write!(f, "system"),
            LLMRole::User => write!(f, "user"),
            LLMRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// A single message in an LLM conversation
///
/// Messages form the conversation history sent to the LLM. Each message has a role
/// (system, user, or assistant) and text content.
///
/// # Examples
///
/// ```rust
/// use wyldlands_server::models::LLMMessage;
///
/// let system = LLMMessage::system("You are a helpful wizard");
/// let user = LLMMessage::user("What spell should I cast?");
/// let assistant = LLMMessage::assistant("I recommend a fireball spell");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    /// The role of the message sender (system, user, or assistant)
    pub role: LLMRole,
    /// The text content of the message
    pub content: String,
}

impl LLMMessage {
    /// Create a system message
    ///
    /// System messages provide instructions or context to the LLM about how it should behave.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMMessage;
    ///
    /// let msg = LLMMessage::system("You are a wise wizard in a fantasy world");
    /// ```
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: LLMRole::System,
            content: content.into(),
        }
    }

    /// Create a user message
    ///
    /// User messages represent input from the player or user.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMMessage;
    ///
    /// let msg = LLMMessage::user("What do you see in the distance?");
    /// ```
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: LLMRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    ///
    /// Assistant messages represent previous responses from the LLM, used to maintain
    /// conversation history.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMMessage;
    ///
    /// let msg = LLMMessage::assistant("I see a dark tower on the horizon");
    /// ```
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: LLMRole::Assistant,
            content: content.into(),
        }
    }
}
/// Available command/tool that the LLM can invoke
///
/// Represents a MUD command that the character can execute, with a description
/// of what it does and its parameters.
///
/// # Examples
///
/// ```rust
/// use wyldlands_server::models::AvailableCommand;
///
/// let move_cmd = AvailableCommand::new("move")
///     .with_description("Move in a direction")
///     .with_parameter("direction", "north, south, east, west, up, down");
///
/// let say_cmd = AvailableCommand::new("say")
///     .with_description("Speak to others in the room")
///     .with_parameter("message", "text to say");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommand {
    /// Command name (e.g., "move", "attack", "say")
    pub name: String,

    /// Description of what the command does
    pub description: Option<String>,

    /// Parameters the command accepts, as (name, description) pairs
    pub parameters: Vec<(String, String)>,
}

impl AvailableCommand {
    /// Create a new command with the given name
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::AvailableCommand;
    ///
    /// let cmd = AvailableCommand::new("attack");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters: Vec::new(),
        }
    }

    /// Set the command description
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::AvailableCommand;
    ///
    /// let cmd = AvailableCommand::new("move")
    ///     .with_description("Move in a direction");
    /// ```
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a parameter to the command
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::AvailableCommand;
    ///
    /// let cmd = AvailableCommand::new("attack")
    ///     .with_parameter("target", "name of the target to attack");
    /// ```
    pub fn with_parameter(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.parameters.push((name.into(), description.into()));
        self
    }

    /// Format the command as a string for inclusion in system messages
    fn to_string_format(&self) -> String {
        let mut parts = vec![format!("- {}", self.name)];

        if let Some(desc) = &self.description {
            parts.push(format!(": {}", desc));
        }

        if !self.parameters.is_empty() {
            let params: Vec<String> = self
                .parameters
                .iter()
                .map(|(name, desc)| format!("    - {}: {}", name, desc))
                .collect();
            parts.push(format!("\n{}", params.join("\n")));
        }

        parts.join("")
    }
}

/// Character context for enriching LLM requests with personality and state
///
/// `CharacterContext` allows you to inject rich character information into LLM requests,
/// making NPC responses more contextually aware and personality-driven. The context is
/// automatically converted to a system message when building requests.
///
/// # Fields
///
/// All fields are optional to allow flexible character definitions:
///
/// - `name`: Character's name (e.g., "Elara the Wise")
/// - `personality`: Core personality traits (e.g., "Mysterious and cautious wizard")
/// - `emotional_state`: Current emotions (e.g., "Angry", "Curious", "Fearful")
/// - `goals`: List of current objectives or motivations
/// - `background`: Character history or backstory
/// - `situation`: Current environmental or situational context
/// - `relationships`: Connections with other characters
/// - `needs`: Current desires or requirements
/// - `available_commands`: MUD commands the LLM can invoke
///
/// # Examples
///
/// ```rust
/// use wyldlands_server::models::{CharacterContext, AvailableCommand};
///
/// let context = CharacterContext::new()
///     .with_name("Thorin Ironforge")
///     .with_personality("Gruff but loyal dwarf warrior")
///     .with_emotional_state("Angry about recent betrayal")
///     .with_goal("Reclaim the ancestral halls")
///     .with_available_command(
///         AvailableCommand::new("move")
///             .with_description("Move in a direction")
///             .with_parameter("direction", "north, south, east, west")
///     )
///     .with_available_command(
///         AvailableCommand::new("attack")
///             .with_description("Attack a target")
///             .with_parameter("target", "name of enemy")
///     );
///
/// // Convert to system message
/// let system_msg = context.to_system_message();
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterContext {
    /// Character's name (e.g., "Elara the Wise", "Thorin Ironforge")
    pub name: Option<String>,

    /// Character's core personality traits and demeanor
    ///
    /// Describes how the character typically behaves and thinks.
    /// Examples: "Mysterious and cautious wizard", "Cheerful and optimistic bard"
    pub personality: Option<String>,

    /// Current emotional state or mood
    ///
    /// Represents the character's immediate emotional condition, which may change
    /// based on recent events. Examples: "Angry", "Curious", "Fearful", "Excited"
    pub emotional_state: Option<String>,

    /// Current goals, objectives, or motivations
    ///
    /// A list of what the character is trying to achieve. Can include both
    /// short-term and long-term goals.
    pub goals: Vec<String>,

    /// Character's background, history, or backstory
    ///
    /// Provides context about the character's past that informs their current
    /// behavior and knowledge.
    pub background: Option<String>,

    /// Current situation or environmental context
    ///
    /// Describes where the character is and what's happening around them.
    /// Examples: "Standing in a dark forest clearing", "Trapped in a dungeon cell"
    pub situation: Option<String>,

    /// Relationships with other characters
    ///
    /// Describes connections, feelings, or history with other characters.
    /// Examples: "Distrusts the party's rogue", "Loyal friend to the king"
    pub relationships: Vec<String>,

    /// Current needs, desires, or requirements
    ///
    /// What the character currently wants or needs, which may drive their actions.
    /// Examples: "Information about the artifact", "Food and rest", "Revenge"
    pub needs: Vec<String>,

    /// Available commands/tools the character can execute
    ///
    /// List of MUD commands that the LLM can invoke on behalf of the character.
    /// Each command includes a description and parameters to guide the LLM in
    /// using them appropriately. Examples: move, attack, say, get, drop, etc.
    pub available_commands: Vec<AvailableCommand>,
}

impl CharacterContext {
    /// Create a new empty character context
    ///
    /// Returns a default context with all fields set to None or empty.
    /// Use the builder methods to populate the context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_name("Gandalf")
    ///     .with_personality("Wise and powerful wizard");
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the character's name
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_name("Elara the Wise");
    /// ```
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the character's personality traits
    ///
    /// Describes the character's core personality and behavioral tendencies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_personality("Mysterious and cautious, speaks in riddles");
    /// ```
    pub fn with_personality(mut self, personality: impl Into<String>) -> Self {
        self.personality = Some(personality.into());
        self
    }

    /// Set the character's current emotional state
    ///
    /// Represents the character's immediate mood or emotional condition.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_emotional_state("Angry and vengeful");
    /// ```
    pub fn with_emotional_state(mut self, state: impl Into<String>) -> Self {
        self.emotional_state = Some(state.into());
        self
    }

    /// Add a single goal to the character's objectives
    ///
    /// Goals represent what the character is trying to achieve. Multiple goals
    /// can be added by calling this method multiple times.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_goal("Find the ancient artifact")
    ///     .with_goal("Protect the village");
    /// ```
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goals.push(goal.into());
        self
    }

    /// Add multiple goals at once
    ///
    /// Convenience method for adding several goals in one call.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_goals(vec![
    ///         "Reclaim the throne".to_string(),
    ///         "Defeat the usurper".to_string(),
    ///     ]);
    /// ```
    pub fn with_goals(mut self, goals: Vec<String>) -> Self {
        self.goals.extend(goals);
        self
    }

    /// Set the character's background or history
    ///
    /// Provides context about the character's past that informs their current
    /// behavior and knowledge.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_background("Former captain of the royal guard, exiled for treason");
    /// ```
    pub fn with_background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Set the current situation or environmental context
    ///
    /// Describes where the character is and what's happening around them.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_situation("Trapped in a dark dungeon cell, hearing footsteps approaching");
    /// ```
    pub fn with_situation(mut self, situation: impl Into<String>) -> Self {
        self.situation = Some(situation.into());
        self
    }

    /// Add a relationship with another character
    ///
    /// Describes connections, feelings, or history with other characters.
    /// Multiple relationships can be added.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_relationship("Distrusts the party's rogue due to past betrayal")
    ///     .with_relationship("Loyal friend to the king");
    /// ```
    pub fn with_relationship(mut self, relationship: impl Into<String>) -> Self {
        self.relationships.push(relationship.into());
        self
    }

    /// Add a current need or desire
    ///
    /// Represents what the character currently wants or needs, which may drive
    /// their actions and dialogue.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_need("Information about the dragon's lair")
    ///     .with_need("Food and rest");
    /// ```
    pub fn with_need(mut self, need: impl Into<String>) -> Self {
        self.needs.push(need.into());
        self
    }

    /// Add an available command that the LLM can invoke
    ///
    /// Commands represent MUD actions the character can take. The LLM will be
    /// informed about these commands and can choose to invoke them.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::{CharacterContext, AvailableCommand};
    ///
    /// let context = CharacterContext::new()
    ///     .with_available_command(
    ///         AvailableCommand::new("move")
    ///             .with_description("Move in a direction")
    ///             .with_parameter("direction", "north, south, east, west")
    ///     )
    ///     .with_available_command(
    ///         AvailableCommand::new("say")
    ///             .with_description("Speak to others")
    ///             .with_parameter("message", "text to say")
    ///     );
    /// ```
    pub fn with_available_command(mut self, command: AvailableCommand) -> Self {
        self.available_commands.push(command);
        self
    }

    /// Add multiple available commands at once
    ///
    /// Convenience method for adding several commands in one call.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::{CharacterContext, AvailableCommand};
    ///
    /// let commands = vec![
    ///     AvailableCommand::new("move").with_description("Move in a direction"),
    ///     AvailableCommand::new("attack").with_description("Attack a target"),
    /// ];
    ///
    /// let context = CharacterContext::new()
    ///     .with_available_commands(commands);
    /// ```
    pub fn with_available_commands(mut self, commands: Vec<AvailableCommand>) -> Self {
        self.available_commands.extend(commands);
        self
    }

    /// Convert the character context into a formatted system message
    ///
    /// Transforms all the context fields into a structured text format suitable
    /// for use as an LLM system message. Only non-empty fields are included.
    ///
    /// The message format is:
    /// - Name: "You are {name}."
    /// - Personality: "Personality: {personality}"
    /// - Background: "Background: {background}"
    /// - Emotional state: "Current emotional state: {state}"
    /// - Goals: "Goals: {goal1}, {goal2}, ..."
    /// - Needs: "Current needs: {need1}, {need2}, ..."
    /// - Situation: "Current situation: {situation}"
    /// - Relationships: "Relationships: {rel1}; {rel2}; ..."
    /// - Available commands: List of commands with descriptions and parameters
    ///
    /// # Returns
    ///
    /// A formatted string suitable for use as a system message in an LLM request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::CharacterContext;
    ///
    /// let context = CharacterContext::new()
    ///     .with_name("Thorin")
    ///     .with_personality("Gruff dwarf warrior")
    ///     .with_emotional_state("Angry");
    ///
    /// let message = context.to_system_message();
    /// // Returns: "You are Thorin.\nPersonality: Gruff dwarf warrior\nCurrent emotional state: Angry"
    /// ```
    pub fn to_system_message(&self) -> String {
        let mut parts = Vec::new();

        if let Some(name) = &self.name {
            parts.push(format!("You are {}.", name));
        }

        if let Some(personality) = &self.personality {
            parts.push(format!("Personality: {}", personality));
        }

        if let Some(background) = &self.background {
            parts.push(format!("Background: {}", background));
        }

        if let Some(emotional_state) = &self.emotional_state {
            parts.push(format!("Current emotional state: {}", emotional_state));
        }

        if !self.goals.is_empty() {
            parts.push(format!("Goals: {}", self.goals.join(", ")));
        }

        if !self.needs.is_empty() {
            parts.push(format!("Current needs: {}", self.needs.join(", ")));
        }

        if let Some(situation) = &self.situation {
            parts.push(format!("Current situation: {}", situation));
        }

        if !self.relationships.is_empty() {
            parts.push(format!("Relationships: {}", self.relationships.join("; ")));
        }

        if !self.available_commands.is_empty() {
            parts.push("\nAvailable commands you can use:".to_string());
            for cmd in &self.available_commands {
                parts.push(cmd.to_string_format());
            }
        }

        parts.join("\n")
    }
}

/// Request parameters for LLM completion
///
/// Represents a complete request to an LLM provider, including conversation history,
/// model selection, sampling parameters, and optional character context.
///
/// # Sampling Parameters
///
/// - `temperature`: Controls randomness (0.0 = deterministic, 2.0 = very random)
/// - `max_tokens`: Maximum number of tokens to generate in the response
/// - `top_p`: Nucleus sampling threshold (0.0-1.0)
/// - `frequency_penalty`: Penalizes frequent tokens (-2.0 to 2.0)
/// - `presence_penalty`: Penalizes tokens that have appeared (-2.0 to 2.0)
///
/// # Examples
///
/// ```rust
/// use wyldlands_server::models::{LLMRequest, LLMMessage, CharacterContext};
///
/// // Simple request
/// let request = LLMRequest::new("gpt-4")
///     .with_message(LLMMessage::user("Hello!"))
///     .with_temperature(0.7)
///     .with_max_tokens(100);
///
/// // Request with character context
/// let context = CharacterContext::new()
///     .with_name("Elara")
///     .with_personality("Wise wizard");
///
/// let contextual_request = LLMRequest::new("gpt-4")
///     .with_context(context)
///     .with_message(LLMMessage::user("What do you see?"))
///     .build_with_context();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    /// Conversation history as a sequence of messages
    ///
    /// Messages are processed in order, with system messages providing context,
    /// user messages representing input, and assistant messages showing previous responses.
    pub messages: Vec<LLMMessage>,

    /// Model identifier to use for completion
    ///
    /// The format depends on the provider:
    /// - OpenAI: "gpt-4", "gpt-3.5-turbo", etc.
    /// - Ollama: "llama2", "mistral", etc.
    /// - Mistral: Model file path or identifier
    pub model: String,

    /// Temperature for sampling (0.0 - 2.0)
    ///
    /// Controls randomness in the output:
    /// - 0.0: Deterministic, always picks the most likely token
    /// - 0.7: Balanced creativity and coherence (recommended for most uses)
    /// - 1.0: Standard sampling
    /// - 2.0: Very random and creative
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum number of tokens to generate
    ///
    /// Limits the length of the response. One token is roughly 4 characters
    /// or 0.75 words in English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p (nucleus) sampling threshold (0.0 - 1.0)
    ///
    /// Only considers tokens whose cumulative probability mass is within top_p.
    /// Lower values make output more focused and deterministic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0)
    ///
    /// Penalizes tokens based on how often they appear in the text so far.
    /// Positive values reduce repetition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 to 2.0)
    ///
    /// Penalizes tokens that have already appeared in the text.
    /// Positive values encourage discussing new topics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Optional character context for NPC interactions
    ///
    /// When present, this context is converted to a system message via
    /// `build_with_context()`. Not serialized in API requests.
    #[serde(skip)]
    pub context: Option<CharacterContext>,
}

impl LLMRequest {
    /// Create a new LLM request with the specified model
    ///
    /// Initializes a request with empty messages and no sampling parameters set.
    /// Use the builder methods to configure the request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMRequest;
    ///
    /// let request = LLMRequest::new("gpt-4");
    /// ```
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            model: model.into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            context: None,
        }
    }

    /// Add a single message to the conversation history
    ///
    /// Messages are appended in the order they are added.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::{LLMRequest, LLMMessage};
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_message(LLMMessage::system("You are helpful"))
    ///     .with_message(LLMMessage::user("Hello!"));
    /// ```
    pub fn with_message(mut self, message: LLMMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages at once
    ///
    /// Convenience method for adding a batch of messages to the conversation history.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::{LLMRequest, LLMMessage};
    ///
    /// let history = vec![
    ///     LLMMessage::user("What's the weather?"),
    ///     LLMMessage::assistant("It's sunny today."),
    /// ];
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_messages(history);
    /// ```
    pub fn with_messages(mut self, messages: Vec<LLMMessage>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set the temperature for sampling
    ///
    /// Controls randomness in the output. Typical values:
    /// - 0.0: Deterministic
    /// - 0.7: Balanced (recommended)
    /// - 1.0: Standard
    /// - 2.0: Very creative
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMRequest;
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_temperature(0.7);
    /// ```
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum number of tokens to generate
    ///
    /// Limits the length of the response. One token ≈ 4 characters or 0.75 words.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMRequest;
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_max_tokens(500);
    /// ```
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the top-p (nucleus) sampling threshold
    ///
    /// Only considers tokens whose cumulative probability is within top_p.
    /// Lower values (e.g., 0.1) make output more focused.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMRequest;
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_top_p(0.9);
    /// ```
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set character context for NPC interactions
    ///
    /// The context will be converted to a system message when `build_with_context()` is called.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::{LLMRequest, CharacterContext};
    ///
    /// let context = CharacterContext::new()
    ///     .with_name("Elara")
    ///     .with_personality("Wise wizard");
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_context(context);
    /// ```
    pub fn with_context(mut self, context: CharacterContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Build the request with character context injected as a system message
    ///
    /// If a context is present, it will be converted to a system message and:
    /// - Inserted at the beginning if no system message exists
    /// - Prepended to the existing system message if one is present
    ///
    /// This method should be called after all messages and context have been added.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::{LLMRequest, LLMMessage, CharacterContext};
    ///
    /// let context = CharacterContext::new()
    ///     .with_name("Thorin")
    ///     .with_personality("Gruff dwarf");
    ///
    /// let request = LLMRequest::new("gpt-4")
    ///     .with_context(context)
    ///     .with_message(LLMMessage::user("What do you think?"))
    ///     .build_with_context();
    /// ```
    pub fn build_with_context(mut self) -> Self {
        if let Some(context) = &self.context {
            let system_message = LLMMessage::system(context.to_system_message());
            // Insert at the beginning if no system message exists, or replace the first one
            if self.messages.is_empty() || self.messages[0].role != LLMRole::System {
                self.messages.insert(0, system_message);
            } else {
                // Prepend context to existing system message
                let existing_content = &self.messages[0].content;
                self.messages[0].content =
                    format!("{}\n\n{}", context.to_system_message(), existing_content);
            }
        }
        self
    }
}

/// Response from an LLM completion request
///
/// Contains the generated text, model information, and token usage statistics.
///
/// # Examples
///
/// ```rust
/// use wyldlands_server::models::LLMResponse;
///
/// let response = LLMResponse::new("Hello, world!", "gpt-4");
/// println!("Generated: {}", response.content);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    /// The generated text content from the LLM
    pub content: String,

    /// The model that generated this response
    pub model: String,

    /// Number of tokens in the prompt (if available)
    ///
    /// Useful for tracking API costs and understanding input size.
    pub prompt_tokens: Option<u32>,

    /// Number of tokens generated in the completion (if available)
    ///
    /// Useful for tracking API costs and response length.
    pub completion_tokens: Option<u32>,

    /// Total tokens used (prompt + completion, if available)
    pub total_tokens: Option<u32>,

    /// Reason the generation finished (if available)
    ///
    /// Common values: "stop" (natural end), "length" (max tokens reached),
    /// "content_filter" (filtered by safety systems)
    pub finish_reason: Option<String>,
}

impl LLMResponse {
    /// Create a new response with minimal information
    ///
    /// Token counts and finish reason will be None. Use this for testing
    /// or when detailed statistics aren't available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMResponse;
    ///
    /// let response = LLMResponse::new("Generated text", "gpt-4");
    /// ```
    pub fn new(content: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            model: model.into(),
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            finish_reason: None,
        }
    }
}

/// Errors that can occur during LLM operations
///
/// Provides detailed error types for different failure modes when interacting
/// with LLM providers.
#[derive(Debug, Clone)]
pub enum LLMError {
    /// Network or connection error occurred
    ///
    /// The provider could not be reached due to network issues.
    NetworkError(String),

    /// API error from the provider
    ///
    /// The provider returned an error (e.g., invalid request, rate limit exceeded,
    /// model not found).
    ApiError(String),

    /// Authentication failed
    ///
    /// The API key was invalid, missing, or expired.
    AuthError(String),

    /// Configuration error
    ///
    /// The provider configuration was invalid or incomplete.
    ConfigError(String),

    /// Provider not available or unreachable
    ///
    /// The specified provider could not be found or is not responding.
    ProviderUnavailable(String),

    /// Request timeout
    ///
    /// The request took too long and was cancelled.
    Timeout(String),

    /// Other unspecified error
    ///
    /// Catch-all for errors that don't fit other categories.
    Other(String),
}

impl fmt::Display for LLMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            LLMError::ApiError(msg) => write!(f, "API error: {}", msg),
            LLMError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            LLMError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            LLMError::ProviderUnavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            LLMError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            LLMError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for LLMError {}

/// Configuration for an LLM provider
///
/// Contains all necessary information to connect to and use an LLM provider,
/// including authentication, endpoints, and operational parameters.
///
/// # Supported Providers
///
/// - **OpenAI**: Cloud-based API (requires API key)
/// - **Ollama**: Local server for running open-source models
/// - **LM Studio**: Local OpenAI-compatible server
/// - **Mistral**: Embedded models using mistral.rs
///
/// # Examples
///
/// ```rust
/// use wyldlands_server::models::LLMConfig;
///
/// // OpenAI configuration
/// let openai = LLMConfig::openai("sk-...", "gpt-4");
///
/// // Ollama configuration
/// let ollama = LLMConfig::ollama("http://localhost:11434/api/chat", "llama2");
///
/// // Mistral embedded configuration
/// let mistral = LLMConfig::mistral("mistral-7b-instruct");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Provider type identifier
    ///
    /// Valid values: "openai", "ollama", "lmstudio", "mistral"
    pub provider: String,

    /// API endpoint URL
    ///
    /// The base URL for API requests. Not used for embedded providers like Mistral.
    pub endpoint: String,

    /// API key for authentication (if required)
    ///
    /// Required for OpenAI, optional for others. Mistral and Ollama typically don't need keys.
    pub api_key: Option<String>,

    /// Default model identifier to use
    ///
    /// Format varies by provider:
    /// - OpenAI: "gpt-4", "gpt-3.5-turbo"
    /// - Ollama: "llama2", "mistral"
    /// - Mistral: Model file path or HuggingFace identifier
    pub default_model: String,

    /// Request timeout in seconds
    ///
    /// How long to wait for a response before timing out.
    /// Embedded models typically need longer timeouts.
    pub timeout_seconds: u64,

    /// Maximum number of retry attempts on failure
    ///
    /// Retries are attempted for transient errors like network issues.
    pub max_retries: u32,
}

impl LLMConfig {
    /// Create configuration for OpenAI API
    ///
    /// Sets up connection to OpenAI's cloud API with standard defaults.
    ///
    /// # Parameters
    ///
    /// - `api_key`: Your OpenAI API key (starts with "sk-")
    /// - `model`: Model to use (e.g., "gpt-4", "gpt-3.5-turbo")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMConfig;
    ///
    /// let config = LLMConfig::openai("sk-your-api-key", "gpt-4");
    /// ```
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: "openai".to_string(),
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: Some(api_key.into()),
            default_model: model.into(),
            timeout_seconds: 30,
            max_retries: 3,
        }
    }

    /// Create configuration for Ollama local server
    ///
    /// Ollama runs open-source models locally. No API key required.
    ///
    /// # Parameters
    ///
    /// - `endpoint`: Ollama API endpoint (typically "http://localhost:11434/api/chat")
    /// - `model`: Model name (e.g., "llama2", "mistral", "codellama")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMConfig;
    ///
    /// let config = LLMConfig::ollama("http://localhost:11434/api/chat", "llama2");
    /// ```
    pub fn ollama(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: "ollama".to_string(),
            endpoint: endpoint.into(),
            api_key: None,
            default_model: model.into(),
            timeout_seconds: 60,
            max_retries: 3,
        }
    }

    /// Create configuration for LM Studio local server
    ///
    /// LM Studio provides an OpenAI-compatible API for local models.
    ///
    /// # Parameters
    ///
    /// - `endpoint`: LM Studio endpoint (typically "http://localhost:1234/v1/chat/completions")
    /// - `model`: Model identifier (can be any string, LM Studio uses loaded model)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMConfig;
    ///
    /// let config = LLMConfig::lmstudio(
    ///     "http://localhost:1234/v1/chat/completions",
    ///     "local-model"
    /// );
    /// ```
    pub fn lmstudio(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: "lmstudio".to_string(),
            endpoint: endpoint.into(),
            api_key: None,
            default_model: model.into(),
            timeout_seconds: 60,
            max_retries: 3,
        }
    }

    /// Create configuration for embedded Mistral models
    ///
    /// Uses mistral.rs to run models directly in-process. No external server needed.
    /// Models are loaded from HuggingFace or local files.
    ///
    /// # Parameters
    ///
    /// - `model`: Model identifier (HuggingFace repo or local path)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wyldlands_server::models::LLMConfig;
    ///
    /// let config = LLMConfig::mistral("mistralai/Mistral-7B-Instruct-v0.2");
    /// ```
    ///
    /// # Note
    ///
    /// Embedded models have longer timeout (120s) as they may need to load
    /// and initialize on first use.
    pub fn mistral(model: impl Into<String>) -> Self {
        Self {
            provider: "mistral".to_string(),
            endpoint: String::new(), // Not used for embedded models
            api_key: None,
            default_model: model.into(),
            timeout_seconds: 120, // Embedded models may take longer
            max_retries: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_message_creation() {
        let system_msg = LLMMessage::system("You are a helpful assistant");
        assert_eq!(system_msg.role, LLMRole::System);

        let user_msg = LLMMessage::user("Hello!");
        assert_eq!(user_msg.role, LLMRole::User);

        let assistant_msg = LLMMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, LLMRole::Assistant);
    }

    #[test]
    fn test_llm_request_builder() {
        let request = LLMRequest::new("gpt-4")
            .with_message(LLMMessage::system("You are helpful"))
            .with_message(LLMMessage::user("Hello"))
            .with_temperature(0.7)
            .with_max_tokens(100);

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn test_llm_config() {
        let openai = LLMConfig::openai("sk-test", "gpt-4");
        assert_eq!(openai.provider, "openai");
        assert!(openai.api_key.is_some());

        let ollama = LLMConfig::ollama("http://localhost:11434", "llama2");
        assert_eq!(ollama.provider, "ollama");
        assert!(ollama.api_key.is_none());

        let mistral = LLMConfig::mistral("mistral-7b-instruct");
        assert_eq!(mistral.provider, "mistral");
        assert!(mistral.api_key.is_none());
    }

    #[test]
    fn test_character_context_builder() {
        let context = CharacterContext::new()
            .with_name("Elara")
            .with_personality("Wise and mysterious wizard")
            .with_emotional_state("Curious and cautious")
            .with_goal("Find the ancient artifact")
            .with_background("Former court mage of the kingdom")
            .with_situation("Standing in a dark forest clearing")
            .with_relationship("Distrusts the party's rogue")
            .with_need("Information about the artifact's location");

        assert_eq!(context.name, Some("Elara".to_string()));
        assert_eq!(context.goals.len(), 1);
        assert_eq!(context.relationships.len(), 1);
        assert_eq!(context.needs.len(), 1);
    }

    #[test]
    fn test_character_context_to_system_message() {
        let context = CharacterContext::new()
            .with_name("Thorin")
            .with_personality("Gruff but loyal dwarf warrior")
            .with_emotional_state("Angry about recent betrayal")
            .with_goal("Reclaim the ancestral halls");

        let message = context.to_system_message();

        assert!(message.contains("You are Thorin"));
        assert!(message.contains("Personality: Gruff but loyal dwarf warrior"));
        assert!(message.contains("Current emotional state: Angry about recent betrayal"));
        assert!(message.contains("Goals: Reclaim the ancestral halls"));
    }

    #[test]
    fn test_llm_request_with_context() {
        let context = CharacterContext::new()
            .with_name("Aria")
            .with_personality("Cheerful bard")
            .with_emotional_state("Excited");

        let request = LLMRequest::new("gpt-4")
            .with_context(context)
            .with_message(LLMMessage::user("What do you think of this tavern?"))
            .build_with_context();

        // Should have system message with context + user message
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, LLMRole::System);
        assert!(request.messages[0].content.contains("You are Aria"));
        assert_eq!(request.messages[1].role, LLMRole::User);
    }

    #[test]
    fn test_llm_request_context_prepends_to_existing_system() {
        let context = CharacterContext::new()
            .with_name("Zara")
            .with_personality("Cunning thief");

        let request = LLMRequest::new("gpt-4")
            .with_message(LLMMessage::system("You are in a fantasy world"))
            .with_context(context)
            .with_message(LLMMessage::user("What should I do?"))
            .build_with_context();

        // Should still have 2 messages (system + user)
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, LLMRole::System);
        // Context should be prepended to existing system message
        assert!(request.messages[0].content.contains("You are Zara"));
        assert!(
            request.messages[0]
                .content
                .contains("You are in a fantasy world")
        );
    }

    #[test]
    fn test_character_context_with_multiple_goals() {
        let context = CharacterContext::new().with_name("Marcus").with_goals(vec![
            "Protect the village".to_string(),
            "Find his missing brother".to_string(),
            "Defeat the dragon".to_string(),
        ]);

        let message = context.to_system_message();
        assert!(message.contains("Protect the village"));
        assert!(message.contains("Find his missing brother"));
        assert!(message.contains("Defeat the dragon"));
    }
}

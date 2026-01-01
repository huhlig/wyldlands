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

//! Common types for LLM integration

use serde::{Deserialize, Serialize};
use std::fmt;

/// LLM message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

impl fmt::Display for LlmRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmRole::System => write!(f, "system"),
            LlmRole::User => write!(f, "user"),
            LlmRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// A message in an LLM conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

impl LlmMessage {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::System,
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: LlmRole::Assistant,
            content: content.into(),
        }
    }
}

/// LLM request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Conversation history
    pub messages: Vec<LlmMessage>,
    /// Model to use (provider-specific)
    pub model: String,
    /// Temperature (0.0 - 2.0, higher = more random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
}

impl LlmRequest {
    /// Create a new LLM request
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            model: model.into(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }

    /// Add a message to the request
    pub fn with_message(mut self, message: LlmMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages
    pub fn with_messages(mut self, messages: Vec<LlmMessage>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set top-p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Generated content
    pub content: String,
    /// Model used
    pub model: String,
    /// Tokens used in prompt
    pub prompt_tokens: Option<u32>,
    /// Tokens generated
    pub completion_tokens: Option<u32>,
    /// Total tokens
    pub total_tokens: Option<u32>,
    /// Finish reason
    pub finish_reason: Option<String>,
}

impl LlmResponse {
    /// Create a new response
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

/// LLM error types
#[derive(Debug, Clone)]
pub enum LlmError {
    /// Network or connection error
    NetworkError(String),
    /// API error (invalid request, rate limit, etc.)
    ApiError(String),
    /// Authentication error
    AuthError(String),
    /// Invalid configuration
    ConfigError(String),
    /// Provider not available
    ProviderUnavailable(String),
    /// Timeout
    Timeout(String),
    /// Other error
    Other(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            LlmError::ApiError(msg) => write!(f, "API error: {}", msg),
            LlmError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
            LlmError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            LlmError::ProviderUnavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            LlmError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            LlmError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider type (openai, ollama, lmstudio)
    pub provider: String,
    /// API endpoint URL
    pub endpoint: String,
    /// API key (if required)
    pub api_key: Option<String>,
    /// Default model to use
    pub default_model: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries
    pub max_retries: u32,
}

impl LlmConfig {
    /// Create OpenAI configuration
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

    /// Create Ollama configuration
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

    /// Create LM Studio configuration
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_message_creation() {
        let system_msg = LlmMessage::system("You are a helpful assistant");
        assert_eq!(system_msg.role, LlmRole::System);

        let user_msg = LlmMessage::user("Hello!");
        assert_eq!(user_msg.role, LlmRole::User);

        let assistant_msg = LlmMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, LlmRole::Assistant);
    }

    #[test]
    fn test_llm_request_builder() {
        let request = LlmRequest::new("gpt-4")
            .with_message(LlmMessage::system("You are helpful"))
            .with_message(LlmMessage::user("Hello"))
            .with_temperature(0.7)
            .with_max_tokens(100);

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
    }

    #[test]
    fn test_llm_config() {
        let openai = LlmConfig::openai("sk-test", "gpt-4");
        assert_eq!(openai.provider, "openai");
        assert!(openai.api_key.is_some());

        let ollama = LlmConfig::ollama("http://localhost:11434", "llama2");
        assert_eq!(ollama.provider, "ollama");
        assert!(ollama.api_key.is_none());
    }
}

// Made with Bob

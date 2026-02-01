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

//! LLM Manager for coordinating multiple providers

use super::providers::{
    LlmProvider, LmStudioProvider, MistralProvider, OllamaProvider, OpenAiProvider,
};
use super::types::{CharacterContext, LLMConfig, LLMError, LLMRequest, LLMResponse};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// LLM Manager handles multiple providers and routing
pub struct ModelManager {
    llm_providers: Arc<RwLock<HashMap<String, Box<dyn LlmProvider>>>>,
    default_llm_provider: Arc<RwLock<Option<String>>>,
}

impl ModelManager {
    /// Create a new Model manager
    pub fn new() -> Self {
        Self {
            llm_providers: Arc::new(RwLock::new(HashMap::new())),
            default_llm_provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a provider
    pub async fn register_llm_provider(
        &self,
        name: impl Into<String>,
        config: LLMConfig,
    ) -> Result<(), LLMError> {
        let name = name.into();
        let provider: Box<dyn LlmProvider> = match config.provider.as_str() {
            "openai" => Box::new(OpenAiProvider::new(config)?),
            "ollama" => Box::new(OllamaProvider::new(config)?),
            "lmstudio" => Box::new(LmStudioProvider::new(config)?),
            "mistral" => Box::new(MistralProvider::new(config).await?),
            _ => {
                return Err(LLMError::ConfigError(format!(
                    "Unknown provider type: {}",
                    config.provider
                )));
            }
        };

        let mut providers = self.llm_providers.write().await;
        providers.insert(name.clone(), provider);

        // Set as default if it's the first provider
        let mut default = self.default_llm_provider.write().await;
        if default.is_none() {
            *default = Some(name);
        }

        Ok(())
    }

    /// Set the default provider
    pub async fn set_default_llm_provider(&self, name: impl Into<String>) -> Result<(), LLMError> {
        let name = name.into();
        let providers = self.llm_providers.read().await;

        if !providers.contains_key(&name) {
            return Err(LLMError::ConfigError(format!(
                "Provider '{}' not registered",
                name
            )));
        }

        let mut default = self.default_llm_provider.write().await;
        *default = Some(name);

        Ok(())
    }

    /// Get a provider by name
    async fn get_llm_provider(&self, name: &str) -> Result<Box<dyn LlmProvider>, LLMError> {
        let providers = self.llm_providers.read().await;
        providers
            .get(name)
            .map(|p| {
                // TODO: FIX This
                // This is a workaround since we can't clone trait objects
                // In a real implementation, you'd want to use Arc<dyn LlmProvider>
                Err(LLMError::Other(
                    "Provider cloning not implemented".to_string(),
                ))
            })
            .unwrap_or_else(|| {
                Err(LLMError::ProviderUnavailable(format!(
                    "Provider '{}' not found",
                    name
                )))
            })
    }

    /// Send a completion request using the default provider
    pub async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let default = self.default_llm_provider.read().await;
        let provider_name = default
            .as_ref()
            .ok_or_else(|| LLMError::ConfigError("No default provider set".to_string()))?;

        self.complete_with_llm_provider(provider_name, request)
            .await
    }

    /// Send a completion request using a specific provider
    pub async fn complete_with_llm_provider(
        &self,
        provider_name: &str,
        request: LLMRequest,
    ) -> Result<LLMResponse, LLMError> {
        let providers = self.llm_providers.read().await;
        let provider = providers.get(provider_name).ok_or_else(|| {
            LLMError::ProviderUnavailable(format!("Provider '{}' not found", provider_name))
        })?;

        provider.complete(request).await
    }

    /// Check if a provider is available
    pub async fn is_llm_provider_available(&self, provider_name: &str) -> bool {
        let providers = self.llm_providers.read().await;
        if let Some(provider) = providers.get(provider_name) {
            provider.is_available().await
        } else {
            false
        }
    }

    /// List all registered providers
    pub async fn list_llm_providers(&self) -> Vec<String> {
        let providers = self.llm_providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Get the default provider name
    pub async fn get_default_llm_provider(&self) -> Option<String> {
        let default = self.default_llm_provider.read().await;
        default.clone()
    }

    /// Remove a provider
    pub async fn remove_llm_provider(&self, name: &str) -> Result<(), LLMError> {
        let mut providers = self.llm_providers.write().await;
        providers.remove(name);

        // Clear default if it was the removed provider
        let mut default = self.default_llm_provider.write().await;
        if default.as_ref().map(|d| d == name).unwrap_or(false) {
            *default = None;
        }

        Ok(())
    }

    /// Create a simple completion request with a single user message
    pub fn create_simple_llm_request(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
    ) -> LLMRequest {
        LLMRequest::new(model)
            .with_message(super::types::LLMMessage::user(prompt))
            .with_temperature(0.7)
            .with_max_tokens(500)
    }

    /// Create a request with system prompt and user message
    pub fn create_llm_request_with_system(
        &self,
        model: impl Into<String>,
        system_prompt: impl Into<String>,
        user_message: impl Into<String>,
    ) -> LLMRequest {
        LLMRequest::new(model)
            .with_message(super::types::LLMMessage::system(system_prompt))
            .with_message(super::types::LLMMessage::user(user_message))
            .with_temperature(0.7)
            .with_max_tokens(500)
    }

    /// Create a request with character context
    /// The context will be automatically converted to a system message
    pub fn create_llm_request_with_context(
        &self,
        model: impl Into<String>,
        context: CharacterContext,
        user_message: impl Into<String>,
    ) -> LLMRequest {
        LLMRequest::new(model)
            .with_context(context)
            .with_message(super::types::LLMMessage::user(user_message))
            .with_temperature(0.7)
            .with_max_tokens(500)
            .build_with_context()
    }

    /// Create a request with character context and conversation history
    pub fn create_llm_contextual_conversation(
        &self,
        model: impl Into<String>,
        context: CharacterContext,
        conversation_history: Vec<super::types::LLMMessage>,
    ) -> LLMRequest {
        LLMRequest::new(model)
            .with_context(context)
            .with_messages(conversation_history)
            .with_temperature(0.7)
            .with_max_tokens(500)
            .build_with_context()
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = ModelManager::new();
        assert!(manager.get_default_llm_provider().await.is_none());
        assert_eq!(manager.list_llm_providers().await.len(), 0);
    }

    #[tokio::test]
    async fn test_register_provider() {
        let manager = ModelManager::new();
        let config = LLMConfig::ollama("http://localhost:11434/api/chat", "llama2");

        let result = manager.register_llm_provider("ollama", config).await;
        assert!(result.is_ok());

        let providers = manager.list_llm_providers().await;
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"ollama".to_string()));

        // Should be set as default automatically
        assert_eq!(
            manager.get_default_llm_provider().await,
            Some("ollama".to_string())
        );
    }

    #[tokio::test]
    async fn test_set_default_provider() {
        let manager = ModelManager::new();

        // Register two providers
        let config1 = LLMConfig::ollama("http://localhost:11434/api/chat", "llama2");
        manager
            .register_llm_provider("ollama", config1)
            .await
            .unwrap();

        let config2 = LLMConfig::lmstudio("http://localhost:1234/v1/chat/completions", "local");
        manager
            .register_llm_provider("lmstudio", config2)
            .await
            .unwrap();

        // Default should be the first one
        assert_eq!(
            manager.get_default_llm_provider().await,
            Some("ollama".to_string())
        );

        // Change default
        manager.set_default_llm_provider("lmstudio").await.unwrap();
        assert_eq!(
            manager.get_default_llm_provider().await,
            Some("lmstudio".to_string())
        );
    }

    #[tokio::test]
    async fn test_remove_provider() {
        let manager = ModelManager::new();
        let config = LLMConfig::ollama("http://localhost:11434/api/chat", "llama2");

        manager
            .register_llm_provider("ollama", config)
            .await
            .unwrap();
        assert_eq!(manager.list_llm_providers().await.len(), 1);

        manager.remove_llm_provider("ollama").await.unwrap();
        assert_eq!(manager.list_llm_providers().await.len(), 0);
        assert!(manager.get_default_llm_provider().await.is_none());
    }

    #[test]
    fn test_create_simple_request() {
        let manager = ModelManager::new();
        let request = manager.create_simple_llm_request("gpt-4", "Hello, world!");

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].content, "Hello, world!");
    }

    #[test]
    fn test_create_request_with_system() {
        let manager = ModelManager::new();
        let request = manager.create_llm_request_with_system(
            "gpt-4",
            "You are a helpful assistant",
            "Hello!",
        );

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].content, "You are a helpful assistant");
        assert_eq!(request.messages[1].content, "Hello!");
    }
}

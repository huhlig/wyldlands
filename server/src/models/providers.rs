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

//! LLM provider implementations

mod lmstudio;
mod mistral;
mod ollama;
mod openai;

pub use self::lmstudio::LmStudioProvider;
pub use self::mistral::MistralProvider;
pub use self::ollama::OllamaProvider;
pub use self::openai::OpenAiProvider;

use super::types::{LLMError, LLMRequest, LLMResponse};
use async_trait::async_trait;

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a request to the LLM
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;

    /// Check if the provider is available
    async fn is_available(&self) -> bool;

    /// Get provider name
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use crate::models::{LLMConfig, LmStudioProvider, OllamaProvider, OpenAiProvider};

    #[test]
    fn test_openai_provider_creation() {
        let config = LLMConfig::openai("test-key", "gpt-4");
        let provider = OpenAiProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_openai_provider_requires_api_key() {
        let mut config = LLMConfig::openai("test-key", "gpt-4");
        config.api_key = None;
        let provider = OpenAiProvider::new(config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_ollama_provider_creation() {
        let config = LLMConfig::ollama("http://localhost:11434/api/chat", "llama2");
        let provider = OllamaProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_lmstudio_provider_creation() {
        let config =
            LLMConfig::lmstudio("http://localhost:1234/v1/chat/completions", "local-model");
        let provider = LmStudioProvider::new(config);
        assert!(provider.is_ok());
    }
}

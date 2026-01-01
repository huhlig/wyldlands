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

//! LLM provider implementations

use super::types::{LlmConfig, LlmError, LlmRequest, LlmResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a request to the LLM
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Check if the provider is available
    async fn is_available(&self) -> bool;

    /// Get provider name
    fn name(&self) -> &str;
}

/// OpenAI provider
pub struct OpenAiProvider {
    config: LlmConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        if config.api_key.is_none() {
            return Err(LlmError::ConfigError(
                "OpenAI requires an API key".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LlmError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        #[derive(Serialize)]
        struct OpenAiRequest {
            model: String,
            messages: Vec<super::types::LlmMessage>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_p: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            frequency_penalty: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            presence_penalty: Option<f32>,
        }

        #[derive(Deserialize)]
        struct OpenAiResponse {
            choices: Vec<OpenAiChoice>,
            usage: Option<OpenAiUsage>,
            model: String,
        }

        #[derive(Deserialize)]
        struct OpenAiChoice {
            message: super::types::LlmMessage,
            finish_reason: Option<String>,
        }

        #[derive(Deserialize)]
        struct OpenAiUsage {
            prompt_tokens: u32,
            completion_tokens: u32,
            total_tokens: u32,
        }

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| LlmError::AuthError("No API key configured".to_string()))?;

        let openai_request = OpenAiRequest {
            model: request.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
        };

        let response = self
            .client
            .post(&self.config.endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        let choice = openai_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ApiError("No choices in response".to_string()))?;

        Ok(LlmResponse {
            content: choice.message.content.clone(),
            model: openai_response.model,
            prompt_tokens: openai_response.usage.as_ref().map(|u| u.prompt_tokens),
            completion_tokens: openai_response.usage.as_ref().map(|u| u.completion_tokens),
            total_tokens: openai_response.usage.as_ref().map(|u| u.total_tokens),
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn is_available(&self) -> bool {
        // Simple health check - try to reach the endpoint
        self.client
            .get(&self.config.endpoint)
            .send()
            .await
            .is_ok()
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
}

/// Ollama provider
pub struct OllamaProvider {
    config: LlmConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LlmError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        #[derive(Serialize)]
        struct OllamaRequest {
            model: String,
            messages: Vec<super::types::LlmMessage>,
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            options: Option<OllamaOptions>,
        }

        #[derive(Serialize)]
        struct OllamaOptions {
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            num_predict: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_p: Option<f32>,
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            message: super::types::LlmMessage,
            model: String,
            #[serde(default)]
            done: bool,
        }

        let options = if request.temperature.is_some()
            || request.max_tokens.is_some()
            || request.top_p.is_some()
        {
            Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
                top_p: request.top_p,
            })
        } else {
            None
        };

        let ollama_request = OllamaRequest {
            model: request.model.clone(),
            messages: request.messages,
            stream: false,
            options,
        };

        let response = self
            .client
            .post(&self.config.endpoint)
            .header("Content-Type", "application/json")
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        Ok(LlmResponse {
            content: ollama_response.message.content,
            model: ollama_response.model,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            finish_reason: if ollama_response.done {
                Some("stop".to_string())
            } else {
                None
            },
        })
    }

    async fn is_available(&self) -> bool {
        // Check if Ollama is running
        self.client
            .get(format!("{}/api/tags", self.config.endpoint.trim_end_matches("/api/chat")))
            .send()
            .await
            .is_ok()
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}

/// LM Studio provider (compatible with OpenAI API)
pub struct LmStudioProvider {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LmStudioProvider {
    /// Create a new LM Studio provider
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LlmError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // LM Studio uses OpenAI-compatible API
        #[derive(Serialize)]
        struct LmStudioRequest {
            model: String,
            messages: Vec<super::types::LlmMessage>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_p: Option<f32>,
        }

        #[derive(Deserialize)]
        struct LmStudioResponse {
            choices: Vec<LmStudioChoice>,
            model: String,
        }

        #[derive(Deserialize)]
        struct LmStudioChoice {
            message: super::types::LlmMessage,
            finish_reason: Option<String>,
        }

        let lmstudio_request = LmStudioRequest {
            model: request.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
        };

        let response = self
            .client
            .post(&self.config.endpoint)
            .header("Content-Type", "application/json")
            .json(&lmstudio_request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::ApiError(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        let lmstudio_response: LmStudioResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        let choice = lmstudio_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ApiError("No choices in response".to_string()))?;

        Ok(LlmResponse {
            content: choice.message.content.clone(),
            model: lmstudio_response.model,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn is_available(&self) -> bool {
        // Check if LM Studio is running
        self.client
            .get(&self.config.endpoint)
            .send()
            .await
            .is_ok()
    }

    fn name(&self) -> &str {
        "LM Studio"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let config = LlmConfig::openai("test-key", "gpt-4");
        let provider = OpenAiProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_openai_provider_requires_api_key() {
        let mut config = LlmConfig::openai("test-key", "gpt-4");
        config.api_key = None;
        let provider = OpenAiProvider::new(config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_ollama_provider_creation() {
        let config = LlmConfig::ollama("http://localhost:11434/api/chat", "llama2");
        let provider = OllamaProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_lmstudio_provider_creation() {
        let config = LlmConfig::lmstudio("http://localhost:1234/v1/chat/completions", "local-model");
        let provider = LmStudioProvider::new(config);
        assert!(provider.is_ok());
    }
}

// Made with Bob

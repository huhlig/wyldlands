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

use crate::llm::{LLMConfig, LLMError, LLMMessage, LLMRequest, LLMResponse, LlmProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Ollama provider
pub struct OllamaProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: LLMConfig) -> Result<Self, LLMError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LLMError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        #[derive(Serialize)]
        struct OllamaRequest {
            model: String,
            messages: Vec<LLMMessage>,
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
            message: LLMMessage,
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
            .map_err(|e| LLMError::NetworkError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::ApiError(format!(
                "API returned {}: {}",
                status, error_text
            )));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ApiError(format!("Failed to parse response: {}", e)))?;

        Ok(LLMResponse {
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
            .get(format!(
                "{}/api/tags",
                self.config.endpoint.trim_end_matches("/api/chat")
            ))
            .send()
            .await
            .is_ok()
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}

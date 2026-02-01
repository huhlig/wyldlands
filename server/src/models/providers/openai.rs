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

use crate::models::{LLMConfig, LLMError, LLMMessage, LLMRequest, LLMResponse, LlmProvider};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OpenAI provider
pub struct OpenAiProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(config: LLMConfig) -> Result<Self, LLMError> {
        if config.api_key.is_none() {
            return Err(LLMError::ConfigError(
                "OpenAI requires an API key".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LLMError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        #[derive(Serialize)]
        struct OpenAiRequest {
            model: String,
            messages: Vec<LLMMessage>,
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
            message: LLMMessage,
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
            .ok_or_else(|| LLMError::AuthError("No API key configured".to_string()))?;

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

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ApiError(format!("Failed to parse response: {}", e)))?;

        let choice = openai_response
            .choices
            .first()
            .ok_or_else(|| LLMError::ApiError("No choices in response".to_string()))?;

        Ok(LLMResponse {
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
        self.client.get(&self.config.endpoint).send().await.is_ok()
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
}

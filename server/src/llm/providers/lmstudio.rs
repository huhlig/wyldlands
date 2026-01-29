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

use crate::llm::{LLMConfig, LLMError, LLMMessage, LlmProvider, LLMRequest, LLMResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// LM Studio provider (compatible with OpenAI API)
pub struct LmStudioProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

impl LmStudioProvider {
    /// Create a new LM Studio provider
    pub fn new(config: LLMConfig) -> Result<Self, LLMError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| LLMError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        // LM Studio uses OpenAI-compatible API
        #[derive(Serialize)]
        struct LmStudioRequest {
            model: String,
            messages: Vec<LLMMessage>,
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
            message: LLMMessage,
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

        let lmstudio_response: LmStudioResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ApiError(format!("Failed to parse response: {}", e)))?;

        let choice = lmstudio_response
            .choices
            .first()
            .ok_or_else(|| LLMError::ApiError("No choices in response".to_string()))?;

        Ok(LLMResponse {
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
        self.client.get(&self.config.endpoint).send().await.is_ok()
    }

    fn name(&self) -> &str {
        "LM Studio"
    }
}

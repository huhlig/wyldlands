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

use crate::models::{LLMConfig, LLMError, LLMRequest, LLMResponse, LLMRole, LlmProvider};
use async_trait::async_trait;
use mistralrs::{IsqType, Model, RequestBuilder, TextMessageRole, TextModelBuilder};

/// Embedded Mistral Provider using mistral.rs
pub struct MistralProvider {
    config: LLMConfig,
    model: Model,
}

impl MistralProvider {
    /// Create a new Mistral provider with an embedded model
    pub async fn new(config: LLMConfig) -> Result<Self, LLMError> {
        let model = TextModelBuilder::new(&config.default_model)
            .with_isq(IsqType::Q4K)
            .with_logging()
            .build()
            .await
            .map_err(|e| LLMError::ConfigError(format!("Failed to build model: {}", e)))?;

        Ok(Self { config, model })
    }
}

#[async_trait]
impl LlmProvider for MistralProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        // Build the request with messages
        let mut mistral_request = RequestBuilder::new();

        for message in &request.messages {
            let role = match message.role {
                LLMRole::System => TextMessageRole::System,
                LLMRole::User => TextMessageRole::User,
                LLMRole::Assistant => TextMessageRole::Assistant,
            };
            mistral_request = mistral_request.add_message(role, &message.content);
        }

        // Set sampling parameters if provided
        if let Some(temp) = request.temperature {
            mistral_request = mistral_request.set_sampler_temperature(temp as f64);
        }
        // Note: mistralrs doesn't expose set_max_tokens or set_top_p in RequestBuilder
        // These would need to be set via sampling params if needed

        // Send the request
        let response = self
            .model
            .send_chat_request(mistral_request)
            .await
            .map_err(|e| LLMError::ApiError(format!("Mistral request failed: {}", e)))?;

        // Extract the response
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LLMError::ApiError("No response choices returned".to_string()))?;

        let content = choice
            .message
            .content
            .as_ref()
            .ok_or_else(|| LLMError::ApiError("No content in response".to_string()))?
            .clone();

        Ok(LLMResponse {
            content,
            model: self.config.default_model.clone(),
            prompt_tokens: Some(response.usage.prompt_tokens as u32),
            completion_tokens: Some(response.usage.completion_tokens as u32),
            total_tokens: Some(response.usage.total_tokens as u32),
            finish_reason: Some(choice.finish_reason.clone()),
        })
    }

    async fn is_available(&self) -> bool {
        // The model is embedded, so it's always available once loaded
        true
    }

    fn name(&self) -> &str {
        "Mistral (Embedded)"
    }
}

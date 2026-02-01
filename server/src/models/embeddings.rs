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

//! # Embedding Generation Module
//!
//! This module provides vector embedding generation for memory content using
//! sentence transformers via the Candle ML framework.
//!
//! ## Features
//!
//! - **Sentence Transformers**: Uses pre-trained models for semantic embeddings
//! - **Model Caching**: Downloads and caches models locally
//! - **Batch Processing**: Efficient batch embedding generation
//! - **Multiple Models**: Support for different embedding models
//!
//! ## Supported Models
//!
//! - `all-MiniLM-L6-v2`: Fast, 384-dimensional embeddings (default)
//! - `all-mpnet-base-v2`: High quality, 768-dimensional embeddings
//! - `paraphrase-multilingual-MiniLM-L12-v2`: Multilingual support
//!
//! ## Usage
//!
//! ```rust,ignore
//! use wyldlands_server::ecs::embeddings::EmbeddingGenerator;
//!
//! // Create generator with default model
//! let generator = EmbeddingGenerator::new().await?;
//!
//! // Generate embedding for text
//! let embedding = generator.generate("The dragon guards its treasure").await?;
//!
//! // Batch generation
//! let texts = vec!["text1", "text2", "text3"];
//! let embeddings = generator.generate_batch(&texts).await?;
//! ```

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{Repo, RepoType, api::sync::Api};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokenizers::Tokenizer;
use tokio::sync::RwLock;

/// Errors that can occur during embedding generation
#[derive(Debug, Error)]
pub enum EmbeddingError {
    /// Failed to load the model
    #[error("Model loading failed: {0}")]
    ModelLoadError(String),

    /// Failed to tokenize text
    #[error("Tokenization failed: {0}")]
    TokenizationError(String),

    /// Failed to generate embedding
    #[error("Embedding generation failed: {0}")]
    GenerationError(String),

    /// Model not initialized
    #[error("Model not initialized")]
    NotInitialized,

    /// Candle framework error
    #[error("Candle error: {0}")]
    CandleError(#[from] candle_core::Error),

    /// Tokenizer error
    #[error("Tokenizer error: {0}")]
    TokenizerError(String),
}

/// Result type for embedding operations
pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

/// Supported embedding models
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModel {
    /// all-MiniLM-L6-v2: Fast, 384-dimensional embeddings
    /// Best for: Speed, resource-constrained environments
    MiniLM,

    /// all-mpnet-base-v2: High quality, 768-dimensional embeddings
    /// Best for: Accuracy, semantic similarity
    MPNet,

    /// paraphrase-multilingual-MiniLM-L12-v2: Multilingual, 384-dimensional
    /// Best for: Multi-language support
    MultilingualMiniLM,
}

impl EmbeddingModel {
    /// Get the Hugging Face model identifier
    fn model_id(&self) -> &'static str {
        match self {
            EmbeddingModel::MiniLM => "sentence-transformers/all-MiniLM-L6-v2",
            EmbeddingModel::MPNet => "sentence-transformers/all-mpnet-base-v2",
            EmbeddingModel::MultilingualMiniLM => {
                "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2"
            }
        }
    }

    /// Get the embedding dimension for this model
    pub fn dimension(&self) -> usize {
        match self {
            EmbeddingModel::MiniLM => 384,
            EmbeddingModel::MPNet => 768,
            EmbeddingModel::MultilingualMiniLM => 384,
        }
    }
}

impl Default for EmbeddingModel {
    fn default() -> Self {
        EmbeddingModel::MiniLM
    }
}

/// Generator for creating vector embeddings from text
///
/// Uses sentence transformer models via Candle to generate semantic embeddings
/// suitable for similarity search and memory retrieval.
pub struct EmbeddingGenerator {
    model: Arc<RwLock<Option<BertModel>>>,
    tokenizer: Arc<RwLock<Option<Tokenizer>>>,
    device: Device,
    model_type: EmbeddingModel,
    model_path: Option<PathBuf>,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator with the default model
    ///
    /// Downloads the model if not already cached locally.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let generator = EmbeddingGenerator::new().await?;
    /// ```
    pub async fn new() -> EmbeddingResult<Self> {
        Self::with_model(EmbeddingModel::default()).await
    }

    /// Create a new embedding generator with a specific model
    ///
    /// # Arguments
    ///
    /// * `model_type` - The embedding model to use
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let generator = EmbeddingGenerator::with_model(EmbeddingModel::MPNet).await?;
    /// ```
    pub async fn with_model(model_type: EmbeddingModel) -> EmbeddingResult<Self> {
        let device = Device::Cpu; // Use CPU for now, can add GPU support later

        Ok(Self {
            model: Arc::new(RwLock::new(None)),
            tokenizer: Arc::new(RwLock::new(None)),
            device,
            model_type,
            model_path: None,
        })
    }

    /// Initialize the model (lazy loading)
    ///
    /// Downloads and loads the model and tokenizer if not already loaded.
    async fn ensure_initialized(&self) -> EmbeddingResult<()> {
        let model_guard = self.model.read().await;
        if model_guard.is_some() {
            return Ok(());
        }
        drop(model_guard);

        // Download model files
        let api = Api::new().map_err(|e| EmbeddingError::ModelLoadError(e.to_string()))?;

        let repo = api.repo(Repo::new(
            self.model_type.model_id().to_string(),
            RepoType::Model,
        ));

        let config_path = repo
            .get("config.json")
            .map_err(|e| EmbeddingError::ModelLoadError(e.to_string()))?;

        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| EmbeddingError::ModelLoadError(e.to_string()))?;

        let weights_path = repo
            .get("model.safetensors")
            .map_err(|e| EmbeddingError::ModelLoadError(e.to_string()))?;

        // Load config
        let config = std::fs::read_to_string(config_path)
            .map_err(|e| EmbeddingError::ModelLoadError(e.to_string()))?;
        let config: Config = serde_json::from_str(&config)
            .map_err(|e| EmbeddingError::ModelLoadError(e.to_string()))?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| EmbeddingError::TokenizerError(e.to_string()))?;

        // Load model weights
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &self.device)? };

        let model = BertModel::load(vb, &config)?;

        // Store model and tokenizer
        let mut model_guard = self.model.write().await;
        *model_guard = Some(model);
        drop(model_guard);

        let mut tokenizer_guard = self.tokenizer.write().await;
        *tokenizer_guard = Some(tokenizer);
        drop(tokenizer_guard);

        Ok(())
    }

    /// Generate an embedding for a single text
    ///
    /// # Arguments
    ///
    /// * `text` - The text to embed
    ///
    /// # Returns
    ///
    /// Returns a vector of floats representing the embedding
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let embedding = generator.generate("The dragon guards its treasure").await?;
    /// assert_eq!(embedding.len(), 384); // For MiniLM model
    /// ```
    pub async fn generate(&self, text: &str) -> EmbeddingResult<Vec<f32>> {
        self.ensure_initialized().await?;

        let tokenizer_guard = self.tokenizer.read().await;
        let tokenizer = tokenizer_guard
            .as_ref()
            .ok_or(EmbeddingError::NotInitialized)?;

        // Tokenize
        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| EmbeddingError::TokenizationError(e.to_string()))?;

        let tokens = encoding.get_ids();
        let token_ids = Tensor::new(tokens, &self.device)?.unsqueeze(0)?; // Add batch dimension

        let token_type_ids = Tensor::zeros_like(&token_ids)?;

        drop(tokenizer_guard);

        // Generate embedding
        let model_guard = self.model.read().await;
        let model = model_guard.as_ref().ok_or(EmbeddingError::NotInitialized)?;

        let embeddings = model.forward(&token_ids, &token_type_ids, None)?;

        // Mean pooling
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = embeddings.squeeze(0)?;

        // Normalize
        let embeddings = self.normalize_l2(&embeddings)?;

        // Convert to Vec<f32>
        let embedding_vec = embeddings.to_vec1::<f32>()?;

        Ok(embedding_vec)
    }

    /// Generate embeddings for multiple texts in batch
    ///
    /// More efficient than calling `generate()` multiple times.
    ///
    /// # Arguments
    ///
    /// * `texts` - Slice of texts to embed
    ///
    /// # Returns
    ///
    /// Returns a vector of embeddings, one for each input text
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let texts = vec!["text1", "text2", "text3"];
    /// let embeddings = generator.generate_batch(&texts).await?;
    /// assert_eq!(embeddings.len(), 3);
    /// ```
    pub async fn generate_batch(&self, texts: &[&str]) -> EmbeddingResult<Vec<Vec<f32>>> {
        self.ensure_initialized().await?;

        let mut embeddings = Vec::with_capacity(texts.len());

        // TODO: Implement true batch processing
        // For now, process sequentially
        for text in texts {
            let embedding = self.generate(text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    /// Normalize a tensor using L2 normalization
    fn normalize_l2(&self, tensor: &Tensor) -> EmbeddingResult<Tensor> {
        let sum_sqr = tensor.sqr()?.sum_all()?;
        let norm = sum_sqr.sqrt()?;
        let normalized = tensor.broadcast_div(&norm)?;
        Ok(normalized)
    }

    /// Get the dimension of embeddings produced by this generator
    pub fn dimension(&self) -> usize {
        self.model_type.dimension()
    }

    /// Get the model type being used
    pub fn model_type(&self) -> EmbeddingModel {
        self.model_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires model download
    async fn test_embedding_generation() {
        let generator = EmbeddingGenerator::new().await.unwrap();

        let text = "The quick brown fox jumps over the lazy dog";
        let embedding = generator.generate(text).await.unwrap();

        assert_eq!(embedding.len(), 384); // MiniLM dimension

        // Check that embedding is normalized (L2 norm ≈ 1.0)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    #[ignore] // Requires model download
    async fn test_batch_generation() {
        let generator = EmbeddingGenerator::new().await.unwrap();

        let texts = vec![
            "The cat sits on the mat",
            "A dog runs in the park",
            "Birds fly in the sky",
        ];

        let embeddings = generator.generate_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 384);
        }
    }

    #[tokio::test]
    #[ignore] // Requires model download
    async fn test_semantic_similarity() {
        let generator = EmbeddingGenerator::new().await.unwrap();

        let text1 = "The cat sits on the mat";
        let text2 = "A feline rests on the rug";
        let text3 = "The weather is sunny today";

        let emb1 = generator.generate(text1).await.unwrap();
        let emb2 = generator.generate(text2).await.unwrap();
        let emb3 = generator.generate(text3).await.unwrap();

        // Cosine similarity (dot product of normalized vectors)
        let sim_1_2: f32 = emb1.iter().zip(&emb2).map(|(a, b)| a * b).sum();
        let sim_1_3: f32 = emb1.iter().zip(&emb3).map(|(a, b)| a * b).sum();

        // Similar sentences should have higher similarity
        assert!(sim_1_2 > sim_1_3);
    }
}



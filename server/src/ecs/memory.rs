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

//! # Memory System
//!
//! This module implements a sophisticated AI memory system for NPCs and entities, inspired by
//! cognitive science principles and modern AI memory architectures.
//!
//! ## Architecture Overview
//!
//! The memory system is based on research from [arXiv:2512.12818v1](https://arxiv.org/html/2512.12818v1)
//! and implements four distinct memory types that mirror human cognitive processes:
//!
//! - **World Memory (Semantic)**: Static knowledge and facts about the game world
//! - **Experience Memory (Episodic)**: Personal history and past interactions
//! - **Opinion Memory (Inference)**: Weighted preferences and learned biases
//! - **Observation Memory (Working)**: Immediate sensory input and current context
//!
//! ## Key Features
//!
//! - **Semantic Search**: Vector embeddings enable similarity-based memory retrieval
//! - **Memory Consolidation**: Automatic compression of redundant memories
//! - **Contextual Reflection**: Generate responses based on relevant memories
//! - **Tag-based Filtering**: Flexible memory organization and retrieval
//! - **Entity Relationships**: Track connections between memories and entities
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use crate::ecs::memory::{MemoryResource, MemoryKind};
//!
//! let mut memory = MemoryResource::new();
//!
//! // Store a new experience
//! memory.retain(
//!     entity_id,
//!     MemoryKind::Experience,
//!     "Met a friendly merchant in the tavern",
//!     timestamp,
//!     Some("social interaction"),
//!     [("location", "tavern"), ("mood", "friendly")],
//!     [],
//!     ["social", "merchant"],
//! );
//!
//! // Recall similar memories
//! let memories = memory.recall(
//!     entity_id,
//!     "Who have I met recently?",
//!     [MemoryKind::Experience],
//!     ["social"],
//!     MemoryTagMode::Any,
//! );
//! ```

use crate::ecs::components::EntityId;
use crate::models::{EmbeddingGenerator, EmbeddingModel};
use chrono::{DateTime, Utc};
use metrics::{counter, gauge, histogram};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

/// Helper to convert MemoryKind to database string
impl MemoryKind {
    fn to_db_str(&self) -> &'static str {
        match self {
            MemoryKind::World => "World",
            MemoryKind::Experience => "Experience",
            MemoryKind::Opinion => "Opinion",
            MemoryKind::Observation => "Observation",
        }
    }

    fn from_db_str(s: &str) -> Result<Self, MemoryError> {
        match s {
            "World" => Ok(MemoryKind::World),
            "Experience" => Ok(MemoryKind::Experience),
            "Opinion" => Ok(MemoryKind::Opinion),
            "Observation" => Ok(MemoryKind::Observation),
            _ => Err(MemoryError::InvalidContent(format!(
                "Invalid memory kind: {}",
                s
            ))),
        }
    }
}

/// Errors that can occur during memory operations.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// The requested memory was not found.
    #[error("Memory not found: {0:?}")]
    NotFound(MemoryId),

    /// The specified entity was not found.
    #[error("Entity not found: {0:?}")]
    EntityNotFound(EntityId),

    /// Failed to generate embeddings for memory content.
    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),

    /// Database operation failed.
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// Entity has exceeded its memory limit.
    #[error("Memory limit exceeded for entity {0:?}")]
    MemoryLimitExceeded(EntityId),

    /// Invalid memory content (empty, too long, etc.).
    #[error("Invalid memory content: {0}")]
    InvalidContent(String),

    /// LLM operation failed during reflection or consolidation.
    #[error("LLM operation failed: {0}")]
    LlmError(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Result type for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Global memory resource that manages all entity memories in the ECS world.
///
/// This resource provides the primary interface for memory operations including
/// storage (retain), retrieval (recall), reasoning (reflect), and maintenance
/// (consolidate). It handles memory persistence and vector embeddings internally.
///
/// # Thread Safety
///
/// This resource is designed to be used within the Bevy ECS framework and follows
/// its threading model. Memory operations should be performed through systems.
/// Memory resource that manages all entity memories in the ECS world.
///
/// This resource provides database-backed memory storage with semantic search capabilities
/// and performance-optimized caching using Moka.
#[derive(Clone)]
pub struct MemoryResource {
    /// Database connection pool for persistence
    pool: PgPool,
    /// Configuration for memory operations
    config: MemoryConfig,
    /// Embedding generator for semantic search
    embedding_generator: Arc<RwLock<Option<EmbeddingGenerator>>>,
    /// Cache for frequently accessed memories
    memory_cache: Cache<MemoryId, Arc<MemoryNode>>,
    /// Cache for entity memory lists
    entity_memory_cache: Cache<EntityId, Arc<Vec<MemoryNode>>>,
    /// Cache for query embeddings
    embedding_cache: Cache<String, Arc<Vec<f32>>>,
}

impl std::fmt::Debug for MemoryResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryResource")
            .field("pool", &"PgPool")
            .field("config", &self.config)
            .field(
                "embedding_generator",
                &"Arc<RwLock<Option<EmbeddingGenerator>>>",
            )
            .field("memory_cache", &"Cache<MemoryId, Arc<MemoryNode>>")
            .field(
                "entity_memory_cache",
                &"Cache<EntityId, Arc<Vec<MemoryNode>>>",
            )
            .field("embedding_cache", &"Cache<String, Arc<Vec<f32>>>")
            .finish()
    }
}

/// Configuration parameters for memory operations.
///
/// Controls various aspects of memory processing including token limits,
/// embedding dimensions, and retrieval parameters.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum tokens for LLM responses during reflection operations.
    ///
    /// Default: 4096 tokens (~3000 words)
    pub max_tokens: usize,

    /// Maximum number of memories to retrieve during recall operations.
    ///
    /// Default: 10 memories
    pub max_recall_results: usize,

    /// Similarity threshold for memory retrieval (0.0 to 1.0).
    ///
    /// Memories with similarity scores below this threshold are filtered out.
    /// Default: 0.7 (70% similarity)
    pub similarity_threshold: f32,

    /// Maximum number of memories an entity can have.
    ///
    /// When this limit is reached, low-importance memories are pruned.
    /// Default: 1000 memories
    pub max_memories_per_entity: usize,

    /// Minimum importance threshold for keeping memories.
    ///
    /// Memories with importance below this value may be pruned.
    /// Default: 0.1 (10%)
    pub min_importance_threshold: f32,

    /// Memory count threshold that triggers automatic consolidation.
    ///
    /// When an entity reaches this percentage of max_memories_per_entity,
    /// automatic consolidation is triggered.
    /// Default: 800 (80% of max)
    pub consolidation_threshold: usize,

    /// Base decay rate for memory importance (per day).
    ///
    /// Higher values mean memories lose importance faster.
    /// Default: 0.01 (1% per day)
    pub base_decay_rate: f32,

    /// Cache configuration
    /// Maximum number of memories to cache in memory.
    ///
    /// Default: 10000 memories
    pub cache_max_capacity: u64,

    /// Time-to-live for cached memories.
    ///
    /// Default: 300 seconds (5 minutes)
    pub cache_ttl_seconds: u64,

    /// Time-to-idle for cached memories.
    ///
    /// Default: 60 seconds (1 minute)
    pub cache_tti_seconds: u64,

    /// Maximum number of query embeddings to cache.
    ///
    /// Default: 1000 embeddings
    pub embedding_cache_capacity: u64,

    /// Time-to-live for cached embeddings.
    ///
    /// Default: 600 seconds (10 minutes)
    pub embedding_cache_ttl_seconds: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            max_recall_results: 10,
            similarity_threshold: 0.7,
            max_memories_per_entity: 1000,
            min_importance_threshold: 0.1,
            consolidation_threshold: 800,
            base_decay_rate: 0.01,
            cache_max_capacity: 10000,
            cache_ttl_seconds: 300,
            cache_tti_seconds: 60,
            embedding_cache_capacity: 1000,
            embedding_cache_ttl_seconds: 600,
        }
    }
}

/// # Memory Concepts
///
/// In AI memory, these concepts differentiate how AI processes, stores, and uses data. The
/// World represents vast, static knowledge (semantic memory), while Experience is the stored
/// history of interactions (episodic memory). Observations are raw, immediate data points, which
/// the AI interprets into Opinions (subjective, weighted preferences or, more accurately,
/// probabilistic inferences).
///
/// ## Key Differences:
/// - World vs. Experience: World is general knowledge; Experience is personal history.
/// - Observation vs. Opinion: Observation is raw data; Opinion is the AI's probabilistic
///   interpretation or preference based on that data.
/// - Structure: The AI uses these to transform raw observations through its stored experiences
///   and world knowledge to deliver a tailored,, seemingly "opinionated" output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryKind {
    /// World (Semantic Memory): This is the AI's "long-term" knowledge base—the facts, definitions,
    /// and relationships it learned during training. It is static data that doesn't change unless
    /// the model is retrained. It represents the "external world" of information.
    World,
    /// Experience (Episodic Memory): These are "episodes" or logs of past interactions and
    /// conversations stored over time. This memory allows the AI to recall previous user
    /// preferences, context from earlier in a chat, or the outcome of a past task to improve
    /// future accuracy.
    Experience,
    /// Opinion (Inference/Weighting): AI does not have personal beliefs, but it generates
    /// "opinions" based on weighted probabilities and context. If trained on biased data or told
    /// to prefer one outcome, the AI will prioritize that, representing a "subjective" or
    /// "weighted" perspective driven by data patterns rather than feelings.
    Opinion,
    /// Observation (Input/Immediate Data): These are raw data points or prompts received in the
    /// present moment. They are the "current state" of input—for example, user input, sensor data,
    /// or a specific piece of text.
    Observation,
}

/// How to match tags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryTagMode {
    /// any - OR, includes untagged
    #[default]
    Any,
    /// all - AND, includes untagged
    All,
    /// any_strict - OR, excludes untagged
    AnyStrict,
    /// all_strict - AND, excludes untagged
    AllStrict,
}

/// Cache statistics for monitoring and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Number of entries in the memory cache
    pub memory_cache_size: u64,
    /// Number of entries in the entity memory list cache
    pub entity_cache_size: u64,
    /// Number of entries in the embedding cache
    pub embedding_cache_size: u64,
    /// Maximum capacity of the memory cache
    pub memory_cache_capacity: u64,
    /// Maximum capacity of the entity cache
    pub entity_cache_capacity: u64,
    /// Maximum capacity of the embedding cache
    pub embedding_cache_capacity: u64,
}

/// Item for batch memory retention.
#[derive(Debug, Clone)]
pub struct MemoryBatchItem {
    /// The entity that owns this memory
    pub entity_id: EntityId,
    /// Type of memory
    pub kind: MemoryKind,
    /// Memory content
    pub content: String,
    /// When the memory was created
    pub timestamp: DateTime<Utc>,
    /// Optional context
    pub context: Option<String>,
    /// Metadata key-value pairs
    pub metadata: BTreeMap<String, String>,
    /// Related entities
    pub entities: Vec<(EntityId, String)>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl MemoryBatchItem {
    /// Create a new batch item with minimal fields.
    pub fn new(entity_id: EntityId, kind: MemoryKind, content: String) -> Self {
        Self {
            entity_id,
            kind,
            content,
            timestamp: Utc::now(),
            context: None,
            metadata: BTreeMap::new(),
            entities: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add context to the batch item.
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Add metadata to the batch item.
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add an entity relationship to the batch item.
    pub fn with_entity(mut self, entity_id: EntityId, role: String) -> Self {
        self.entities.push((entity_id, role));
        self
    }

    /// Add a tag to the batch item.
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

/// Unique identifier for a memory node.
///
/// Wraps a UUID to provide type safety and prevent mixing memory IDs with other UUID types.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MemoryId(uuid::Uuid);

impl MemoryId {
    /// Creates a new random memory ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Creates a memory ID from an existing UUID.
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Returns a reference to the inner UUID.
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }

    /// Returns the inner UUID by value.
    pub fn uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

/// A single memory node representing a stored piece of information.
///
/// Memory nodes are the fundamental unit of the memory system. Each node contains:
/// - The actual memory content (text)
/// - Metadata for organization and retrieval
/// - Vector embeddings for semantic search
/// - Relationships to other memories and entities
///
/// # Memory Lifecycle
///
/// 1. **Creation**: Memories are created via `MemoryResource::retain()`
/// 2. **Storage**: Persisted with embeddings for semantic search
/// 3. **Retrieval**: Found via `recall()` using similarity search
/// 4. **Consolidation**: Merged or compressed to reduce redundancy
/// 5. **Deletion**: Removed when no longer relevant
///
/// # Examples
///
/// ```rust,ignore
/// let memory = MemoryNode {
///     memory_id: MemoryId::new(),
///     entity_id: npc_entity,
///     kind: MemoryKind::Experience,
///     content: "Fought a dragon and barely survived".to_string(),
///     timestamp: current_time,
///     context: Some("combat encounter".to_string()),
///     metadata: [("danger_level", "extreme")].into_iter().collect(),
///     related: BTreeMap::new(),
///     entities: [(dragon_entity, "dragon".to_string())].into_iter().collect(),
///     embedding: vec![0.1, 0.2, ...], // Generated by embedding model
///     tags: ["combat", "dragon", "survival"].into_iter().collect(),
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryNode {
    /// Unique identifier for this memory node.
    pub memory_id: MemoryId,

    /// The entity that owns this memory (NPC, player, etc.).
    pub entity_id: EntityId,

    /// Classification of the memory type (World, Experience, Opinion, Observation).
    pub kind: MemoryKind,

    /// The actual text content of the memory.
    ///
    /// This should be a natural language description of the memory, suitable for
    /// both human reading and LLM processing.
    pub content: String,

    /// When this memory was created or when the event occurred.
    pub timestamp: DateTime<Utc>,

    /// When this memory was last accessed (for importance calculation).
    pub last_accessed: DateTime<Utc>,

    /// Number of times this memory has been accessed.
    ///
    /// Used to boost importance of frequently recalled memories.
    pub access_count: u32,

    /// Base importance score (0.0 to 1.0).
    ///
    /// Higher values indicate more significant memories that should be retained longer.
    /// This value is set at creation and can be modified.
    pub importance: f32,

    /// Rate at which this memory's importance decays over time.
    ///
    /// Higher values mean faster decay. Typically between 0.001 and 0.1.
    pub decay_rate: f32,

    /// Additional context about the memory's circumstances.
    ///
    /// Examples: "during combat", "in the tavern", "while trading"
    pub context: Option<String>,

    /// User-defined key-value metadata for custom filtering and organization.
    ///
    /// Examples: location, mood, importance_level, danger_rating
    pub metadata: BTreeMap<String, String>,

    /// References to related memories with relationship descriptions.
    ///
    /// Key: Related memory ID
    /// Value: Description of the relationship (e.g., "caused by", "led to", "similar to")
    pub related: BTreeMap<MemoryId, String>,

    /// Entities mentioned or involved in this memory.
    ///
    /// Key: Entity ID
    /// Value: Role or description (e.g., "attacker", "merchant", "witness")
    pub entities: BTreeMap<EntityId, String>,

    /// Vector embedding of the memory content for semantic similarity search.
    ///
    /// Generated by an embedding model (e.g., text-embedding-ada-002, sentence-transformers).
    /// Typical dimensions: 384, 768, or 1536 depending on the model.
    pub embedding: Vec<f32>,

    /// Tags for categorical filtering during recall and reflection.
    ///
    /// Examples: "combat", "social", "quest", "merchant", "danger"
    pub tags: BTreeSet<String>,
}

impl MemoryNode {
    /// Calculate the current importance of this memory based on age and access patterns.
    ///
    /// Importance decays exponentially over time but is boosted by frequent access.
    /// The formula is: `base_importance * decay_factor * access_boost`
    ///
    /// # Arguments
    ///
    /// * `now` - Current timestamp for calculating age
    ///
    /// # Returns
    ///
    /// Current importance score (0.0 to ~1.5, though typically 0.0 to 1.0)
    pub fn calculate_current_importance(&self, now: DateTime<Utc>) -> f32 {
        let age_seconds = (now - self.timestamp).num_seconds().max(0) as f32;
        let days = age_seconds / 86400.0;

        // Exponential decay based on age
        let decay_factor = (-self.decay_rate * days).exp();

        // Boost based on access count (diminishing returns)
        let access_boost = 1.0 + (self.access_count as f32).ln().max(0.0) * 0.1;

        // Recency boost if accessed recently
        let recency_seconds = (now - self.last_accessed).num_seconds().max(0) as f32;
        let recency_boost = if recency_seconds < 3600.0 {
            1.1 // 10% boost if accessed in last hour
        } else {
            1.0
        };

        self.importance * decay_factor * access_boost * recency_boost
    }

    /// Mark this memory as accessed, updating access count and timestamp.
    pub fn mark_accessed(&mut self, now: DateTime<Utc>) {
        self.last_accessed = now;
        self.access_count = self.access_count.saturating_add(1);
    }

    /// Check if this memory should be pruned based on current importance.
    pub fn should_prune(&self, now: DateTime<Utc>, threshold: f32) -> bool {
        self.calculate_current_importance(now) < threshold
    }
}

/// Calculate cosine similarity between two vectors.
///
/// Returns a value between -1.0 and 1.0, where 1.0 means identical direction.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

impl MemoryResource {
    /// Creates a new memory resource with a database connection pool.
    ///
    /// Uses default configuration and MiniLM embedding model.
    /// The embedding generator is lazily initialized on first use.
    pub fn new(pool: PgPool) -> Self {
        Self::with_config(pool, MemoryConfig::default())
    }

    /// Creates a new memory resource with custom configuration.
    ///
    /// The embedding generator is lazily initialized on first use with the MiniLM model.
    /// Initializes Moka caches for performance optimization.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool for persistence
    /// * `config` - Custom memory configuration
    pub fn with_config(pool: PgPool, config: MemoryConfig) -> Self {
        // Initialize memory cache with TTL and TTI
        let memory_cache = Cache::builder()
            .max_capacity(config.cache_max_capacity)
            .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
            .time_to_idle(Duration::from_secs(config.cache_tti_seconds))
            .build();

        // Initialize entity memory list cache
        let entity_memory_cache = Cache::builder()
            .max_capacity(config.cache_max_capacity / 10) // Fewer entities than memories
            .time_to_live(Duration::from_secs(config.cache_ttl_seconds))
            .time_to_idle(Duration::from_secs(config.cache_tti_seconds))
            .build();

        // Initialize embedding cache
        let embedding_cache = Cache::builder()
            .max_capacity(config.embedding_cache_capacity)
            .time_to_live(Duration::from_secs(config.embedding_cache_ttl_seconds))
            .build();

        info!(
            memory_cache_capacity = config.cache_max_capacity,
            entity_cache_capacity = config.cache_max_capacity / 10,
            embedding_cache_capacity = config.embedding_cache_capacity,
            "Memory caches initialized"
        );

        Self {
            pool,
            config,
            embedding_generator: Arc::new(RwLock::new(None)),
            memory_cache,
            entity_memory_cache,
            embedding_cache,
        }
    }

    /// Ensure the embedding generator is initialized.
    ///
    /// This is called automatically by methods that need embeddings.
    /// Uses lazy initialization to avoid loading the model until needed.
    async fn ensure_embedding_generator(&self) -> MemoryResult<()> {
        let guard = self.embedding_generator.read().await;
        if guard.is_some() {
            return Ok(());
        }
        drop(guard);

        // Initialize the generator
        let generator = EmbeddingGenerator::with_model(EmbeddingModel::MiniLM)
            .await
            .map_err(|e| MemoryError::EmbeddingError(e.to_string()))?;

        let mut guard = self.embedding_generator.write().await;
        *guard = Some(generator);

        info!("Embedding generator initialized with MiniLM model");
        Ok(())
    }

    /// Generate an embedding for text content with caching.
    ///
    /// Automatically initializes the embedding generator if needed.
    /// Checks cache first to avoid redundant embedding generation.
    async fn generate_embedding(&self, text: &str) -> MemoryResult<Vec<f32>> {
        // Check cache first
        if let Some(cached) = self.embedding_cache.get(text).await {
            counter!("memory.cache.hits", "type" => "embedding").increment(1);
            debug!("Embedding cache hit for query");
            return Ok((*cached).clone());
        }

        counter!("memory.cache.misses", "type" => "embedding").increment(1);

        self.ensure_embedding_generator().await?;

        let guard = self.embedding_generator.read().await;
        let generator = guard
            .as_ref()
            .ok_or_else(|| MemoryError::EmbeddingError("Generator not initialized".to_string()))?;

        let embedding = generator
            .generate(text)
            .await
            .map_err(|e| MemoryError::EmbeddingError(e.to_string()))?;

        // Cache the embedding
        self.embedding_cache
            .insert(text.to_string(), Arc::new(embedding.clone()))
            .await;

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts in batch.
    ///
    /// More efficient than calling generate_embedding multiple times.
    /// Uses caching for individual texts.
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> MemoryResult<Vec<Vec<f32>>> {
        let start = std::time::Instant::now();

        let mut results = Vec::with_capacity(texts.len());
        let mut uncached_texts = Vec::new();
        let mut uncached_indices = Vec::new();

        // Check cache for each text
        for (i, text) in texts.iter().enumerate() {
            if let Some(cached) = self.embedding_cache.get(text).await {
                counter!("memory.cache.hits", "type" => "embedding").increment(1);
                results.push(Some((*cached).clone()));
            } else {
                counter!("memory.cache.misses", "type" => "embedding").increment(1);
                results.push(None);
                uncached_texts.push(text.as_str());
                uncached_indices.push(i);
            }
        }

        // Generate embeddings for uncached texts
        if !uncached_texts.is_empty() {
            self.ensure_embedding_generator().await?;

            let guard = self.embedding_generator.read().await;
            let generator = guard.as_ref().ok_or_else(|| {
                MemoryError::EmbeddingError("Generator not initialized".to_string())
            })?;

            let embeddings = generator
                .generate_batch(&uncached_texts)
                .await
                .map_err(|e| MemoryError::EmbeddingError(e.to_string()))?;

            // Cache and insert results
            for (idx, embedding) in uncached_indices.iter().zip(embeddings.iter()) {
                let text = &texts[*idx];
                self.embedding_cache
                    .insert(text.clone(), Arc::new(embedding.clone()))
                    .await;
                results[*idx] = Some(embedding.clone());
            }
        }

        let duration = start.elapsed().as_secs_f64();
        histogram!("memory.batch.embedding.duration").record(duration);
        counter!("memory.batch.embedding.count").increment(texts.len() as u64);

        Ok(results.into_iter().map(|r| r.unwrap()).collect())
    }

    /// Get a reference to the database pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get the current configuration.
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }

    /// Count the number of memories for an entity.
    pub async fn count_memories(&self, entity_id: EntityId) -> MemoryResult<usize> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM wyldlands.entity_memory WHERE entity_id = $1")
                .bind(entity_id.uuid())
                .fetch_one(&self.pool)
                .await?;

        Ok(count as usize)
    }

    /// Prune low-importance memories for an entity.
    ///
    /// Removes memories whose current importance falls below the configured threshold,
    /// while ensuring a minimum number of memories are retained.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity whose memories to prune
    /// * `min_keep` - Minimum number of memories to keep regardless of importance
    ///
    /// # Returns
    ///
    /// Returns the number of memories pruned.
    pub async fn prune_low_importance_memories(
        &mut self,
        entity_id: EntityId,
        min_keep: usize,
    ) -> MemoryResult<usize> {
        let now = Utc::now();

        // Get all memories for the entity
        let memories = self.list_memories(entity_id).await?;

        if memories.len() <= min_keep {
            return Ok(0); // Don't prune if at or below minimum
        }

        // Calculate current importance for each memory
        let mut memory_importance: Vec<(MemoryId, f32)> = memories
            .iter()
            .map(|m| (m.memory_id.clone(), m.calculate_current_importance(now)))
            .collect();

        // Sort by importance (lowest first)
        memory_importance
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Determine how many to prune
        let can_prune = memories.len() - min_keep;
        let mut pruned = 0;

        // Prune memories below threshold, but respect min_keep
        for (memory_id, importance) in memory_importance.iter().take(can_prune) {
            if *importance < self.config.min_importance_threshold {
                self.delete_memory(memory_id.clone()).await?;
                pruned += 1;
            }
        }

        Ok(pruned)
    }

    /// Automatically consolidate memories if the entity is approaching its memory limit.
    ///
    /// Checks if the entity has exceeded the consolidation threshold and triggers
    /// consolidation if needed. Uses LLM if available for intelligent summarization.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity to check
    /// * `llm_manager` - Optional LLM manager for intelligent consolidation
    ///
    /// # Returns
    ///
    /// Returns `true` if consolidation was triggered, `false` otherwise.
    pub async fn auto_consolidate_if_needed(
        &mut self,
        entity_id: EntityId,
        llm_manager: Option<&crate::models::ModelManager>,
    ) -> MemoryResult<bool> {
        let count = self.count_memories(entity_id).await?;
        if count >= self.config.consolidation_threshold {
            info!(
                entity_id = %entity_id,
                memory_count = count,
                threshold = self.config.consolidation_threshold,
                "Auto-consolidation triggered"
            );
            self.consolidate(
                entity_id,
                "",
                None,
                [],
                MemoryTagMode::Any,
                llm_manager,
                None, // Use default similarity threshold
            )
            .await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Store a new memory for an entity.
    ///
    /// This is the primary method for creating memories. The memory content will be
    /// automatically embedded using the configured embedding model for semantic search.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity that owns this memory (NPC, player, etc.)
    /// * `kind` - The type of memory (World, Experience, Opinion, Observation)
    /// * `content` - Natural language description of the memory
    /// * `timestamp` - When the memory was created or event occurred
    /// * `context` - Optional situational context (e.g., "during combat")
    /// * `metadata` - Key-value pairs for custom organization
    /// * `entities` - Other entities involved in this memory
    /// * `tags` - Categorical tags for filtering
    ///
    /// # Returns
    ///
    /// Returns the `MemoryId` of the newly created memory.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// memory.retain(
    ///     npc_entity,
    ///     MemoryKind::Experience,
    ///     "Defeated a goblin in the forest",
    ///     current_time,
    ///     Some("combat"),
    ///     [("location", "forest"), ("outcome", "victory")],
    ///     [(goblin_entity, "enemy")],
    ///     ["combat", "goblin", "forest"],
    /// );
    /// ```
    #[instrument(skip(self, metadata, entities, tags), fields(entity_id = %entity_id, kind = ?kind))]
    pub async fn retain<'a>(
        &mut self,
        entity_id: EntityId,
        kind: MemoryKind,
        content: &str,
        timestamp: DateTime<Utc>,
        context: Option<&str>,
        metadata: impl IntoIterator<Item = (&'a str, &'a str)>,
        entities: impl IntoIterator<Item = (EntityId, &'a str)>,
        tags: impl IntoIterator<Item = &'a str>,
    ) -> MemoryResult<MemoryId> {
        let start = std::time::Instant::now();
        // Validate content
        if content.trim().is_empty() {
            return Err(MemoryError::InvalidContent(
                "Content cannot be empty".to_string(),
            ));
        }

        if content.len() > 10000 {
            return Err(MemoryError::InvalidContent(
                "Content exceeds maximum length of 10000 characters".to_string(),
            ));
        }

        // Check memory limit
        let count = self.count_memories(entity_id).await?;
        if count >= self.config.max_memories_per_entity {
            return Err(MemoryError::MemoryLimitExceeded(entity_id));
        }

        // Create memory ID
        let memory_id = MemoryId::new();

        // Convert metadata to JSON
        let metadata_map: BTreeMap<String, String> = metadata
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let metadata_json = serde_json::to_value(&metadata_map)?;

        // Convert tags to array
        let tags_vec: Vec<String> = tags.into_iter().map(|s| s.to_string()).collect();

        // Convert kind to string
        let kind_str = match kind {
            MemoryKind::World => "World",
            MemoryKind::Experience => "Experience",
            MemoryKind::Opinion => "Opinion",
            MemoryKind::Observation => "Observation",
        };

        // Generate embedding for semantic search
        debug!("Generating embedding for memory content");
        let embedding = self.generate_embedding(content).await?;
        counter!("memory.embeddings.generated").increment(1);

        // Insert memory into database
        sqlx::query(
            r#"
            INSERT INTO wyldlands.entity_memory
            (memory_id, entity_id, kind, content, timestamp, last_accessed,
             access_count, importance, decay_rate, context, metadata, related, embedding, tags)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(memory_id.uuid())
        .bind(entity_id.uuid())
        .bind(kind_str)
        .bind(content)
        .bind(timestamp)
        .bind(timestamp) // last_accessed = timestamp initially
        .bind(0i32) // access_count = 0
        .bind(0.5f32) // default importance
        .bind(self.config.base_decay_rate)
        .bind(context)
        .bind(metadata_json)
        .bind(serde_json::json!({})) // empty related initially
        .bind(embedding)
        .bind(&tags_vec)
        .execute(&self.pool)
        .await?;

        // Insert entity relationships
        for (involved_entity_id, role) in entities {
            sqlx::query(
                r#"
                INSERT INTO wyldlands.entity_memory_entities
                (memory_id, involved_entity_id, role)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(memory_id.uuid())
            .bind(involved_entity_id.uuid())
            .bind(role)
            .execute(&self.pool)
            .await?;
        }

        // Invalidate entity cache since we added a new memory
        self.invalidate_entity_cache(&entity_id).await;

        // TODO: Auto-consolidate if needed
        // self.auto_consolidate_if_needed(entity_id).await?;

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        counter!("memory.operations.total", "operation" => "retain").increment(1);
        histogram!("memory.operation.duration", "operation" => "retain").record(duration);
        counter!("memory.retentions", "kind" => kind_str.to_string()).increment(1);
        histogram!("memory.importance", "kind" => kind_str.to_string()).record(0.5); // default importance

        // Update cache stats gauge
        let stats = self.cache_stats();
        gauge!("memory.cache.size", "type" => "memory").set(stats.memory_cache_size as f64);
        gauge!("memory.cache.size", "type" => "entity").set(stats.entity_cache_size as f64);
        gauge!("memory.cache.size", "type" => "embedding").set(stats.embedding_cache_size as f64);

        info!(
            memory_id = ?memory_id,
            kind = kind_str,
            duration_ms = duration * 1000.0,
            "Memory retained successfully"
        );

        Ok(memory_id)
    }

    /// Retain multiple memories in a batch operation.
    ///
    /// More efficient than calling retain multiple times as it:
    /// - Generates embeddings in batch
    /// - Uses a single transaction for all inserts
    /// - Invalidates caches once at the end
    ///
    /// # Arguments
    ///
    /// * `memories` - Vector of memory data to retain
    ///
    /// # Returns
    ///
    /// Returns a vector of MemoryIds for the created memories.
    pub async fn retain_batch(
        &mut self,
        memories: Vec<MemoryBatchItem>,
    ) -> MemoryResult<Vec<MemoryId>> {
        let start = std::time::Instant::now();

        if memories.is_empty() {
            return Ok(Vec::new());
        }

        info!(count = memories.len(), "Starting batch memory retention");

        // Validate all memories first
        for item in &memories {
            if item.content.trim().is_empty() {
                return Err(MemoryError::InvalidContent(
                    "Content cannot be empty".to_string(),
                ));
            }
            if item.content.len() > 10000 {
                return Err(MemoryError::InvalidContent(
                    "Content exceeds maximum length of 10000 characters".to_string(),
                ));
            }
        }

        // Generate embeddings in batch
        let contents: Vec<String> = memories.iter().map(|m| m.content.clone()).collect();
        let embeddings = self.generate_embeddings_batch(&contents).await?;

        // Start transaction
        let mut tx = self.pool.begin().await?;
        let mut memory_ids = Vec::with_capacity(memories.len());
        let mut affected_entities = std::collections::HashSet::new();

        for (item, embedding) in memories.iter().zip(embeddings.iter()) {
            let memory_id = MemoryId::new();
            memory_ids.push(memory_id.clone());
            affected_entities.insert(item.entity_id);

            // Convert metadata to JSON
            let metadata_json = serde_json::to_value(&item.metadata)?;

            // Convert kind to string
            let kind_str = item.kind.to_db_str();

            // Insert memory
            sqlx::query(
                r#"
                INSERT INTO wyldlands.entity_memory
                (memory_id, entity_id, kind, content, timestamp, last_accessed,
                 access_count, importance, decay_rate, context, metadata, related, embedding, tags)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                "#,
            )
            .bind(memory_id.uuid())
            .bind(item.entity_id.uuid())
            .bind(kind_str)
            .bind(&item.content)
            .bind(item.timestamp)
            .bind(item.timestamp)
            .bind(0i32)
            .bind(0.5f32)
            .bind(self.config.base_decay_rate)
            .bind(&item.context)
            .bind(metadata_json)
            .bind(serde_json::json!({}))
            .bind(embedding)
            .bind(&item.tags)
            .execute(&mut *tx)
            .await?;

            // Insert entity relationships
            for (involved_entity_id, role) in &item.entities {
                sqlx::query(
                    r#"
                    INSERT INTO wyldlands.entity_memory_entities
                    (memory_id, involved_entity_id, role)
                    VALUES ($1, $2, $3)
                    "#,
                )
                .bind(memory_id.uuid())
                .bind(involved_entity_id.uuid())
                .bind(role)
                .execute(&mut *tx)
                .await?;
            }

            counter!("memory.retentions", "kind" => kind_str.to_string()).increment(1);
        }

        // Commit transaction
        tx.commit().await?;

        // Invalidate caches for all affected entities
        for entity_id in affected_entities {
            self.invalidate_entity_cache(&entity_id).await;
        }

        let duration = start.elapsed().as_secs_f64();
        counter!("memory.operations.total", "operation" => "retain_batch").increment(1);
        histogram!("memory.operation.duration", "operation" => "retain_batch").record(duration);
        counter!("memory.batch.retain.count").increment(memories.len() as u64);

        info!(
            count = memories.len(),
            duration_ms = duration * 1000.0,
            "Batch memory retention completed"
        );

        Ok(memory_ids)
    }

    /// Retrieve memories using semantic similarity search.
    ///
    /// Searches an entity's memories using vector similarity to find the most relevant
    /// memories for a given query. Results are ranked by semantic similarity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity whose memories to search
    /// * `query` - Natural language search query
    /// * `kinds` - Filter by memory types (empty = all types)
    /// * `tags` - Filter by tags (empty = all tags)
    /// * `tags_match` - How to match tags (Any, All, AnyStrict, AllStrict)
    ///
    /// # Returns
    ///
    /// Returns a vector of `MemoryNode`s sorted by relevance (highest first).
    /// The number of results is limited by `MemoryConfig::max_recall_results`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Find combat-related memories
    /// let memories = memory.recall(
    ///     npc_entity,
    ///     "What fights have I been in?",
    ///     [MemoryKind::Experience],
    ///     ["combat"],
    ///     MemoryTagMode::Any,
    /// );
    /// ```
    #[instrument(skip(self, kinds, tags), fields(entity_id = %entity_id, query = %query, tags_match = ?tags_match))]
    pub async fn recall<'a>(
        &self,
        entity_id: EntityId,
        query: &str,
        kinds: impl IntoIterator<Item = MemoryKind>,
        tags: impl IntoIterator<Item = &'a str>,
        tags_match: MemoryTagMode,
    ) -> MemoryResult<Vec<MemoryNode>> {
        let start = std::time::Instant::now();
        let now = Utc::now();

        // Convert iterators to collections for filtering
        let kinds_vec: Vec<MemoryKind> = kinds.into_iter().collect();
        let tags_vec: Vec<String> = tags.into_iter().map(|s| s.to_string()).collect();

        // Get all memories for the entity
        let mut memories = self.list_memories(entity_id).await?;

        // Filter by kinds if specified
        if !kinds_vec.is_empty() {
            memories.retain(|m| kinds_vec.contains(&m.kind));
        }

        // Filter by tags based on match mode
        if !tags_vec.is_empty() {
            memories.retain(|m| {
                let has_tags = !m.tags.is_empty();
                match tags_match {
                    MemoryTagMode::Any => {
                        // Include if has any of the tags OR has no tags
                        !has_tags || tags_vec.iter().any(|t| m.tags.contains(t))
                    }
                    MemoryTagMode::All => {
                        // Include if has all tags OR has no tags
                        !has_tags || tags_vec.iter().all(|t| m.tags.contains(t))
                    }
                    MemoryTagMode::AnyStrict => {
                        // Include only if has tags AND has any of the specified tags
                        has_tags && tags_vec.iter().any(|t| m.tags.contains(t))
                    }
                    MemoryTagMode::AllStrict => {
                        // Include only if has tags AND has all specified tags
                        has_tags && tags_vec.iter().all(|t| m.tags.contains(t))
                    }
                }
            });
        }

        // Generate query embedding for semantic similarity search
        debug!("Generating query embedding for semantic search");
        let query_embedding = self.generate_embedding(query).await?;
        counter!("memory.embeddings.generated").increment(1);

        // Score memories using vector similarity + importance + recency
        let mut scored_memories: Vec<(MemoryNode, f32)> = memories
            .into_iter()
            .map(|m| {
                // Calculate cosine similarity between query and memory embeddings
                let similarity =
                    if !m.embedding.is_empty() && m.embedding.len() == query_embedding.len() {
                        cosine_similarity(&query_embedding, &m.embedding)
                    } else {
                        // Fallback to text matching if embedding is missing or wrong size
                        let query_lower = query.to_lowercase();
                        let content_lower = m.content.to_lowercase();
                        if content_lower.contains(&query_lower) {
                            0.8
                        } else {
                            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
                            let matches = query_words
                                .iter()
                                .filter(|word| content_lower.contains(*word))
                                .count();
                            (matches as f32 / query_words.len().max(1) as f32) * 0.6
                        }
                    };

                // Calculate current importance with decay
                let importance = m.calculate_current_importance(now);

                // Recency boost (memories from last hour get a boost)
                let recency_boost = if (now - m.timestamp).num_hours() < 1 {
                    1.15
                } else {
                    1.0
                };

                // Combined score: 60% similarity, 30% importance, 10% recency
                let score = (similarity * 0.6 + importance * 0.3 + 0.1) * recency_boost;

                (m, score)
            })
            .collect();

        // Sort by score (highest first)
        scored_memories.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N results
        let mut results: Vec<MemoryNode> = scored_memories
            .into_iter()
            .take(self.config.max_recall_results)
            .map(|(m, _)| m)
            .collect();

        // Mark memories as accessed (update in database)
        for memory in &mut results {
            memory.mark_accessed(now);

            // Update in database
            sqlx::query(
                r#"
                UPDATE wyldlands.entity_memory
                SET last_accessed = $1, access_count = access_count + 1
                WHERE memory_id = $2
                "#,
            )
            .bind(now)
            .bind(memory.memory_id.uuid())
            .execute(&self.pool)
            .await?;
        }

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        let result_count = results.len();
        counter!("memory.operations.total", "operation" => "recall").increment(1);
        histogram!("memory.operation.duration", "operation" => "recall").record(duration);
        histogram!("memory.recall.results.count", "filter_mode" => format!("{:?}", tags_match))
            .record(result_count as f64);

        debug!(
            count = result_count,
            duration_ms = duration * 1000.0,
            "Memory recall completed"
        );

        Ok(results)
    }

    /// Generate a contextual response based on relevant memories.
    ///
    /// Uses an LLM to generate a natural language response to a query, informed by
    /// the entity's relevant memories. This is useful for NPC dialogue, decision-making,
    /// and contextual awareness.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity to query
    /// * `query` - The question or prompt
    /// * `context` - Additional context to include in the prompt
    /// * `tags` - Filter memories by tags
    /// * `tags_match` - How to match tags
    /// * `llm_manager` - Optional LLM manager for generating responses. If None, returns formatted memories.
    /// * `model` - Optional model name to use (defaults to "gpt-4")
    ///
    /// # Returns
    ///
    /// Returns a natural language response (LLM-generated if manager provided, formatted otherwise)
    /// along with the memories that were used to inform the response.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // With LLM integration
    /// let (response, used_memories) = memory.reflect(
    ///     npc_entity,
    ///     "What do you think about the player?",
    ///     Some("The player is asking about your opinion"),
    ///     ["social", "player"],
    ///     MemoryTagMode::Any,
    ///     Some(&llm_manager),
    ///     Some("gpt-4"),
    /// ).await?;
    ///
    /// // Without LLM (returns formatted memories)
    /// let (response, used_memories) = memory.reflect(
    ///     npc_entity,
    ///     "What do you know about dragons?",
    ///     None,
    ///     ["dragon"],
    ///     MemoryTagMode::Any,
    ///     None,
    ///     None,
    /// ).await?;
    /// ```
    pub async fn reflect<'a>(
        &self,
        entity_id: EntityId,
        query: &str,
        context: Option<&str>,
        tags: impl IntoIterator<Item = &'a str>,
        tags_match: MemoryTagMode,
        llm_manager: Option<&crate::models::ModelManager>,
        model: Option<&str>,
    ) -> MemoryResult<(String, Vec<MemoryNode>)> {
        // Recall relevant memories using all memory kinds
        let memories = self
            .recall(
                entity_id,
                query,
                [
                    MemoryKind::World,
                    MemoryKind::Experience,
                    MemoryKind::Opinion,
                    MemoryKind::Observation,
                ],
                tags,
                tags_match,
            )
            .await?;

        // Build memory context string
        let mut memory_context = String::new();

        if !memories.is_empty() {
            memory_context.push_str("Relevant memories:\n");
            for (i, memory) in memories.iter().enumerate() {
                memory_context.push_str(&format!(
                    "{}. [{}] {}\n",
                    i + 1,
                    match memory.kind {
                        MemoryKind::World => "World Knowledge",
                        MemoryKind::Experience => "Experience",
                        MemoryKind::Opinion => "Opinion",
                        MemoryKind::Observation => "Observation",
                    },
                    memory.content
                ));
                if let Some(ctx) = &memory.context {
                    memory_context.push_str(&format!("   Context: {}\n", ctx));
                }
            }
        }

        // If no LLM manager provided, return formatted memories
        let Some(llm) = llm_manager else {
            let mut response = String::new();
            if let Some(ctx) = context {
                response.push_str(&format!("Context: {}\n\n", ctx));
            }
            response.push_str(&memory_context);
            response.push_str(&format!("\nQuery: {}\n\n", query));
            response.push_str(&format!(
                "Based on {} relevant memories, here's my response: [LLM integration not available]",
                memories.len()
            ));
            return Ok((response, memories));
        };

        // Build LLM request with memory context
        let system_message = format!(
            "You are responding based on the following memories:\n\n{}\n\n\
            Use these memories to provide a contextual, natural response. \
            Speak in first person as if you are the entity with these memories. \
            Be concise and relevant to the query.",
            memory_context
        );

        let mut user_message = String::new();
        if let Some(ctx) = context {
            user_message.push_str(&format!("Context: {}\n\n", ctx));
        }
        user_message.push_str(&format!("Query: {}", query));

        let model_name = model.unwrap_or("gpt-4");
        let request = crate::models::LLMRequest::new(model_name)
            .with_message(crate::models::LLMMessage::system(system_message))
            .with_message(crate::models::LLMMessage::user(user_message))
            .with_temperature(0.7)
            .with_max_tokens(self.config.max_tokens as u32);

        // Send request to LLM
        let response = llm
            .complete(request)
            .await
            .map_err(|e| MemoryError::LlmError(format!("LLM request failed: {}", e)))?;

        // Note: memories are already marked as accessed by recall()

        Ok((response.content, memories))
    }

    /// Consolidate redundant or similar memories to improve efficiency.
    ///
    /// Over time, entities accumulate many similar memories. This method uses vector
    /// embeddings to identify similar memories and an LLM to merge them into more
    /// concise representations, reducing storage and improving retrieval performance.
    ///
    /// The consolidation process:
    /// 1. Retrieves memories matching the criteria
    /// 2. Groups similar memories using embedding cosine similarity
    /// 3. Uses LLM to create intelligent summaries of each group
    /// 4. Replaces the group with a single consolidated memory
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity whose memories to consolidate
    /// * `query` - Optional focus area for consolidation
    /// * `context` - Additional context for the consolidation process
    /// * `tags` - Only consolidate memories with these tags
    /// * `tags_match` - How to match tags
    /// * `llm_manager` - Optional LLM manager for intelligent summarization
    /// * `similarity_threshold` - Minimum similarity (0.0-1.0) to group memories (default: 0.75)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Consolidate all combat memories with LLM
    /// memory.consolidate(
    ///     npc_entity,
    ///     "combat experiences",
    ///     None,
    ///     ["combat"],
    ///     MemoryTagMode::Any,
    ///     Some(&llm_manager),
    ///     Some(0.75),
    /// ).await?;
    /// ```
    #[instrument(skip(self, tags, llm_manager), fields(entity_id = %entity_id, query = %query))]
    pub async fn consolidate<'a>(
        &mut self,
        entity_id: EntityId,
        query: &str,
        context: Option<&str>,
        tags: impl IntoIterator<Item = &'a str>,
        tags_match: MemoryTagMode,
        llm_manager: Option<&crate::models::ModelManager>,
        similarity_threshold: Option<f32>,
    ) -> MemoryResult<usize> {
        let start = std::time::Instant::now();
        let threshold = similarity_threshold.unwrap_or(0.75);
        info!("Starting memory consolidation for entity");

        // Recall memories matching criteria
        let memories = self
            .recall(
                entity_id,
                query,
                [MemoryKind::Experience, MemoryKind::Observation], // Focus on consolidating experiences and observations
                tags,
                tags_match,
            )
            .await?;

        if memories.len() < 2 {
            info!("Not enough memories to consolidate (need at least 2)");
            return Ok(0);
        }

        debug!(
            memory_count = memories.len(),
            "Retrieved memories for consolidation"
        );

        // Group similar memories using embedding-based similarity
        let mut groups: Vec<Vec<MemoryNode>> = Vec::new();

        for memory in memories {
            // Skip memories with no embedding
            if memory.embedding.is_empty() {
                debug!(memory_id = ?memory.memory_id, "Skipping memory with no embedding");
                continue;
            }

            let mut best_group_idx: Option<usize> = None;
            let mut best_similarity: f32 = threshold;

            // Find the most similar group
            for (idx, group) in groups.iter().enumerate() {
                if let Some(first) = group.first() {
                    // Only group memories of the same kind
                    if memory.kind != first.kind {
                        continue;
                    }

                    // Calculate average similarity to group
                    let similarities: Vec<f32> = group
                        .iter()
                        .filter(|m| !m.embedding.is_empty())
                        .map(|m| cosine_similarity(&memory.embedding, &m.embedding))
                        .collect();

                    if !similarities.is_empty() {
                        let avg_similarity =
                            similarities.iter().sum::<f32>() / similarities.len() as f32;

                        if avg_similarity > best_similarity {
                            best_similarity = avg_similarity;
                            best_group_idx = Some(idx);
                        }
                    }
                }
            }

            // Add to best group or create new group
            if let Some(idx) = best_group_idx {
                debug!(
                    memory_id = ?memory.memory_id,
                    group_idx = idx,
                    similarity = best_similarity,
                    "Adding memory to existing group"
                );
                groups[idx].push(memory);
            } else {
                debug!(memory_id = ?memory.memory_id, "Creating new group for memory");
                groups.push(vec![memory]);
            }
        }

        info!(group_count = groups.len(), "Grouped memories by similarity");

        let mut consolidated_count = 0;
        let mut groups_consolidated = 0;

        // Consolidate each group with 2+ memories
        for group in groups {
            if group.len() < 2 {
                continue;
            }

            debug!(
                group_size = group.len(),
                kind = ?group[0].kind,
                "Consolidating memory group"
            );

            // Create consolidated memory
            let first = &group[0];
            let all_tags: BTreeSet<String> =
                group.iter().flat_map(|m| m.tags.iter().cloned()).collect();

            // Use LLM to create intelligent summary if available
            let consolidated_content = if let Some(llm) = llm_manager {
                // Build prompt for LLM summarization
                let memories_text = group
                    .iter()
                    .enumerate()
                    .map(|(i, m)| format!("{}. {}", i + 1, m.content))
                    .collect::<Vec<_>>()
                    .join("\n");

                let system_prompt = format!(
                    "You are consolidating {} similar memories into a single, concise memory. \
                    Preserve all important details and facts while removing redundancy. \
                    The consolidated memory should be clear, factual, and comprehensive.",
                    group.len()
                );

                let user_prompt = format!(
                    "Consolidate these {} related memories into a single memory:\n\n{}\n\n\
                    Create a single, concise memory that captures all important information. \
                    Keep it under 200 words.",
                    group.len(),
                    memories_text
                );

                let request = crate::models::LLMRequest::new("gpt-4")
                    .with_message(crate::models::LLMMessage::system(system_prompt))
                    .with_message(crate::models::LLMMessage::user(user_prompt))
                    .with_temperature(0.3) // Lower temperature for factual consolidation
                    .with_max_tokens(300);

                match llm.complete(request).await {
                    Ok(response) => {
                        info!(
                            group_size = group.len(),
                            original_length = memories_text.len(),
                            consolidated_length = response.content.len(),
                            "LLM successfully consolidated memories"
                        );
                        response.content
                    }
                    Err(e) => {
                        debug!(error = %e, "LLM consolidation failed, using fallback");
                        // Fallback to simple concatenation
                        if group.len() <= 3 {
                            group
                                .iter()
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("; ")
                        } else {
                            format!(
                                "Summary of {} related memories: {}",
                                group.len(),
                                group
                                    .iter()
                                    .take(3)
                                    .map(|m| m.content.as_str())
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            )
                        }
                    }
                }
            } else {
                // No LLM available, use simple concatenation
                debug!("No LLM available, using simple concatenation");
                if group.len() <= 3 {
                    group
                        .iter()
                        .map(|m| m.content.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                } else {
                    format!(
                        "Summary of {} related memories: {}",
                        group.len(),
                        group
                            .iter()
                            .take(3)
                            .map(|m| m.content.as_str())
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                }
            };

            // Calculate average importance (with boost for consolidated memories)
            let avg_importance =
                group.iter().map(|m| m.importance).sum::<f32>() / group.len() as f32;
            let boosted_importance = (avg_importance * 1.1).min(1.0);

            // Generate embedding for consolidated content
            debug!("Generating embedding for consolidated memory");
            let embedding = self.generate_embedding(&consolidated_content).await?;

            // Create new consolidated memory
            let memory_id = MemoryId::new();
            let now = Utc::now();

            sqlx::query(
                r#"
                INSERT INTO wyldlands.entity_memory
                (memory_id, entity_id, kind, content, timestamp, last_accessed,
                 access_count, importance, decay_rate, context, metadata, related, embedding, tags)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                "#,
            )
            .bind(memory_id.uuid())
            .bind(entity_id.uuid())
            .bind(first.kind.to_db_str())
            .bind(&consolidated_content)
            .bind(now)
            .bind(now)
            .bind(0i32)
            .bind(boosted_importance)
            .bind(self.config.base_decay_rate * 0.8) // Slower decay for consolidated memories
            .bind(context)
            .bind(serde_json::json!({}))
            .bind(serde_json::json!({}))
            .bind(embedding)
            .bind(all_tags.iter().cloned().collect::<Vec<_>>())
            .execute(&self.pool)
            .await?;

            // Delete old memories
            for memory in &group {
                self.delete_memory(memory.memory_id.clone()).await?;
            }

            consolidated_count += group.len();
            groups_consolidated += 1;

            info!(
                memory_id = ?memory_id,
                group_size = group.len(),
                "Successfully consolidated memory group"
            );
        }

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        counter!("memory.operations.total", "operation" => "consolidate").increment(1);
        histogram!("memory.operation.duration", "operation" => "consolidate").record(duration);
        counter!("memory.consolidations.groups").increment(groups_consolidated);
        counter!("memory.consolidations.memories").increment(consolidated_count as u64);

        info!(
            groups_consolidated,
            memories_consolidated = consolidated_count,
            duration_ms = duration * 1000.0,
            "Memory consolidation completed"
        );

        Ok(consolidated_count)
    }

    /// List all memories for an entity with caching.
    ///
    /// Returns all memories owned by the specified entity, without filtering or ranking.
    /// Results are cached for improved performance on repeated queries.
    /// Useful for debugging and administrative interfaces.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity whose memories to list
    ///
    /// # Returns
    ///
    /// Returns a vector of all `MemoryNode`s for the entity.
    pub async fn list_memories(&self, entity_id: EntityId) -> MemoryResult<Vec<MemoryNode>> {
        // Check cache first
        if let Some(cached) = self.entity_memory_cache.get(&entity_id).await {
            counter!("memory.cache.hits", "type" => "entity_list").increment(1);
            debug!(entity_id = %entity_id, "Entity memory list cache hit");
            return Ok((*cached).clone());
        }

        counter!("memory.cache.misses", "type" => "entity_list").increment(1);

        let rows = sqlx::query(
            r#"
            SELECT memory_id, entity_id, kind, content, timestamp, last_accessed,
                   access_count, importance, decay_rate, context, metadata, related,
                   embedding, tags
            FROM wyldlands.entity_memory
            WHERE entity_id = $1
            ORDER BY timestamp DESC
            "#,
        )
        .bind(entity_id.uuid())
        .fetch_all(&self.pool)
        .await?;

        let mut memories = Vec::new();
        for row in rows {
            let memory = self.row_to_memory_node(row).await?;
            memories.push(memory);
        }

        // Cache the result
        self.entity_memory_cache
            .insert(entity_id, Arc::new(memories.clone()))
            .await;

        Ok(memories)
    }

    /// Retrieve a specific memory by ID with caching.
    ///
    /// # Arguments
    ///
    /// * `memory_id` - The unique identifier of the memory
    ///
    /// # Returns
    ///
    /// Returns the `MemoryNode` if found.
    pub async fn get_memory(&self, memory_id: MemoryId) -> MemoryResult<MemoryNode> {
        // Check cache first
        if let Some(cached) = self.memory_cache.get(&memory_id).await {
            counter!("memory.cache.hits", "type" => "memory").increment(1);
            debug!(memory_id = ?memory_id, "Memory cache hit");
            return Ok(Arc::unwrap_or_clone(cached));
        }

        counter!("memory.cache.misses", "type" => "memory").increment(1);

        let row = sqlx::query(
            r#"
            SELECT memory_id, entity_id, kind, content, timestamp, last_accessed,
                   access_count, importance, decay_rate, context, metadata, related,
                   embedding, tags
            FROM wyldlands.entity_memory
            WHERE memory_id = $1
            "#,
        )
        .bind(memory_id.uuid())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MemoryError::NotFound(memory_id.clone()))?;

        let memory = self.row_to_memory_node(row).await?;

        // Cache the result
        self.memory_cache
            .insert(memory_id.clone(), Arc::new(memory.clone()))
            .await;

        Ok(memory)
    }

    /// Invalidate cache for a specific memory.
    ///
    /// Call this after updating or deleting a memory to ensure cache consistency.
    pub async fn invalidate_memory_cache(&self, memory_id: &MemoryId) {
        self.memory_cache.invalidate(memory_id).await;
        counter!("memory.cache.invalidations", "type" => "memory").increment(1);
    }

    /// Invalidate cache for an entity's memory list.
    ///
    /// Call this after adding, updating, or deleting memories for an entity.
    pub async fn invalidate_entity_cache(&self, entity_id: &EntityId) {
        self.entity_memory_cache.invalidate(entity_id).await;
        counter!("memory.cache.invalidations", "type" => "entity_list").increment(1);
    }

    /// Clear all caches.
    ///
    /// Useful for testing or when you need to ensure fresh data.
    pub async fn clear_all_caches(&self) {
        self.memory_cache.invalidate_all();
        self.entity_memory_cache.invalidate_all();
        self.embedding_cache.invalidate_all();

        // Wait for invalidation to complete
        self.memory_cache.run_pending_tasks().await;
        self.entity_memory_cache.run_pending_tasks().await;
        self.embedding_cache.run_pending_tasks().await;

        info!("All memory caches cleared");
    }

    /// Get cache statistics for monitoring.
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            memory_cache_size: self.memory_cache.entry_count(),
            entity_cache_size: self.entity_memory_cache.entry_count(),
            embedding_cache_size: self.embedding_cache.entry_count(),
            memory_cache_capacity: self.config.cache_max_capacity,
            entity_cache_capacity: self.config.cache_max_capacity / 10,
            embedding_cache_capacity: self.config.embedding_cache_capacity,
        }
    }

    /// Helper method to convert a database row to a MemoryNode
    async fn row_to_memory_node(&self, row: sqlx::postgres::PgRow) -> MemoryResult<MemoryNode> {
        use sqlx::Row;

        let memory_id = MemoryId::from_uuid(row.get("memory_id"));
        let entity_id = EntityId::from_uuid(row.get("entity_id"));
        let kind = MemoryKind::from_db_str(row.get("kind"))?;
        let content: String = row.get("content");
        let timestamp: DateTime<Utc> = row.get("timestamp");
        let last_accessed: DateTime<Utc> = row.get("last_accessed");
        let access_count: i32 = row.get("access_count");
        let importance: f32 = row.get("importance");
        let decay_rate: f32 = row.get("decay_rate");
        let context: Option<String> = row.get("context");
        let metadata_json: serde_json::Value = row.get("metadata");
        let related_json: serde_json::Value = row.get("related");
        let embedding: Vec<f32> = row.get("embedding");
        let tags: Vec<String> = row.get("tags");

        // Convert metadata JSON to BTreeMap
        let metadata: BTreeMap<String, String> = serde_json::from_value(metadata_json)
            .map_err(|e| MemoryError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?;

        // Convert related JSON to BTreeMap<MemoryId, String>
        let related: BTreeMap<MemoryId, String> =
            if let serde_json::Value::Object(map) = related_json {
                map.into_iter()
                    .filter_map(|(k, v)| {
                        let uuid = uuid::Uuid::parse_str(&k).ok()?;
                        let desc = v.as_str()?.to_string();
                        Some((MemoryId::from_uuid(uuid), desc))
                    })
                    .collect()
            } else {
                BTreeMap::new()
            };

        // Fetch involved entities
        let entity_rows = sqlx::query(
            r#"
            SELECT involved_entity_id, role
            FROM wyldlands.entity_memory_entities
            WHERE memory_id = $1
            "#,
        )
        .bind(memory_id.uuid())
        .fetch_all(&self.pool)
        .await?;

        let entities: BTreeMap<EntityId, String> = entity_rows
            .into_iter()
            .map(|row| {
                let entity_id = EntityId::from_uuid(row.get("involved_entity_id"));
                let role: String = row.get("role");
                (entity_id, role)
            })
            .collect();

        Ok(MemoryNode {
            memory_id,
            entity_id,
            kind,
            content,
            timestamp,
            last_accessed,
            access_count: access_count as u32,
            importance,
            decay_rate,
            context,
            metadata,
            related,
            entities,
            embedding,
            tags: tags.into_iter().collect(),
        })
    }

    /// Modify an existing memory.
    ///
    /// Updates the content, context, or tags of an existing memory. The embedding
    /// will be regenerated if the content changes.
    ///
    /// # Arguments
    ///
    /// * `memory_id` - The memory to modify
    /// * `content` - New content (None = no change)
    /// * `context` - New context (None = no change)
    /// * `tags` - New tags (None = no change)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// memory.alter_memory(
    ///     memory_id,
    ///     Some("Updated memory content"),
    ///     None,
    ///     Some(&["new_tag", "updated"]),
    /// )?;
    /// ```
    pub async fn alter_memory(
        &mut self,
        memory_id: MemoryId,
        content: Option<&str>,
        context: Option<&str>,
        tags: Option<&[&str]>,
    ) -> MemoryResult<()> {
        // Verify memory exists
        let _ = self.get_memory(memory_id.clone()).await?;

        // Update content if provided
        if let Some(new_content) = content {
            if new_content.trim().is_empty() {
                return Err(MemoryError::InvalidContent(
                    "Content cannot be empty".to_string(),
                ));
            }
            if new_content.len() > 10000 {
                return Err(MemoryError::InvalidContent(
                    "Content exceeds maximum length of 10000 characters".to_string(),
                ));
            }

            // TODO: Regenerate embedding when content changes
            sqlx::query("UPDATE wyldlands.entity_memory SET content = $1 WHERE memory_id = $2")
                .bind(new_content)
                .bind(memory_id.uuid())
                .execute(&self.pool)
                .await?;
        }

        // Update context if provided
        if let Some(new_context) = context {
            sqlx::query("UPDATE wyldlands.entity_memory SET context = $1 WHERE memory_id = $2")
                .bind(new_context)
                .bind(memory_id.uuid())
                .execute(&self.pool)
                .await?;
        }

        // Update tags if provided
        if let Some(new_tags) = tags {
            let tags_vec: Vec<String> = new_tags.iter().map(|s| s.to_string()).collect();
            sqlx::query("UPDATE wyldlands.entity_memory SET tags = $1 WHERE memory_id = $2")
                .bind(&tags_vec)
                .bind(memory_id.uuid())
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Delete a memory permanently.
    ///
    /// Removes a memory from storage. This operation cannot be undone.
    ///
    /// # Arguments
    ///
    /// * `memory_id` - The memory to delete
    pub async fn delete_memory(&mut self, memory_id: MemoryId) -> MemoryResult<()> {
        let result = sqlx::query("DELETE FROM wyldlands.entity_memory WHERE memory_id = $1")
            .bind(memory_id.uuid())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(MemoryError::NotFound(memory_id));
        }

        Ok(())
    }
}

// Note: MemoryResource requires a PgPool, so Default trait is not implemented
// Use MemoryResource::new(pool) or MemoryResource::with_config(pool, config) instead

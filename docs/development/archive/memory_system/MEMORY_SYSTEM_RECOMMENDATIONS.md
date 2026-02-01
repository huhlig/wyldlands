# Memory System Recommendations

## Overview

This document provides recommendations for implementing and enhancing the AI memory system in `server/src/ecs/memory.rs`.

## Current State Analysis

### Strengths
- Well-designed architecture based on cognitive science principles
- Clear separation of memory types (World, Experience, Opinion, Observation)
- Flexible tagging and metadata system
- Support for semantic search via embeddings
- Comprehensive API surface

### Issues Identified

1. **Missing Timestamp Implementation**
   - Currently using `()` placeholder
   - Need proper time tracking for memory decay and temporal queries

2. **No Persistence Layer**
   - `persistence: ()` is a placeholder
   - Need database integration for memory storage

3. **Missing Embedding Generation**
   - No integration with embedding models
   - Critical for semantic search functionality

4. **No Memory Decay/Importance**
   - Memories should have importance scores
   - Older/less relevant memories should decay over time

5. **Missing Error Handling**
   - All methods use `unimplemented!()`
   - Need proper error types and handling

6. **No Memory Budget Management**
   - Entities could accumulate unlimited memories
   - Need limits and automatic pruning

## Recommendations

### 1. Implement Proper Timestamp Type

**Priority: HIGH**

```rust
use chrono::{DateTime, Utc};

pub struct MemoryNode {
    // Replace () with:
    pub timestamp: DateTime<Utc>,
    // Add:
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
}
```

**Benefits:**
- Enable temporal queries ("memories from last week")
- Support memory decay based on age
- Track memory usage patterns

### 2. Add Memory Importance and Decay

**Priority: HIGH**

```rust
pub struct MemoryNode {
    // Add:
    pub importance: f32,  // 0.0 to 1.0
    pub decay_rate: f32,  // How quickly importance decreases
}

impl MemoryNode {
    pub fn calculate_current_importance(&self, now: DateTime<Utc>) -> f32 {
        let age_seconds = (now - self.timestamp).num_seconds() as f32;
        let decay = (-self.decay_rate * age_seconds / 86400.0).exp(); // Daily decay
        self.importance * decay * (1.0 + (self.access_count as f32 * 0.1))
    }
}
```

**Benefits:**
- Automatic memory pruning based on relevance
- More realistic NPC behavior (forgetting old events)
- Performance optimization (fewer memories to search)

### 3. Implement Persistence Layer

**Priority: HIGH**

**Options:**

a) **PostgreSQL with pgvector** (Recommended)
   - Native vector similarity search
   - ACID compliance
   - Already used in the project

b) **Qdrant/Milvus**
   - Specialized vector databases
   - Better performance for large-scale deployments
   - Additional infrastructure complexity

**Recommendation:** Start with PostgreSQL + pgvector

```sql
CREATE TABLE memories (
    memory_id UUID PRIMARY KEY,
    entity_id UUID NOT NULL,
    kind VARCHAR(20) NOT NULL,
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    context TEXT,
    metadata JSONB,
    embedding vector(384), -- or 768/1536 depending on model
    tags TEXT[],
    importance REAL,
    access_count INTEGER DEFAULT 0,
    last_accessed TIMESTAMPTZ
);

CREATE INDEX idx_memories_entity ON memories(entity_id);
CREATE INDEX idx_memories_embedding ON memories USING ivfflat (embedding vector_cosine_ops);
CREATE INDEX idx_memories_tags ON memories USING gin(tags);
```

### 4. Integrate Embedding Model

**Priority: HIGH**

**Options:**

a) **sentence-transformers (Rust port)**
   - Local inference, no API costs
   - Models: all-MiniLM-L6-v2 (384d), all-mpnet-base-v2 (768d)

b) **OpenAI text-embedding-ada-002**
   - High quality (1536d)
   - API costs
   - Already have OpenAI integration

c) **Local LLM embedding endpoint**
   - Use existing LLM infrastructure
   - Ollama/LM Studio support embeddings

**Recommendation:** Use local sentence-transformers for cost efficiency

```rust
use rust_bert::pipelines::sentence_embeddings::{
    SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};

pub struct EmbeddingService {
    model: SentenceEmbeddingsModel,
}

impl EmbeddingService {
    pub fn new() -> Result<Self> {
        let model = SentenceEmbeddingsBuilder::remote(
            SentenceEmbeddingsModelType::AllMiniLmL6V2
        ).create_model()?;
        Ok(Self { model })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.encode(&[text])?;
        Ok(embeddings[0].clone())
    }
}
```

### 5. Add Error Handling

**Priority: MEDIUM**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Memory not found: {0}")]
    NotFound(MemoryId),
    
    #[error("Entity not found: {0:?}")]
    EntityNotFound(EntityId),
    
    #[error("Embedding generation failed: {0}")]
    EmbeddingError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Memory limit exceeded for entity")]
    MemoryLimitExceeded,
}

pub type MemoryResult<T> = Result<T, MemoryError>;
```

### 6. Implement Memory Budget Management

**Priority: MEDIUM**

```rust
pub struct MemoryConfig {
    pub max_tokens: usize,
    pub max_recall_results: usize,
    pub similarity_threshold: f32,
    // Add:
    pub max_memories_per_entity: usize,  // e.g., 1000
    pub min_importance_threshold: f32,    // e.g., 0.1
    pub consolidation_threshold: usize,   // e.g., 800 (trigger at 80%)
}

impl MemoryResource {
    pub fn prune_low_importance_memories(&mut self, entity_id: EntityId) -> MemoryResult<usize> {
        // Remove memories below importance threshold
        // Keep at least some minimum number
    }
    
    pub fn auto_consolidate_if_needed(&mut self, entity_id: EntityId) -> MemoryResult<()> {
        let count = self.count_memories(entity_id)?;
        if count > self.config.consolidation_threshold {
            self.consolidate(entity_id, "", None, [], MemoryTagMode::Any)?;
        }
        Ok(())
    }
}
```

### 7. Add Memory Relationships and Graphs

**Priority: LOW**

Enhance the `related` field to support memory chains and causal relationships:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemoryRelationType {
    CausedBy,
    LeadsTo,
    SimilarTo,
    ContradictsTo,
    PartOf,
    Elaborates,
}

pub struct MemoryRelation {
    pub target_id: MemoryId,
    pub relation_type: MemoryRelationType,
    pub strength: f32,  // 0.0 to 1.0
}
```

### 8. Add Memory Querying DSL

**Priority: LOW**

For complex queries:

```rust
pub struct MemoryQuery {
    pub entity_id: EntityId,
    pub text_query: Option<String>,
    pub kinds: Vec<MemoryKind>,
    pub tags: Vec<String>,
    pub tags_match: MemoryTagMode,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub min_importance: Option<f32>,
    pub limit: usize,
}

impl MemoryResource {
    pub fn query(&self, query: MemoryQuery) -> MemoryResult<Vec<MemoryNode>> {
        // Complex query execution
    }
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1-2)
1. Add proper timestamp types
2. Implement error handling
3. Set up PostgreSQL schema with pgvector
4. Create basic CRUD operations

### Phase 2: Semantic Search (Week 3-4)
1. Integrate embedding model
2. Implement `retain()` with embedding generation
3. Implement `recall()` with vector similarity
4. Add basic tests

### Phase 3: Advanced Features (Week 5-6)
1. Implement memory importance and decay
2. Add `consolidate()` functionality
3. Implement memory budget management
4. Add `reflect()` with LLM integration

### Phase 4: Optimization (Week 7-8)
1. Performance tuning
2. Caching layer
3. Batch operations
4. Comprehensive testing

## Testing Strategy

### Unit Tests
- Memory creation and retrieval
- Importance calculation
- Tag filtering logic
- Embedding generation

### Integration Tests
- Database operations
- Vector similarity search
- Memory consolidation
- LLM reflection

### Performance Tests
- Large memory sets (10k+ memories)
- Concurrent access
- Query performance
- Embedding generation speed

## Dependencies to Add

```toml
[dependencies]
# Time handling
chrono = { version = "0.4", features = ["serde"] }

# Embeddings
rust-bert = "0.21"  # or candle-transformers
tokenizers = "0.15"

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls", "uuid", "chrono"] }
pgvector = "0.3"

# Error handling
thiserror = "1.0"
anyhow = "1.0"
```

## Security Considerations

1. **Memory Injection**: Sanitize memory content to prevent prompt injection
2. **Privacy**: Consider memory encryption for sensitive data
3. **Access Control**: Ensure entities can only access their own memories
4. **Rate Limiting**: Prevent memory spam/DoS

## Performance Considerations

1. **Embedding Cache**: Cache embeddings for frequently accessed memories
2. **Batch Processing**: Generate embeddings in batches
3. **Index Optimization**: Proper database indexes for common queries
4. **Memory Pooling**: Reuse embedding model instances

## Future Enhancements

1. **Multi-modal Memories**: Support images, audio descriptions
2. **Shared Memories**: Group memories (e.g., party experiences)
3. **Memory Dreams**: Periodic background consolidation
4. **Memory Visualization**: Tools for debugging and analysis
5. **Memory Export/Import**: Save/load memory states

## References

- [arXiv:2512.12818v1](https://arxiv.org/html/2512.12818v1) - Memory architecture research
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [sentence-transformers](https://www.sbert.net/)
- [Cognitive Memory Systems](https://en.wikipedia.org/wiki/Memory)
# Memory System LLM Integration

## Overview

The memory system now integrates with the LLM manager to provide intelligent, context-aware responses based on entity memories. This enables NPCs to have coherent conversations that reference their past experiences, knowledge, and opinions.

## Architecture

### Components

1. **MemoryResource**: Manages entity memories with database persistence
2. **LlmManager**: Handles LLM provider connections and requests
3. **WorldContext**: Provides access to both memory and LLM systems

### Integration Points

The `reflect()` method in `MemoryResource` accepts an optional `LlmManager` reference:

```rust
pub async fn reflect<'a>(
    &self,
    entity_id: EntityId,
    query: &str,
    context: Option<&str>,
    tags: impl IntoIterator<Item = &'a str>,
    tags_match: MemoryTagMode,
    llm_manager: Option<&LlmManager>,
    model: Option<&str>,
) -> MemoryResult<(String, Vec<MemoryNode>)>
```

## Usage Examples

### Basic Usage (Without LLM)

When no LLM manager is provided, the system returns formatted memories:

```rust
use wyldlands_server::ecs::memory::{MemoryResource, MemoryKind, MemoryTagMode};

let memory = MemoryResource::new(pool);

// Store some memories
memory.retain(
    npc_entity,
    MemoryKind::Experience,
    "Met a friendly merchant in the tavern",
    Utc::now(),
    Some("social interaction"),
    [],
    [],
    ["social", "merchant", "tavern"],
).await?;

// Reflect without LLM (returns formatted memories)
let (response, used_memories) = memory.reflect(
    npc_entity,
    "Who have you met recently?",
    None,
    ["social"],
    MemoryTagMode::Any,
    None,  // No LLM manager
    None,  // No model
).await?;

println!("Response: {}", response);
// Output: Formatted list of relevant memories
```

### Advanced Usage (With LLM Integration)

When an LLM manager is provided, the system generates natural language responses:

```rust
use wyldlands_server::ecs::memory::{MemoryResource, MemoryKind, MemoryTagMode};
use wyldlands_server::llm::{LlmManager, LLMConfig};
use std::sync::Arc;

// Setup LLM manager
let llm_manager = Arc::new(LlmManager::new());
llm_manager.register_provider(
    "openai",
    LLMConfig::openai("your-api-key", "https://api.openai.com/v1/chat/completions")
).await?;

let memory = MemoryResource::new(pool);

// Store diverse memories
memory.retain(
    npc_entity,
    MemoryKind::Experience,
    "Fought a dragon in the northern mountains",
    Utc::now(),
    Some("The dragon was fierce and breathed fire"),
    [],
    [],
    ["combat", "dragon", "mountains"],
).await?;

memory.retain(
    npc_entity,
    MemoryKind::Opinion,
    "Dragons are dangerous but honorable creatures",
    Utc::now(),
    None,
    [],
    [],
    ["dragon", "opinion"],
).await?;

memory.retain(
    npc_entity,
    MemoryKind::World,
    "Dragons live in mountain caves and hoard treasure",
    Utc::now(),
    None,
    [],
    [],
    ["dragon", "knowledge"],
).await?;

// Reflect with LLM integration
let (response, used_memories) = memory.reflect(
    npc_entity,
    "What do you know about dragons?",
    Some("A traveler is asking for advice"),
    ["dragon"],
    MemoryTagMode::Any,
    Some(&llm_manager),
    Some("gpt-4"),
).await?;

println!("NPC says: {}", response);
// Output: Natural language response incorporating all relevant memories
// Example: "Ah, dragons! I've had my share of encounters with them. 
//          I once fought one in the northern mountains - fierce creature, 
//          breathed fire like a furnace. Despite their danger, I've come 
//          to respect them as honorable beings. They typically dwell in 
//          mountain caves, guarding their treasure hoards. If you're 
//          planning to seek one out, be prepared and show respect."
```

### Using with WorldContext

The recommended approach is to use the memory system through `WorldContext`:

```rust
use wyldlands_server::ecs::context::WorldContext;
use wyldlands_server::ecs::memory::{MemoryKind, MemoryTagMode};
use std::sync::Arc;

// Create context with LLM manager
let context = Arc::new(WorldContext::with_llm_manager(
    persistence_manager,
    llm_manager,
));

// Access memory system
let memory = MemoryResource::new(context.persistence().pool().clone());

// Use LLM manager from context
let (response, memories) = memory.reflect(
    entity_id,
    "What should I do next?",
    Some("The NPC is considering their options"),
    ["goal", "plan"],
    MemoryTagMode::Any,
    Some(context.llm_manager().as_ref()),
    Some("gpt-4"),
).await?;
```

## Memory Context in LLM Prompts

The system automatically formats memories for the LLM:

### System Message Format

```
You are responding based on the following memories:

Relevant memories:
1. [World Knowledge] Dragons live in mountain caves and hoard treasure
2. [Experience] Fought a dragon in the northern mountains
   Context: The dragon was fierce and breathed fire
3. [Opinion] Dragons are dangerous but honorable creatures

Use these memories to provide a contextual, natural response. 
Speak in first person as if you are the entity with these memories. 
Be concise and relevant to the query.
```

### User Message Format

```
Context: A traveler is asking for advice

Query: What do you know about dragons?
```

## Configuration

### Memory Configuration

Control how memories are used in reflection:

```rust
let config = MemoryConfig {
    max_tokens: 4096,              // Max tokens for LLM response
    max_recall_results: 10,        // Max memories to include
    similarity_threshold: 0.7,     // Minimum similarity for recall
    ..Default::default()
};

let memory = MemoryResource::with_config(pool, config);
```

### LLM Configuration

Configure the LLM provider:

```rust
// OpenAI
let config = LLMConfig::openai(api_key, endpoint);

// Ollama (local)
let config = LLMConfig::ollama(endpoint, model);

// LM Studio (local)
let config = LLMConfig::lmstudio(endpoint, model);

llm_manager.register_provider("my_provider", config).await?;
```

## Best Practices

### 1. Tag Your Memories

Use descriptive tags to enable efficient filtering:

```rust
memory.retain(
    entity_id,
    MemoryKind::Experience,
    "Learned a new spell from the wizard",
    Utc::now(),
    None,
    [],
    [],
    ["magic", "learning", "wizard", "spell"],  // Good tagging
).await?;
```

### 2. Provide Context

Add context to queries for better responses:

```rust
let (response, _) = memory.reflect(
    entity_id,
    "Should I trust the merchant?",
    Some("The merchant is offering a suspiciously good deal"),  // Context helps
    ["merchant", "trust"],
    MemoryTagMode::Any,
    Some(&llm_manager),
    None,
).await?;
```

### 3. Use Appropriate Memory Kinds

- **World**: Static facts and knowledge
- **Experience**: Personal history and events
- **Opinion**: Subjective beliefs and preferences
- **Observation**: Current/recent sensory input

### 4. Handle LLM Failures Gracefully

```rust
let (response, memories) = match memory.reflect(
    entity_id,
    query,
    context,
    tags,
    MemoryTagMode::Any,
    Some(&llm_manager),
    Some("gpt-4"),
).await {
    Ok(result) => result,
    Err(MemoryError::LlmError(e)) => {
        // Fallback to non-LLM response
        eprintln!("LLM error: {}", e);
        memory.reflect(
            entity_id,
            query,
            context,
            tags,
            MemoryTagMode::Any,
            None,  // No LLM
            None,
        ).await?
    }
    Err(e) => return Err(e),
};
```

### 5. Limit Memory Retrieval

Don't overwhelm the LLM with too many memories:

```rust
// Good: Focused query with limited results
let (response, _) = memory.reflect(
    entity_id,
    "What happened yesterday?",
    None,
    ["recent"],
    MemoryTagMode::Any,
    Some(&llm_manager),
    None,
).await?;

// The system automatically limits to config.max_recall_results (default: 10)
```

## Performance Considerations

### Memory Retrieval

- Recall is optimized with database indexes
- Vector similarity search (when embeddings are implemented) will be fast
- Tag filtering uses GIN indexes for efficient lookups

### LLM Calls

- Each `reflect()` call makes one LLM API request
- Consider caching responses for repeated queries
- Use appropriate `max_tokens` to control costs

### Async Operations

All operations are async and non-blocking:

```rust
// Multiple reflects can run concurrently
let (response1, response2) = tokio::join!(
    memory.reflect(entity1, query1, None, [], MemoryTagMode::Any, Some(&llm), None),
    memory.reflect(entity2, query2, None, [], MemoryTagMode::Any, Some(&llm), None),
);
```

## Testing

### Unit Tests

Test without LLM integration:

```rust
#[tokio::test]
async fn test_reflect_without_llm() {
    let memory = MemoryResource::new(pool);
    
    // Create test memories
    memory.retain(/* ... */).await.unwrap();
    
    // Test without LLM
    let (response, memories) = memory.reflect(
        entity_id,
        "test query",
        None,
        [],
        MemoryTagMode::Any,
        None,  // No LLM for testing
        None,
    ).await.unwrap();
    
    assert!(!response.is_empty());
    assert!(!memories.is_empty());
}
```

### Integration Tests

Test with mock LLM:

```rust
#[tokio::test]
async fn test_reflect_with_llm() {
    let llm_manager = setup_test_llm_manager().await;
    let memory = MemoryResource::new(pool);
    
    // Create test memories
    memory.retain(/* ... */).await.unwrap();
    
    // Test with LLM
    let (response, memories) = memory.reflect(
        entity_id,
        "test query",
        None,
        [],
        MemoryTagMode::Any,
        Some(&llm_manager),
        Some("test-model"),
    ).await.unwrap();
    
    assert!(!response.is_empty());
    assert!(!memories.is_empty());
}
```

## Future Enhancements

### Planned Features

1. **Vector Embeddings**: Semantic similarity search using sentence-transformers
2. **Memory Consolidation**: LLM-powered memory merging
3. **Emotional Context**: Include emotional state in reflections
4. **Multi-turn Conversations**: Maintain conversation history
5. **Memory Importance Learning**: Adjust importance based on usage patterns

### Roadmap

- **Phase 1** (Complete): Basic LLM integration for reflect()
- **Phase 2** (Pending): Vector embedding generation
- **Phase 3** (Pending): LLM-powered consolidation
- **Phase 4** (Pending): Advanced context management
- **Phase 5** (Pending): Performance optimization and caching

## Troubleshooting

### Common Issues

**Issue**: LLM responses are generic or don't reference memories

**Solution**: Ensure memories have good content and tags. Check that recall is finding relevant memories.

**Issue**: LLM requests timeout

**Solution**: Reduce `max_tokens` in config or use a faster model.

**Issue**: Too many memories in context

**Solution**: Use more specific tags and reduce `max_recall_results`.

**Issue**: Responses don't match entity personality

**Solution**: Add personality information to memory context or use CharacterContext in LLM requests.

## See Also

- [Memory System Documentation](MEMORY_SYSTEM_COMPLETE.md)
- [LLM Manager Documentation](../DEVELOPER_GUIDE.md#llm-integration)
- [Memory System Recommendations](MEMORY_SYSTEM_RECOMMENDATIONS.md)
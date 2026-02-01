---
parent: ADR
nav_order: 0013
title: LLM Integration Architecture
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0013: LLM Integration Architecture

## Context and Problem Statement

Modern MUDs can benefit from Large Language Models (LLMs) for dynamic content generation and NPC dialogue. The system must:
- Support multiple LLM providers (OpenAI, Ollama, LM Studio, Mistral)
- Enable NPC dialogue that feels natural and contextual
- Generate creative content (room descriptions, item details, NPC profiles)
- Handle provider failures gracefully
- Manage API costs and rate limits
- Maintain consistent character personalities
- Support both cloud and local LLM hosting

How should we integrate LLMs to provide flexibility, reliability, and cost-effectiveness?

## Decision Drivers

* **Provider Flexibility**: Support multiple LLM providers without code changes
* **Cost Management**: Control API usage and costs
* **Reliability**: Handle provider failures and timeouts
* **Local Hosting**: Support self-hosted models (Ollama, LM Studio)
* **Performance**: Minimize latency for real-time dialogue
* **Extensibility**: Easy to add new providers
* **Context Management**: Maintain conversation history and character context
* **Quality Control**: Ensure generated content fits the game world

## Considered Options

* Multi-Provider Abstraction Layer
* Single Provider (OpenAI Only)
* Plugin-Based Provider System
* Embedded Model Integration

## Decision Outcome

Chosen option: "Multi-Provider Abstraction Layer", because it provides the best balance of flexibility, reliability, and cost management while supporting both cloud and local hosting options.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    ModelManager                          │
│  Coordinates multiple LLM providers                      │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   OpenAI     │  │    Ollama    │  │  LM Studio   │ │
│  │   Provider   │  │   Provider   │  │   Provider   │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│  ┌──────────────┐                                      │
│  │   Mistral    │                                      │
│  │   Provider   │                                      │
│  └──────────────┘                                      │
└─────────────────────────────────────────────────────────┘
                          │
                          │ LLMRequest/LLMResponse
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Application Layer                       │
│  • NPC Dialogue System                                  │
│  • Content Generation (rooms, items, NPCs)             │
│  • Dynamic Quest Generation                             │
└─────────────────────────────────────────────────────────┘
```

### Provider Abstraction

**Trait Definition:**
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;
    async fn is_available(&self) -> bool;
    fn name(&self) -> &str;
}
```

**Supported Providers:**
1. **OpenAI**: GPT-3.5, GPT-4, GPT-4-turbo
2. **Ollama**: Local hosting (llama2, mistral, etc.)
3. **LM Studio**: OpenAI-compatible local API
4. **Mistral**: Mistral AI cloud API

### Request/Response Types

**LLMRequest:**
```rust
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<LLMMessage>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub context: Option<CharacterContext>,
}

pub struct LLMMessage {
    pub role: LLMRole,  // System, User, Assistant
    pub content: String,
}
```

**CharacterContext:**
```rust
pub struct CharacterContext {
    pub name: Option<String>,
    pub personality: Option<String>,
    pub emotional_state: Option<String>,
    pub location: Option<String>,
    pub recent_events: Vec<String>,
    pub relationships: HashMap<String, String>,
}
```

### Positive Consequences

* **Provider Independence**: Switch providers without code changes
* **Cost Control**: Use cheaper local models for testing
* **Reliability**: Fallback to alternative providers
* **Flexibility**: Mix cloud and local providers
* **Extensibility**: Easy to add new providers
* **Context-Aware**: Rich character context for better responses
* **Testability**: Mock providers for testing

### Negative Consequences

* **Complexity**: More complex than single provider
* **Configuration**: Requires provider configuration
* **Consistency**: Different providers may produce different results
* **Latency**: Network calls add latency

## Pros and Cons of the Options

### Multi-Provider Abstraction Layer

* Good, because supports multiple providers
* Good, because enables local hosting
* Good, because provides fallback options
* Good, because easy to add new providers
* Good, because testable with mock providers
* Neutral, because requires configuration
* Bad, because more complex than single provider
* Bad, because providers may behave differently

### Single Provider (OpenAI Only)

* Good, because simple implementation
* Good, because consistent results
* Good, because well-documented API
* Neutral, because requires API key
* Bad, because vendor lock-in
* Bad, because ongoing API costs
* Bad, because no local hosting option
* Bad, because single point of failure

### Plugin-Based Provider System

* Good, because highly extensible
* Good, because dynamic provider loading
* Neutral, because requires plugin infrastructure
* Bad, because more complex
* Bad, because harder to configure
* Bad, because potential security issues

### Embedded Model Integration

* Good, because no network latency
* Good, because no API costs
* Good, because complete control
* Neutral, because requires model files
* Bad, because large memory footprint
* Bad, because slower inference
* Bad, because harder to update models

## Implementation Details

### ModelManager

**Location:** `server/src/models/manager.rs`

```rust
pub struct ModelManager {
    llm_providers: Arc<RwLock<HashMap<String, Box<dyn LlmProvider>>>>,
    default_llm_provider: Arc<RwLock<Option<String>>>,
}

impl ModelManager {
    pub async fn register_llm_provider(
        &self,
        name: impl Into<String>,
        config: LLMConfig,
    ) -> Result<(), LLMError>;
    
    pub async fn complete(
        &self,
        request: LLMRequest,
    ) -> Result<LLMResponse, LLMError>;
    
    pub async fn complete_with_provider(
        &self,
        provider_name: &str,
        request: LLMRequest,
    ) -> Result<LLMResponse, LLMError>;
}
```

### Provider Implementations

**OpenAI Provider:**
- Uses `reqwest` for HTTP requests
- Supports GPT-3.5, GPT-4 models
- Requires API key
- Endpoint: `https://api.openai.com/v1/chat/completions`

**Ollama Provider:**
- Local HTTP API
- Supports any Ollama model
- No API key required
- Endpoint: `http://localhost:11434/api/chat`

**LM Studio Provider:**
- OpenAI-compatible local API
- Supports any loaded model
- No API key required
- Endpoint: `http://localhost:1234/v1/chat/completions`

**Mistral Provider:**
- Mistral AI cloud API
- Supports Mistral models
- Requires API key
- Endpoint: `https://api.mistral.ai/v1/chat/completions`

### Configuration

**server/config.yaml:**
```yaml
llm:
  default_provider: "openai"
  
  providers:
    openai:
      provider: "openai"
      api_key: "${OPENAI_API_KEY}"
      model: "gpt-4"
      endpoint: "https://api.openai.com/v1/chat/completions"
      timeout_seconds: 30
      max_retries: 3
    
    ollama:
      provider: "ollama"
      model: "llama2"
      endpoint: "http://localhost:11434/api/chat"
      timeout_seconds: 60
    
    lmstudio:
      provider: "lmstudio"
      model: "local-model"
      endpoint: "http://localhost:1234/v1/chat/completions"
      timeout_seconds: 60
```

### Use Cases

**1. NPC Dialogue:**
```rust
let context = CharacterContext::new()
    .with_name("Elara the Wise")
    .with_personality("Mysterious wizard, speaks in riddles")
    .with_emotional_state("Curious")
    .with_location("Ancient Library");

let request = LLMRequest::new("gpt-4")
    .with_context(context)
    .with_message(LLMMessage::system("You are Elara the Wise..."))
    .with_message(LLMMessage::user("What do you know about the ancient artifact?"))
    .with_temperature(0.8);

let response = model_manager.complete(request).await?;
```

**2. Room Description Generation:**
```rust
let request = LLMRequest::new("gpt-4")
    .with_message(LLMMessage::system(
        "Generate a fantasy MUD room description in JSON format..."
    ))
    .with_message(LLMMessage::user(
        "a dark mysterious cave with glowing crystals"
    ))
    .with_temperature(0.8);

let response = model_manager.complete(request).await?;
let room_data: RoomData = serde_json::from_str(&response.content)?;
```

**3. Content Generation Commands:**
- `room generate <uuid> <prompt>` - Generate room descriptions
- `item generate <uuid> <prompt>` - Generate item details
- `npc generate <uuid> <prompt>` - Generate NPC profiles

### Error Handling

**Error Types:**
```rust
pub enum LLMError {
    ProviderNotFound(String),
    ProviderUnavailable(String),
    RequestFailed(String),
    InvalidResponse(String),
    Timeout,
    RateLimitExceeded,
    InvalidApiKey,
}
```

**Retry Strategy:**
- Automatic retry on transient failures
- Exponential backoff
- Configurable max retries
- Fallback to alternative provider

### Quality Control

**Prompt Engineering:**
- System prompts define role and output format
- JSON-based responses for structured data
- Temperature control for creativity vs consistency
- Token limits to control response length

**Validation:**
- Parse JSON responses
- Validate required fields
- Sanitize content for game world
- Reject inappropriate content

## Validation

The LLM integration is validated by:

1. **Unit Tests**: Provider creation and configuration
2. **Integration Tests**: End-to-end LLM requests (with mocks)
3. **Manual Testing**: Real provider testing with various prompts
4. **Error Handling Tests**: Timeout, rate limit, invalid response scenarios
5. **Performance Tests**: Latency and throughput measurements

## More Information

### Cost Management

**Strategies:**
1. Use local models (Ollama, LM Studio) for development
2. Cache common responses
3. Limit max tokens per request
4. Use cheaper models for non-critical content
5. Implement rate limiting per user/session

### Performance Optimization

**Techniques:**
1. Async/await for non-blocking requests
2. Connection pooling for HTTP clients
3. Request batching where possible
4. Streaming responses for long content
5. Timeout configuration per provider

### Future Enhancements

1. **Response Caching**: Cache common NPC responses
2. **Fine-Tuned Models**: Train custom models for game world
3. **Streaming Responses**: Real-time response streaming
4. **Multi-Model Ensembles**: Combine multiple models
5. **Embeddings**: Vector search for semantic content
6. **Function Calling**: Structured tool use
7. **Vision Models**: Image-based content generation

### Related Decisions

- [ADR-0004](ADR-0004-Use-Entity-Component-System.md) - ECS enables NPC components
- [ADR-0014](ADR-0014-GOAP-AI-System-Design.md) - GOAP AI works with LLM dialogue
- [ADR-0020](ADR-0020-Configuration-Management-Approach.md) - LLM configuration

### References

- Model Manager: [server/src/models/manager.rs](../../server/src/models/manager.rs)
- Provider Trait: [server/src/models/providers.rs](../../server/src/models/providers.rs)
- Types: [server/src/models/types.rs](../../server/src/models/types.rs)
- LLM Generation Guide: [docs/LLM_GENERATION.md](../LLM_GENERATION.md)
- NPC System: [docs/NPC_SYSTEM.md](../NPC_SYSTEM.md)
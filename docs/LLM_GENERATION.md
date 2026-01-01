# LLM Content Generation

Use Large Language Models to automatically generate creative content for rooms, items, and NPCs.

## Quick Start

```
room generate <uuid> <prompt>    # Generate room description
item generate <uuid> <prompt>    # Generate item details
npc generate <uuid> <prompt>     # Generate NPC profile
```

## Commands

### Room Generation
```
room generate <uuid> <prompt>
```

Generates creative room name and descriptions.

**Example:**
```
room generate 123e4567-e89b-12d3-a456-426614174000 a dark mysterious cave with glowing crystals
```

**Output:**
- Name (2-5 words)
- Short description (one line)
- Long description (detailed, multi-sentence)

### Item Generation
```
item generate <uuid> <prompt>
```

Generates item name, description, and keywords.

**Example:**
```
item generate 123e4567-e89b-12d3-a456-426614174000 an ancient sword with glowing runes
```

**Output:**
- Name (2-5 words)
- Keywords (searchable terms)
- Short description (one line)
- Long description (detailed appearance)

### NPC Generation
```
npc generate <uuid> <prompt>
```

Generates complete NPC profile including personality.

**Example:**
```
npc generate 123e4567-e89b-12d3-a456-426614174000 a grumpy old blacksmith
```

**Output:**
- Name
- Short description (one line)
- Long description (physical details)
- Background (character history)
- Speaking style
- System prompt (for LLM dialogue)

## Implementation Status

⚠️ **Requires Integration** - Commands are implemented but need LLM manager integration with WorldContext.

✅ **Complete:**
- Command structure and routing
- LLM manager abstraction
- JSON-based prompting
- Multiple provider support (OpenAI, Ollama, LM Studio)
- Error handling

⏳ **Needs:**
- LlmManager in WorldContext
- Command system integration
- Command registration

## How It Works

### 1. Prompt Engineering
Each command uses crafted system prompts that:
- Define the role (creative writer for fantasy MUD)
- Specify exact JSON output format
- Request specific fields for entity type
- Encourage vivid, immersive descriptions

### 2. LLM Request
```rust
let request = LlmRequest::new("gpt-4")
    .with_message(LlmMessage::system(system_prompt))
    .with_message(LlmMessage::user(user_prompt))
    .with_temperature(0.8)   // Creative but consistent
    .with_max_tokens(500);   // Sufficient for details
```

### 3. JSON Parsing
Responses are parsed as structured JSON:
```json
{
  "name": "Crystal Cavern",
  "short_description": "A cave filled with luminescent crystals",
  "long_description": "The walls sparkle with thousands of crystals..."
}
```

### 4. Entity Update
Parsed data updates entity components automatically.

## Configuration

### Provider Setup

**OpenAI:**
```rust
let config = LlmConfig::openai("api-key", "gpt-4");
manager.register_provider("openai", config).await?;
```

**Ollama (Local):**
```rust
let config = LlmConfig::ollama("http://localhost:11434/api/chat", "llama2");
manager.register_provider("ollama", config).await?;
```

**LM Studio (Local):**
```rust
let config = LlmConfig::lmstudio("http://localhost:1234/v1/chat/completions", "model");
manager.register_provider("lmstudio", config).await?;
```

### Temperature Settings
- **0.7-0.8** - Balanced creativity (recommended)
- **0.5-0.6** - More consistent
- **0.9-1.0** - Very creative

### Token Limits
- **Rooms** - 500 tokens
- **Items** - 400 tokens
- **NPCs** - 600 tokens

## Best Practices

### Writing Effective Prompts

**Good:**
- "a dark mysterious cave with glowing blue crystals"
- "an ancient elven sword with intricate runes"
- "a cheerful halfling innkeeper who loves to gossip"

**Less Effective:**
- "cave" (too vague)
- "sword" (lacks detail)
- "npc" (no personality)

### Tips
1. **Be Specific** - Include key details
2. **Use Adjectives** - Describe mood, appearance, age
3. **Add Context** - Mention setting or purpose
4. **Suggest Personality** - For NPCs, include traits

### Quality Control
1. Review generated content before finalizing
2. Edit as needed - LLM output is a starting point
3. Ensure consistency with your world
4. Save effective prompts for reuse

## Troubleshooting

### "LLM generation commands require additional integration"
Commands need LLM manager in context. Follow integration guide in code comments.

### Invalid JSON Response
- Check system prompt format
- Verify model supports JSON output
- Try lower temperature
- Check token limits

### Poor Quality Output
- Improve prompts with more detail
- Try different temperature settings
- Use more capable model (GPT-4 vs GPT-3.5)
- Adjust max_tokens if cut off

### Rate Limiting
- Implement request throttling
- Use local models (Ollama/LM Studio)
- Cache common generations
- Batch similar requests

## Examples

### Complete Room Workflow
```
1. Create room:
   room create <area-uuid> "Empty Cave"

2. Generate description:
   room generate <room-uuid> a vast underground cavern with stalactites

3. Review and edit if needed:
   room edit <room-uuid> description "..."

4. Add exits and continue building
```

### Complete NPC Workflow
```
1. Create NPC:
   npc create "Unnamed Guard"

2. Generate profile:
   npc generate <npc-uuid> a stern but fair town guard captain

3. Enable LLM dialogue:
   npc dialogue <npc-uuid> enabled true

4. Test conversation
```

## API Reference

See inline documentation:
- `server/src/ecs/systems/command/llm_generate.rs` - Commands
- `server/src/llm/` - LLM manager and providers
- `docs/NPC_SYSTEM.md` - NPC LLM features

## See Also
- [NPC System](NPC_SYSTEM.md) - NPC dialogue and AI
- [Builder Commands](BUILDER_COMMANDS.md) - World building
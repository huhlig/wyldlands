# NPC System

AI-controlled Non-Player Characters with GOAP planning, LLM dialogue, and personality systems.

## Quick Start

```
npc create <name>                    # Create NPC at current location
npc edit <uuid> behavior Friendly    # Set AI behavior
npc dialogue <uuid> enabled true     # Enable LLM dialogue
npc goap <uuid> show                 # View GOAP configuration
```

## Components

### Core Components

**Npc** - Marks entity as NPC
- `active` - Whether NPC is active
- `template_id` - Optional template reference

**AIController** - Basic AI behavior
- `behavior_type` - Passive, Wandering, Aggressive, Defensive, Friendly, Merchant, Quest, Custom
- `current_goal` - Active goal
- `state_type` - Idle, Moving, Combat, Fleeing, Following, Dialogue
- `update_interval` - AI update frequency (seconds)

**GoapPlanner** - Goal-Oriented Action Planning
- `actions` - Available actions
- `goals` - Pursuable goals
- `world_state` - NPC's view of the world
- `current_plan` - Action sequence for current goal

**NpcDialogue** - LLM dialogue configuration
- `llm_enabled` - Use LLM for dialogue
- `llm_provider` - openai, ollama, or lmstudio
- `llm_model` - Model name
- `system_prompt` - LLM system prompt
- `temperature` - Response randomness (0.0-2.0)
- `max_tokens` - Maximum response length
- `fallback_responses` - Responses when LLM unavailable

**Personality** - Character traits
- `background` - Character backstory
- `speaking_style` - How NPC speaks

**Memory** - Event tracking
- Stores important interactions
- Short-term and long-term memories
- Tracks involved entities

## GOAP System

Goal-Oriented Action Planning for intelligent NPC decisions.

### How It Works
1. Define goals (what NPC wants)
2. Define actions (what NPC can do)
3. Planner uses A* to find action sequence
4. Execute actions one at a time

### Example
```rust
// Create goal
let goal = GoapGoal::new("be_fed", "Be Fed", 10)
    .with_condition("has_food", true);

// Create actions
let find_food = GoapAction::new("find_food", "Find Food")
    .with_precondition("has_food", false)
    .with_effect("has_food", true)
    .with_cost(5.0);

// Add to planner
planner.add_goal(goal);
planner.add_action(find_food);
```

## LLM Integration

### Supported Providers
- **OpenAI** - GPT-3.5, GPT-4
- **Ollama** - Local LLM hosting
- **LM Studio** - Local LLM with OpenAI-compatible API

### Configuration
```rust
// OpenAI
let config = LlmConfig::openai("api-key", "gpt-4");

// Ollama
let config = LlmConfig::ollama("http://localhost:11434/api/chat", "llama2");

// LM Studio
let config = LlmConfig::lmstudio("http://localhost:1234/v1/chat/completions", "model");
```

## Commands

### Create NPC
```
npc create <name> [template_id]
ncreate <name> [template_id]
```

### List NPCs
```
npc list [filter]
nlist [filter]
```

### Edit Properties
```
npc edit <uuid> <property> <value>
nedit <uuid> <property> <value>
```

**Properties:** `name`, `description`, `behavior`, `active`

**Example:**
```
npc edit 123e4567-e89b-12d3-a456-426614174000 behavior Friendly
npc edit 123e4567-e89b-12d3-a456-426614174000 active true
```

### Configure Dialogue
```
npc dialogue <uuid> <property> <value>
ndialogue <uuid> <property> <value>
```

**Properties:** `enabled`, `model`, `system_prompt`, `temperature`, `max_tokens`

**Example:**
```
npc dialogue <uuid> enabled true
npc dialogue <uuid> model gpt-4
npc dialogue <uuid> system_prompt "You are a grumpy blacksmith"
npc dialogue <uuid> temperature 0.8
```

### Configure GOAP
```
npc goap <uuid> <subcommand> [args]
ngoap <uuid> <subcommand> [args]
```

**Subcommands:**
- `addgoal <name> <priority>` - Add goal
- `addaction <name> <cost>` - Add action
- `setstate <key> <value>` - Set world state
- `show` - Display configuration

**Example:**
```
npc goap <uuid> addgoal patrol_area 5
npc goap <uuid> addaction move_to_waypoint 2.0
npc goap <uuid> setstate at_waypoint true
npc goap <uuid> show
```

## Best Practices

### GOAP Design
- Keep goals simple and focused
- Set action costs to reflect difficulty
- Use world state to track NPC knowledge
- Test with different starting states

### LLM Dialogue
- Write clear, specific system prompts
- Include personality in prompts
- Set temperature 0.7-0.9 for creative dialogue
- Always provide fallback responses
- Limit conversation history

### Performance
- Set appropriate update intervals (1-5 seconds)
- Disable inactive NPCs
- Use templates for common types
- Batch NPC updates

### Memory Management
- Mark important events with high importance
- Clean up old short-term memories
- Use entity references for relationships

## API Reference

See inline documentation:
- `server/src/ecs/components/npc.rs`
- `server/src/ecs/components/goap.rs`
- `server/src/ecs/systems/npc_ai.rs`
- `server/src/llm/`

## Testing
```bash
cargo test --test npc_integration_tests
```

## See Also
- [LLM Generation](LLM_GENERATION.md) - LLM-powered content
- [Combat System](COMBAT_SYSTEM.md) - NPC combat
- [Builder Commands](BUILDER_COMMANDS.md) - Creating NPCs
# Wyldlands Documentation

Documentation for the Wyldlands MUD server and systems.

## Core Systems

### [Combat System](COMBAT_SYSTEM.md)
Turn-based combat with status effects, defensive stances, and flee mechanics.
- Attack, defend, and flee commands
- Status effects (stunned, poisoned, burning, etc.)
- Damage and initiative calculations
- Combat API reference

### [NPC System](NPC_SYSTEM.md)
AI-controlled Non-Player Characters with advanced features.
- GOAP (Goal-Oriented Action Planning)
- LLM-powered dialogue (OpenAI, Ollama, LM Studio)
- Personality and memory systems
- NPC creation and configuration commands

### [LLM Generation](LLM_GENERATION.md)
AI-powered content generation for world building.
- Generate room descriptions
- Generate item details
- Generate NPC profiles
- Multiple LLM provider support

### [Builder Commands](BUILDER_COMMANDS.md)
World creation and editing commands.
- Area management
- Room creation and editing
- Exit configuration
- Item creation and templates

### [Help System](HELP_SYSTEM.md)
Database-driven in-game help system.
- Command help
- Topic organization
- Alias support
- Admin-only topics

## Configuration

### [Configuration Guide](CONFIGURATION.md)
Server configuration and setup instructions.

## Development

Development documentation is in the [development](development/) folder:
- [Development Plan](development/DEVELOPMENT_PLAN.md)
- [Project Status](development/PROJECT_STATUS.md)
- [Area/Room Editor Proposal](development/AREA_ROOM_EDITOR_PROPOSAL.md)

## Quick Links

### For Players
- [Help System](HELP_SYSTEM.md) - In-game help

### For Builders
- [Builder Commands](BUILDER_COMMANDS.md) - World building
- [LLM Generation](LLM_GENERATION.md) - AI content generation

### For Developers
- [Combat System](COMBAT_SYSTEM.md) - Combat API
- [NPC System](NPC_SYSTEM.md) - NPC AI API
- [Development Plan](development/DEVELOPMENT_PLAN.md) - Roadmap

## Getting Started

1. **Players**: Start with the [Help System](HELP_SYSTEM.md)
2. **Builders**: Read [Builder Commands](BUILDER_COMMANDS.md)
3. **Developers**: Check [Development Plan](development/DEVELOPMENT_PLAN.md)

## Documentation Structure

```
docs/
├── README.md                    # This file
├── COMBAT_SYSTEM.md            # Combat mechanics
├── NPC_SYSTEM.md               # NPC AI and dialogue
├── LLM_GENERATION.md           # AI content generation
├── BUILDER_COMMANDS.md         # World building commands
├── HELP_SYSTEM.md              # In-game help system
├── CONFIGURATION.md            # Server configuration
├── RECENT_ADDITIONS.md         # Recent changes
└── development/                # Development docs
    ├── DEVELOPMENT_PLAN.md     # Project roadmap
    ├── PROJECT_STATUS.md       # Current status
    └── ...                     # Other dev docs
```

## Contributing

When updating documentation:
1. Keep it concise and focused
2. Include practical examples
3. Update cross-references
4. Test code examples
5. Update this README if adding new docs

## See Also

- [Main README](../README.md) - Project overview
- [Server README](../server/README.md) - Server documentation
- [Gateway README](../gateway/README.md) - Gateway documentation
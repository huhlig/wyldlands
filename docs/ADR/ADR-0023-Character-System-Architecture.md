---
parent: ADR
nav_order: 0023
title: Character System Architecture
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0023: Character System Architecture

## Context and Problem Statement

Wyldlands requires a flexible and balanced character system that allows meaningful player choices during character creation while maintaining game balance. The system must support diverse character builds, provide clear progression paths, and integrate seamlessly with the ECS architecture.

How should we structure character attributes, talents, and skills to provide depth without overwhelming complexity?

## Decision Drivers

* **Balance**: Prevent min-maxing while allowing specialization
* **Clarity**: Players should understand what each choice means
* **Flexibility**: Support diverse character concepts and playstyles
* **Integration**: Work seamlessly with ECS and combat systems
* **Progression**: Provide clear advancement paths
* **Point-Buy System**: Fair and transparent character creation

## Considered Options

1. **Three-Aspect System (Body/Mind/Soul)** - Chosen option
2. **Traditional Six-Stat System** (STR, DEX, CON, INT, WIS, CHA)
3. **Skill-Only System** (no attributes, only skills)
4. **Class-Based System** (predefined character classes)

## Decision Outcome

Chosen option: **Three-Aspect System (Body/Mind/Soul)**, because it provides:
- Clear conceptual organization (physical, mental, spiritual)
- Balanced three-way interaction in combat and skills
- Unique identity distinct from traditional RPGs
- Natural integration with the game's thematic elements

### Implementation Details

#### Attribute Structure

Each of the three aspects (Body, Mind, Soul) has three sub-attributes:

| Aspect | Offence | Finesse | Defence |
|--------|---------|---------|---------|
| **Body** | Strength | Dexterity | Fortitude |
| **Mind** | Acuity | Focus | Resolve |
| **Soul** | Authority | Resonance | Permanence |

**Attribute Meanings:**
- **Offence**: Power - How strongly this aspect can act upon the world
- **Finesse**: Control - How precisely and efficiently power is applied
- **Defence**: Stability - How well this aspect resists disruption

#### Derived Statistics

Each aspect generates its own resource pools:

```rust
pub struct AttributeScores {
    pub score_offence: i32,    // Primary stat (10-20)
    pub score_finesse: i32,    // Primary stat (10-20)
    pub score_defence: i32,    // Primary stat (10-20)
    
    pub health_current: f32,   // Current pool
    pub health_maximum: f32,   // (offence + defence) * 10
    pub health_regen: f32,     // (finesse + defence) * 0.1
    
    pub energy_current: f32,   // Current pool
    pub energy_maximum: f32,   // (offence + finesse) * 10
    pub energy_regen: f32,     // (finesse + defence) * 0.1
}
```

**Resource Pools:**
- **Body**: Vitality (health) and Stamina (energy)
- **Mind**: Sanity (health) and Psyche (energy)
- **Soul**: Stability (health) and Aether (energy)

#### Point-Buy System

**Attribute/Talent Pool** (shared, default 100 points):
- Attributes start at rank 10 (baseline competence)
- Can be raised to rank 20 (exceptional mastery)
- Progressive cost structure:
  - Ranks 1-5: 1 point each
  - Ranks 6-10: 2 points each
  - Ranks 11-15: 3 points each
  - Ranks 16-20: 4 points each
- Talents cost 5-15 points depending on power level

**Skill Pool** (separate, default 50 points):
- Skills start at rank 0 (untrained)
- Can be raised to rank 10 (master level)
- Progressive cost: 1 point per rank initially, increasing with level
- Skills are specific applications of attributes

#### Talents System

Talents are special abilities that modify gameplay:
- **Prerequisites**: May require minimum attribute ranks or other talents
- **Categories**: Combat, Magic, Crafting, Social, Utility
- **Cost**: 5-15 points from attribute/talent pool
- **Examples**:
  - WeaponMaster: +2 to weapon damage
  - QuickReflexes: +1 initiative in combat
  - ManaEfficiency: -10% spell cost

#### Skills System

Skills represent trained abilities:
- **Attribute-Based**: Each skill is associated with an attribute
- **Experience Tracking**: Skills gain experience through use
- **Knowledge Cap**: Maximum skill level based on character level
- **Categories**: Combat, Magic, Crafting, Social, Knowledge

```rust
pub struct Skill {
    pub level: u8,              // Current skill level (0-10)
    pub experience: u32,        // Experience points
    pub knowledge: u32,         // Knowledge points (cap based on char level)
}
```

### Character Builder

The `CharacterBuilder` enforces all game rules:

```rust
pub struct CharacterBuilder {
    pub name: String,
    pub age: u16,
    pub nationality: Nationality,
    
    pub body_attributes: AttributeScores,
    pub mind_attributes: AttributeScores,
    pub soul_attributes: AttributeScores,
    
    pub talents: Talents,
    pub skills: Skills,
    
    pub attribute_talent_points: i32,  // Remaining points
    pub skill_points: i32,             // Remaining points
    
    pub max_attribute_talent_points: i32,  // From config
    pub max_skill_points: i32,             // From config
}
```

**Validation Rules:**
1. All points must be spent (or explicitly saved)
2. Attributes must be in valid range (10-20)
3. Skills must be in valid range (0-10)
4. Talent prerequisites must be met
5. Character must have a valid name and starting location

### Positive Consequences

* **Clear Organization**: Three aspects are easy to understand and remember
* **Balanced Choices**: No single "dump stat" - all aspects matter
* **Flexible Builds**: Support for physical, mental, and spiritual specialists
* **Natural Progression**: Clear path from novice (10) to master (20)
* **Fair Creation**: Point-buy prevents random stat rolls
* **ECS Integration**: Each aspect is a separate component for efficient queries

### Negative Consequences

* **Learning Curve**: Players must learn a new system (not D&D-like)
* **Complexity**: Nine primary attributes may seem overwhelming initially
* **Balance Challenges**: Requires careful tuning of costs and effects
* **Documentation Needs**: Requires clear explanation of the system

## Pros and Cons of the Options

### Three-Aspect System (Body/Mind/Soul)

* Good, because it provides thematic coherence with the game world
* Good, because it naturally supports diverse character concepts
* Good, because it prevents traditional min-maxing patterns
* Good, because it integrates well with ECS architecture
* Neutral, because it requires players to learn a new system
* Bad, because it may confuse players expecting traditional stats

### Traditional Six-Stat System

* Good, because players are familiar with it
* Good, because it's well-tested and balanced
* Neutral, because it's generic and lacks thematic identity
* Bad, because it doesn't align with the game's three-aspect theme
* Bad, because it encourages traditional dump stats (CHA, INT)

### Skill-Only System

* Good, because it's simple and straightforward
* Good, because it emphasizes player choices over random stats
* Neutral, because it works well for some game types
* Bad, because it lacks the depth of attribute-based systems
* Bad, because it makes character differentiation harder

### Class-Based System

* Good, because it simplifies character creation
* Good, because it ensures balanced starting characters
* Neutral, because it works well for some games
* Bad, because it limits player creativity and choice
* Bad, because it doesn't fit the sandbox nature of MUDs

## Validation

Implementation validated through:
1. **Unit Tests**: `server/tests/character_creation_integration_tests.rs` (15+ tests)
2. **Point-Buy Calculator**: Validates all point expenditures
3. **Attribute Validation**: Ensures all values in valid ranges
4. **Talent Prerequisites**: Checks all talent requirements
5. **Integration Tests**: Full character creation flow tested

## More Information

### Related Components

- `server/src/ecs/components/character/attributes.rs` - Attribute system
- `server/src/ecs/components/character/talents.rs` - Talent system
- `server/src/ecs/components/character/skills.rs` - Skill system
- `server/src/ecs/components/character/builder.rs` - Character builder
- `server/src/ecs/components/character/nationality.rs` - Nationality system

### Related ADRs

- [ADR-0004](ADR-0004-Use-Entity-Component-System.md) - ECS architecture
- [ADR-0011](ADR-0011-Character-Creation-System-Architecture.md) - Character creation flow
- [ADR-0012](ADR-0012-Session-State-Management-Strategy.md) - Session states

### Configuration

Character creation parameters are configurable in `server/config.yaml`:

```yaml
character_creation:
  max_attribute_talent_points: 100
  max_skill_points: 50
  min_attribute_rank: 10
  max_attribute_rank: 20
  max_skill_rank: 10
```

### Future Enhancements

1. **Racial Modifiers**: Different nationalities provide attribute bonuses
2. **Age Effects**: Age affects starting attributes and skills
3. **Background System**: Character backgrounds provide bonus skills/talents
4. **Advancement**: Post-creation attribute and skill progression
5. **Respec System**: Allow limited character rebuilding
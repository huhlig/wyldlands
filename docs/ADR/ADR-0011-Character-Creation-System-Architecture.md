---
parent: ADR
nav_order: 0011
title: Character Creation System Architecture
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0011: Character Creation System Architecture

## Context and Problem Statement

Players need to create characters with customizable attributes, talents, and skills. The character creation system must:
- Provide meaningful choices that affect gameplay
- Balance character power levels
- Validate all inputs server-side
- Support future expansion (races, classes, backgrounds)
- Persist character data reliably
- Integrate with the session state machine

How should we design the character creation system to be flexible, balanced, and maintainable?

## Decision Drivers

* **Game Balance**: Prevent overpowered or underpowered characters
* **Player Agency**: Meaningful choices during character creation
* **Server Authority**: All validation must happen server-side
* **Extensibility**: Easy to add new attributes, talents, and skills
* **Performance**: Character creation should be fast and responsive
* **Data Integrity**: Character data must be validated and consistent
* **User Experience**: Clear feedback on point allocation and validation

## Considered Options

* Point-Buy System with Shared Pools
* Class-Based Templates
* Random Generation with Rerolls
* Hybrid Point-Buy with Templates

## Decision Outcome

Chosen option: "Point-Buy System with Shared Pools", because it provides the best balance of player agency, game balance, and extensibility while maintaining server authority over all validation.

### Character Creation Architecture

**Three-Tier Attribute System:**
```
Body Attributes (Physical)
├── Offense  (10-20, starting at 10)
├── Finesse  (10-20, starting at 10)
└── Defense  (10-20, starting at 10)

Mind Attributes (Mental)
├── Offense  (10-20, starting at 10)
├── Finesse  (10-20, starting at 10)
└── Defense  (10-20, starting at 10)

Soul Attributes (Spiritual)
├── Offense  (10-20, starting at 10)
├── Finesse  (10-20, starting at 10)
└── Defense  (10-20, starting at 10)
```

**Point Pools:**
- **Attribute/Talent Pool**: Shared pool for attributes and talents (configurable, default: 100 points)
- **Skill Pool**: Separate pool for skills (configurable, default: 50 points)

**Cost Structure:**
- Attributes: Progressive cost (1 point for rank 11, 2 for rank 12, etc.)
- Talents: Fixed cost per talent (configurable)
- Skills: Progressive cost (1 point for rank 1, 2 for rank 2, etc.)

### Positive Consequences

* **Balanced Characters**: Point-buy prevents min-maxing extremes
* **Player Choice**: Players control their character's strengths
* **Server Authority**: All validation happens server-side
* **Extensibility**: Easy to add new attributes, talents, skills
* **Configurable**: Point pools and costs are configurable
* **Clear Feedback**: Players see remaining points in real-time
* **Respec Friendly**: Point-buy makes respeccing easier to implement

### Negative Consequences

* **Complexity**: More complex than class templates
* **Analysis Paralysis**: Players may struggle with too many choices
* **Balance Maintenance**: Requires ongoing tuning of costs and caps

## Pros and Cons of the Options

### Point-Buy System with Shared Pools

* Good, because players have full control over character build
* Good, because prevents extreme min-maxing
* Good, because easy to balance (adjust point costs)
* Good, because extensible (add new options without breaking balance)
* Good, because server-side validation is straightforward
* Neutral, because requires careful cost tuning
* Bad, because more complex than templates
* Bad, because can overwhelm new players

### Class-Based Templates

* Good, because simple for new players
* Good, because guaranteed balanced builds
* Good, because fast character creation
* Neutral, because less player agency
* Bad, because less flexible
* Bad, because harder to add new options
* Bad, because players may feel constrained

### Random Generation with Rerolls

* Good, because very fast
* Good, because simple to implement
* Neutral, because some players enjoy randomness
* Bad, because no player agency
* Bad, because can create unbalanced characters
* Bad, because frustrating for players who want specific builds

### Hybrid Point-Buy with Templates

* Good, because combines benefits of both
* Good, because templates help new players
* Neutral, because more complex to implement
* Bad, because still requires point-buy system
* Bad, because templates may become outdated

## Implementation Details

### Server-Side Character Builder

**Location:** `server/src/ecs/components/character/builder.rs`

```rust
pub struct CharacterBuilder {
    pub name: String,
    pub age: u16,
    pub nationality: Nationality,
    
    // Attribute allocations (rank 10-20)
    pub body_attributes: AttributeScores,
    pub mind_attributes: AttributeScores,
    pub soul_attributes: AttributeScores,
    
    // Talent and skill selections
    pub talents: Talents,
    pub skills: Skills,
    
    // Point pools
    pub attribute_talent_points: i32,
    pub skill_points: i32,
    pub max_attribute_talent_points: i32,
    pub max_skill_points: i32,
    
    pub starting_location_id: Option<String>,
}
```

### Attribute System

**Location:** `server/src/ecs/components/character/attributes.rs`

Each attribute class (Body, Mind, Soul) has:
- **Offense**: Damage/effect output
- **Finesse**: Accuracy/critical chance
- **Defense**: Damage reduction/resistance

Derived stats:
- **Health**: (Offense + Defense) × 10
- **Energy**: (Offense + Finesse) × 10
- **Health Regen**: (Finesse + Defense) × 0.1
- **Energy Regen**: (Finesse + Defense) × 0.1

### Validation Rules

1. **Name Validation**:
   - 3-20 characters
   - Letters, spaces, hyphens, apostrophes only
   - Must be unique (database check)

2. **Attribute Validation**:
   - Each attribute: 10-20 (starting at 10)
   - Total cost must not exceed attribute/talent pool
   - Progressive cost: rank 11 = 1 point, rank 12 = 2 points, etc.

3. **Talent Validation**:
   - Each talent has prerequisites (attributes, other talents)
   - Total cost must not exceed attribute/talent pool
   - No duplicate talents

4. **Skill Validation**:
   - Each skill: 0-10
   - Total cost must not exceed skill pool
   - Progressive cost: rank 1 = 1 point, rank 2 = 2 points, etc.

5. **Starting Location**:
   - Must be a valid starting location ID
   - Location must exist in database

### Database Schema

**Tables:**
- `characters`: Core character data (UUID, account_id, level, race, class)
- `identities`: Character names (entity_uuid, name) with unique constraint
- `entity_attributes`: Body/Mind/Soul attribute scores
- `entity_skills`: Skill ranks and experience
- `entity_metadata`: Talents stored as JSON

### Character Creation Flow

```
1. Client: CreateCharacter RPC
   ↓
2. Server: Create CharacterBuilder with default values
   ↓
3. Client: SetAttribute/SetTalent/SetSkill RPCs
   ↓
4. Server: Validate each change, update builder
   ↓
5. Client: FinalizeCharacter RPC
   ↓
6. Server: Final validation, create entity, persist to database
   ↓
7. Server: Transition to Playing state
```

## Validation

The character creation system is validated by:

1. **Unit Tests**: Attribute cost calculations, validation rules
2. **Integration Tests**: Full character creation flow (70+ tests)
3. **Database Constraints**: Unique names, foreign keys
4. **Server-Side Validation**: All inputs validated before persistence
5. **Point Pool Enforcement**: Cannot exceed allocated points

## More Information

### Configuration

Character creation is configurable via `server/config.yaml`:

```yaml
character_creation:
  max_attribute_talent_points: 100
  max_skill_points: 50
  min_attribute_rank: 10
  max_attribute_rank: 20
  max_skill_rank: 10
```

### Future Enhancements

1. **Races**: Different starting attributes and talents
2. **Classes**: Predefined templates for new players
3. **Backgrounds**: Story-based bonuses and starting equipment
4. **Respeccing**: Allow players to rebuild characters
5. **Attribute Caps**: Level-based caps on attributes
6. **Skill Specializations**: Advanced skill trees

### Related Decisions

- [ADR-0004](ADR-0004-Use-Entity-Component-System.md) - ECS enables flexible character components
- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - Database stores character data
- [ADR-0012](ADR-0012-Session-State-Management-Strategy.md) - Character creation is a session state

### References

- Character Builder: [server/src/ecs/components/character/builder.rs](../../server/src/ecs/components/character/builder.rs)
- Attributes: [server/src/ecs/components/character/attributes.rs](../../server/src/ecs/components/character/attributes.rs)
- Skills: [server/src/ecs/components/character/skills.rs](../../server/src/ecs/components/character/skills.rs)
- Talents: [server/src/ecs/components/character/talents.rs](../../server/src/ecs/components/character/talents.rs)
- Integration Tests: [server/tests/character_creation_integration_tests.rs](../../server/tests/character_creation_integration_tests.rs)
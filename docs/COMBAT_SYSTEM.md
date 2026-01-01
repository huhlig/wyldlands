# Combat System

Turn-based combat with status effects, defensive stances, and flee mechanics.

## Quick Start

```
attack <target>    # Start combat and attack
defend             # Take defensive stance (+defense for 1 round)
flee               # Attempt to escape combat
combat             # View combat status
```

## Components

### Combatant
Tracks combat state for entities:
- `in_combat` - Currently fighting
- `target_id` - Current opponent
- `initiative` - Turn order
- `action_cooldown` - Time between actions (default: 1.0s)
- `is_defending` - Defensive stance active
- `defense_bonus` - Defense bonus from defending

### StatusEffects
Temporary effects on entities:
- **Stunned** - Cannot act or flee
- **Poisoned** - Damage over time
- **Burning** - Fire damage over time
- **Bleeding** - Physical damage over time
- **Defending** - Increased defense (from defend command)
- **Weakened** - Reduced attack power
- **Strengthened** - Increased attack power
- **Slowed** - Reduced action speed
- **Hasted** - Increased action speed

## Commands

### attack / kill / k
```
attack <target>
```
Initiates combat with a target in the same room. Performs immediate attack with 10% critical hit chance (2x damage).

### defend / def
```
defend
```
Takes defensive stance for one round. Grants defense bonus: `5 + (Defence - 10) / 2`

### flee / run
```
flee
```
Attempts to escape combat. Success chance: `50% + (Finesse - 10) * 2%`. Cannot flee while Stunned.

### combat / c
```
combat
```
Shows current combat status: target, health, defending status, and active effects.

## Mechanics

### Damage Calculation
```
base_damage = 10 + (Offence - 10) / 2 + weapon_damage
critical_hit (10% chance) = damage * 2
minimum_damage = 1
```

### Initiative
```
initiative = 10 + (Finesse - 10) / 2 + random(0-10)
```

### Combat Flow
1. Attack command starts combat
2. Both entities get Combatant components
3. Actions occur every 1.0 seconds (cooldown)
4. Status effects update each frame
5. Combat ends when one dies or flees

## API

### CombatSystem Methods
```rust
// Start combat
start_combat_with_registry(world, registry, attacker, defender) -> Result<(), String>

// Combat actions
attack(world, attacker, defender) -> Option<AttackResult>
defend(world, entity) -> Result<(), String>
flee(world, entity) -> Result<bool, String>

// Updates
update_with_registry(world, registry, delta_time)
update_status_effects(world, delta_time)
```

### AttackResult
```rust
pub struct AttackResult {
    pub hit: bool,
    pub damage: i32,
    pub critical: bool,
}
```

## Events
Published through EventBus:
- `CombatStarted` - Combat begins
- `EntityAttacked` - Attack performed
- `EntityDefended` - Defensive stance taken
- `EntityFled` - Successful flee
- `EntityDied` - Entity dies

## Testing
```bash
cargo test --test combat_integration_tests
```

## See Also
- [NPC System](NPC_SYSTEM.md) - AI combatants
- [Builder Commands](BUILDER_COMMANDS.md) - Creating NPCs
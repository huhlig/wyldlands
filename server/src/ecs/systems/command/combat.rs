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

//! Combat command handlers

use crate::ecs::EcsEntity;
use crate::ecs::components::{BodyAttributes, Combatant, Location, Name, StatusEffects};
use crate::ecs::context::WorldContext;
use crate::ecs::events::EventBus;
use crate::ecs::systems::CombatSystem;
use hecs::Entity;
use std::sync::Arc;

/// Handle the attack command
pub async fn handle_attack(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    args: &[String],
) -> Result<String, String> {
    if args.is_empty() {
        return Ok("Attack who?".to_string());
    }

    let target_name = args[0].to_lowercase();

    // Find target and get info in read lock
    let (target, target_display_name) = {
        let world = context.entities().read().await;

        // Find the attacker's location
        let attacker_location = world
            .get::<&Location>(entity)
            .map_err(|_| "You don't have a location".to_string())?;
        let attacker_area = attacker_location.area_id;
        let attacker_room = attacker_location.room_id;

        // Find target in the same room
        let mut target_entity = None;
        for (e, name, location) in world.query::<(Entity, &Name, &Location)>().iter() {
            if e != entity
                && location.area_id == attacker_area
                && location.room_id == attacker_room
                && name.matches(&target_name)
            {
                target_entity = Some(e);
                break;
            }
        }

        let target =
            target_entity.ok_or_else(|| format!("You don't see '{}' here.", target_name))?;

        // Check if target is a valid combatant
        if world.get::<&Combatant>(target).is_err() {
            return Err(format!("You cannot attack that."));
        }

        // Get target name for message
        let target_display_name = world
            .get::<&Name>(target)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "someone".to_string());

        (target, target_display_name)
    };

    // Start combat using the combat system
    let mut world = context.entities().write().await;
    let registry = context.registry().read().await;
    let event_bus = EventBus::new();
    let mut combat_system = CombatSystem::new(event_bus);

    combat_system
        .start_combat_with_registry(&mut world, &registry, entity, target)
        .map_err(|e| format!("Failed to start combat: {}", e))?;

    // Perform the first attack
    let result = combat_system.attack(&mut world, entity, target);

    if let Some(attack_result) = result {
        let crit_msg = if attack_result.critical {
            " *CRITICAL HIT*"
        } else {
            ""
        };
        Ok(format!(
            "You attack {} for {} damage!{}",
            target_display_name, attack_result.damage, crit_msg
        ))
    } else {
        Err("Attack failed.".to_string())
    }
}

/// Handle the defend command
pub async fn handle_defend(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _args: &[String],
) -> Result<String, String> {
    let world = context.entities().read().await;

    // Check if in combat
    let in_combat = world
        .get::<&Combatant>(entity)
        .map(|c| c.in_combat)
        .unwrap_or(false);

    if !in_combat {
        return Err("You are not in combat.".to_string());
    }

    drop(world);

    // Apply defend
    let mut world = context.entities().write().await;
    let event_bus = EventBus::new();
    let mut combat_system = CombatSystem::new(event_bus);

    combat_system.defend(&mut world, entity)?;

    // Get defense bonus for message
    let defense_bonus = world
        .get::<&Combatant>(entity)
        .map(|c| c.defense_bonus)
        .unwrap_or(0);

    Ok(format!(
        "You take a defensive stance, gaining +{} defense for this round.",
        defense_bonus
    ))
}

/// Handle the flee command
pub async fn handle_flee(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _args: &[String],
) -> Result<String, String> {
    let world = context.entities().read().await;

    // Check if in combat
    let in_combat = world
        .get::<&Combatant>(entity)
        .map(|c| c.in_combat)
        .unwrap_or(false);

    if !in_combat {
        return Err("You are not in combat.".to_string());
    }

    // Check for status effects that prevent fleeing
    if let Ok(status_effects) = world.get::<&StatusEffects>(entity) {
        if status_effects.has_effect(crate::ecs::components::StatusEffectType::Stunned) {
            return Err("You are stunned and cannot flee!".to_string());
        }
    }

    drop(world);

    // Attempt to flee
    let mut world = context.entities().write().await;
    let event_bus = EventBus::new();
    let mut combat_system = CombatSystem::new(event_bus);

    let success = combat_system.flee(&mut world, entity)?;

    if success {
        Ok("You successfully flee from combat!".to_string())
    } else {
        Err("You failed to flee!".to_string())
    }
}

/// Handle the combat status command
pub async fn handle_combat_status(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _args: &[String],
) -> Result<String, String> {
    let world = context.entities().read().await;

    let combatant = world
        .get::<&Combatant>(entity)
        .map_err(|_| "You are not a combatant.".to_string())?;

    if !combatant.in_combat {
        return Ok("You are not in combat.".to_string());
    }

    let mut status = String::from("=== Combat Status ===\n");

    // Show target
    if let Some(target_id) = combatant.target_id {
        let registry = context.registry().read().await;
        if let Some(target_entity) = registry.get_entity(target_id.uuid) {
            if let Ok(target_name) = world.get::<&Name>(target_entity) {
                status.push_str(&format!("Target: {}\n", target_name.display));
            }
        }
    }

    // Show health
    if let Ok(attrs) = world.get::<&BodyAttributes>(entity) {
        status.push_str(&format!(
            "Health: {:.0}/{:.0}\n",
            attrs.health_current, attrs.health_maximum
        ));
    }

    // Show defending status
    if combatant.is_defending {
        status.push_str(&format!(
            "Defending: +{} defense\n",
            combatant.defense_bonus
        ));
    }

    // Show status effects
    if let Ok(status_effects) = world.get::<&StatusEffects>(entity) {
        if !status_effects.effects.is_empty() {
            status.push_str("\nStatus Effects:\n");
            for effect in &status_effects.effects {
                status.push_str(&format!(
                    "  - {} ({:.1}s remaining)\n",
                    effect.effect_type.as_str(),
                    effect.duration
                ));
            }
        }
    }

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::GameWorld;
    use crate::ecs::components::EntityUuid;

    #[test]
    fn test_combat_commands_exist() {
        // Simple test to verify the module compiles
        // Full integration tests will be in the integration test file
        assert!(true);
    }
}



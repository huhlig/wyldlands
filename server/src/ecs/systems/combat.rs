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

//! Combat system for fighting mechanics

use crate::ecs::components::{
    AttributeScores, Combatant, EntityId, EntityUuid, EquipSlot, Equipment, StatusEffect,
    StatusEffectType, StatusEffects,
};
use crate::ecs::events::{EventBus, GameEvent};
use crate::ecs::registry::EntityRegistry;
use crate::ecs::{EcsEntity, GameWorld};
use tracing::instrument;
use hecs::Entity;
use std::collections::HashMap;

pub struct CombatSystem {
    event_bus: EventBus,
    target_map: HashMap<EcsEntity, EcsEntity>,
}

#[derive(Debug, Clone)]
pub struct AttackResult {
    pub hit: bool,
    pub damage: i32,
    pub critical: bool,
}

impl CombatSystem {
    /// Create a new combat system
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            event_bus,
            target_map: HashMap::new(),
        }
    }

    /// Start combat between two entities
    pub fn start_combat(
        &mut self,
        world: &mut GameWorld,
        attacker: EcsEntity,
        defender: EcsEntity,
    ) {
        // Set attacker in combat and store defender EntityId
        if let Ok(defender_uuid) = world.get::<&EntityUuid>(defender) {
            if let Ok(mut combatant) = world.get::<&mut Combatant>(attacker) {
                combatant.in_combat = true;
                combatant.target_id = Some(EntityId::new(defender, defender_uuid.0));
            }
        }

        // Set defender in combat and store attacker EntityId
        if let Ok(attacker_uuid) = world.get::<&EntityUuid>(attacker) {
            if let Ok(mut combatant) = world.get::<&mut Combatant>(defender) {
                combatant.in_combat = true;
                combatant.target_id = Some(EntityId::new(attacker, attacker_uuid.0));
            }
        }

        // Track targets in the system
        self.target_map.insert(attacker, defender);
        self.target_map.insert(defender, attacker);

        self.event_bus
            .publish(GameEvent::CombatStarted { attacker, defender });
    }

    /// Start combat using EntityRegistry for proper UUID management
    pub fn start_combat_with_registry(
        &mut self,
        world: &mut GameWorld,
        registry: &EntityRegistry,
        attacker: EcsEntity,
        defender: EcsEntity,
    ) -> Result<(), String> {
        // Get EntityIds from registry
        let defender_id = registry
            .get_entity_id(defender)
            .ok_or_else(|| "Defender not registered".to_string())?;
        let attacker_id = registry
            .get_entity_id(attacker)
            .ok_or_else(|| "Attacker not registered".to_string())?;

        // Set attacker in combat
        if let Ok(mut combatant) = world.get::<&mut Combatant>(attacker) {
            combatant.in_combat = true;
            combatant.target_id = Some(defender_id);
        }

        // Set defender in combat
        if let Ok(mut combatant) = world.get::<&mut Combatant>(defender) {
            combatant.in_combat = true;
            combatant.target_id = Some(attacker_id);
        }

        // Track targets in the system
        self.target_map.insert(attacker, defender);
        self.target_map.insert(defender, attacker);

        self.event_bus
            .publish(GameEvent::CombatStarted { attacker, defender });

        Ok(())
    }

    /// End combat for an entity
    pub fn end_combat(&mut self, world: &mut GameWorld, entity: EcsEntity) {
        if let Ok(mut combatant) = world.get::<&mut Combatant>(entity) {
            combatant.in_combat = false;
            combatant.target_id = None;
        }
        self.target_map.remove(&entity);
    }

    /// Perform an attack
    #[instrument(skip(self, world))]
    pub fn attack(
        &mut self,
        world: &mut GameWorld,
        attacker: EcsEntity,
        defender: EcsEntity,
    ) -> Option<AttackResult> {
        // Calculate base damage
        let mut damage = 10; // Base damage

        // Add offence score modifier
        if let Ok(attrs) = world.get::<&AttributeScores>(attacker) {
            damage += (attrs.score_offence - 10) / 2; // Modifier calculation
        }

        // Add weapon damage
        if let Ok(equipment) = world.get::<&Equipment>(attacker) {
            if let Some(weapon_id) = equipment.get(EquipSlot::MainHand) {
                // Convert Uuid to EcsEntity - for now just use base damage
                // TODO: Properly handle weapon entity lookup
                damage += 5; // Placeholder weapon damage
            }
        }

        // Ensure minimum damage
        damage = damage.max(1);

        // Check for critical hit (10% chance)
        let critical = rand::random::<f32>() < 0.1;
        if critical {
            damage *= 2;
        }

        // Apply damage to defender
        let mut target_died = false;
        if let Ok(mut health) = world.get::<&mut AttributeScores>(defender) {
            health.health_current = (health.health_current - damage as f32).max(0.0);
            target_died = health.health_current <= 0.0;

            self.event_bus.publish(GameEvent::EntityAttacked {
                attacker,
                defender,
                damage,
            });
        } else {
            return None;
        }

        // Handle death
        if target_died {
            self.event_bus.publish(GameEvent::EntityDied {
                entity: defender,
                killer: Some(attacker),
            });

            // End combat for both entities
            self.end_combat(world, attacker);
            self.end_combat(world, defender);
        }

        Some(AttackResult {
            hit: true,
            damage,
            critical,
        })
    }

    /// Update the combat system
    #[instrument(skip(self, world))]
    pub fn update(&mut self, world: &mut GameWorld, delta_time: f32) {
        let mut attacks = Vec::new();
        let mut _dead_targets: Vec<hecs::Entity> = Vec::new();

        // Find entities ready to attack and check target health
        for (entity, combatant) in world.query_mut::<(Entity, &mut Combatant)>() {
            if combatant.in_combat {
                combatant.update_timer(delta_time);

                if combatant.can_attack() {
                    if let Some(&target) = self.target_map.get(&entity) {
                        attacks.push((entity, target));
                        combatant.reset_timer();
                    }
                }
            }
        }

        // Check which targets are alive (separate pass to avoid borrow issues)
        attacks.retain(|(_, target)| {
            if let Ok(health) = world.get::<&AttributeScores>(*target) {
                health.health_current > 0.0
            } else {
                false
            }
        });

        // Execute attacks
        for (attacker, defender) in attacks {
            self.attack(world, attacker, defender);
        }

        // Note: Target cleanup via UUID->Entity lookup requires registry access
        // This is handled by the start_combat_with_registry method
        // For now, the target_map provides runtime target tracking
    }

    /// Update with registry for proper UUID->Entity resolution
    /// Note: With EntityId in components, we can use the entity handle directly,
    /// but we verify it against the registry to ensure it's still valid
    pub fn update_with_registry(
        &mut self,
        world: &mut GameWorld,
        registry: &EntityRegistry,
        delta_time: f32,
    ) {
        let mut attacks = Vec::new();

        // Find entities ready to attack
        for (entity, combatant) in world.query_mut::<(Entity, &mut Combatant)>() {
            if combatant.in_combat {
                combatant.update_timer(delta_time);

                if combatant.can_attack() {
                    // Get target from EntityId
                    if let Some(target_id) = combatant.target_id {
                        // Verify the entity is still in the registry
                        if registry.contains_entity(target_id.entity()) {
                            attacks.push((entity, target_id.entity()));
                            combatant.reset_timer();
                        }
                    }
                }
            }
        }

        // Check which targets are alive
        attacks.retain(|(_, target)| {
            if let Ok(health) = world.get::<&AttributeScores>(*target) {
                health.health_current > 0.0
            } else {
                false
            }
        });

        // Execute attacks
        for (attacker, defender) in attacks {
            self.attack(world, attacker, defender);
        }
    }

    /// Calculate initiative for combat order
    pub fn calculate_initiative(&self, world: &GameWorld, entity: EcsEntity) -> i32 {
        let mut initiative = 10; // Base initiative

        if let Ok(attrs) = world.get::<&AttributeScores>(entity) {
            initiative += (attrs.score_finesse - 10) / 2;
        }

        // Add random component
        initiative += (rand::random::<f32>() * 10.0) as i32;

        initiative
    }

    /// Start defending - increases defense for one round
    pub fn defend(&mut self, world: &mut GameWorld, entity: EcsEntity) -> Result<(), String> {
        // Calculate defense bonus based on attributes
        let defense_bonus = if let Ok(attrs) = world.get::<&AttributeScores>(entity) {
            5 + (attrs.score_defence - 10) / 2
        } else {
            5
        };

        // Set defending state
        if let Ok(mut combatant) = world.get::<&mut Combatant>(entity) {
            combatant.start_defending(defense_bonus);
        } else {
            return Err("Entity is not a combatant".to_string());
        }

        // Add defending status effect
        if let Ok(mut status_effects) = world.get::<&mut StatusEffects>(entity) {
            status_effects.add_effect(StatusEffect::new(
                StatusEffectType::Defending,
                1.0, // Lasts one round
                defense_bonus,
            ));
        } else {
            // Create status effects component if it doesn't exist
            let mut status_effects = StatusEffects::new();
            status_effects.add_effect(StatusEffect::new(
                StatusEffectType::Defending,
                1.0,
                defense_bonus,
            ));
            world
                .insert_one(entity, status_effects)
                .map_err(|e| format!("Failed to add status effects: {}", e))?;
        }

        self.event_bus.publish(GameEvent::EntityDefended { entity });
        Ok(())
    }

    /// Attempt to flee from combat
    pub fn flee(&mut self, world: &mut GameWorld, entity: EcsEntity) -> Result<bool, String> {
        // Check if entity is in combat
        let in_combat = if let Ok(combatant) = world.get::<&Combatant>(entity) {
            combatant.in_combat
        } else {
            return Err("Entity is not a combatant".to_string());
        };

        if !in_combat {
            return Err("Entity is not in combat".to_string());
        }

        // Calculate flee chance based on finesse
        let flee_chance = if let Ok(attrs) = world.get::<&AttributeScores>(entity) {
            0.5 + (attrs.score_finesse as f32 - 10.0) * 0.02
        } else {
            0.5
        };

        // Check for status effects that prevent fleeing
        if let Ok(status_effects) = world.get::<&StatusEffects>(entity) {
            if status_effects.has_effect(StatusEffectType::Stunned) {
                return Ok(false);
            }
        }

        // Roll for flee success
        let success = rand::random::<f32>() < flee_chance;

        if success {
            // End combat for this entity
            self.end_combat(world, entity);
            self.event_bus.publish(GameEvent::EntityFled { entity });
        }

        Ok(success)
    }

    /// Update status effects for all entities
    pub fn update_status_effects(&mut self, world: &mut GameWorld, delta_time: f32) {
        let mut expired_defending: Vec<EcsEntity> = Vec::new();

        // Update all status effects
        for (entity, status_effects) in world.query_mut::<(Entity, &mut StatusEffects)>() {
            status_effects.update(delta_time);

            // Check if defending expired
            if !status_effects.has_effect(StatusEffectType::Defending) {
                expired_defending.push(entity);
            }
        }

        // Remove defending state from combatants whose effect expired
        for entity in expired_defending {
            if let Ok(mut combatant) = world.get::<&mut Combatant>(entity) {
                if combatant.is_defending {
                    combatant.stop_defending();
                }
            }
        }
    }
}

// Simple random number generator for tests
mod rand {
    use std::cell::Cell;

    thread_local! {
        static SEED: Cell<u64> = Cell::new(12345);
    }

    pub fn random<T>() -> T
    where
        T: From<f32>,
    {
        SEED.with(|seed| {
            let mut s = seed.get();
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            seed.set(s);
            T::from((s / 65536) as f32 / 32768.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Name;

    #[test]
    fn test_combat_system_creation() {
        let event_bus = EventBus::new();
        let _system = CombatSystem::new(event_bus);
    }

    #[test]
    fn test_start_combat() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = CombatSystem::new(event_bus);

        let attacker = world.spawn((Name::new("Attacker"), Combatant::new(), EntityUuid::new()));

        let defender = world.spawn((Name::new("Defender"), Combatant::new(), EntityUuid::new()));

        system.start_combat(&mut world, attacker, defender);

        let attacker_combat = world.get::<&Combatant>(attacker).unwrap();
        assert!(attacker_combat.in_combat);
        assert!(attacker_combat.target_id.is_some());

        let defender_combat = world.get::<&Combatant>(defender).unwrap();
        assert!(defender_combat.in_combat);
        assert!(defender_combat.target_id.is_some());
    }

    #[test]
    fn test_attack() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = CombatSystem::new(event_bus);

        let attacker = world.spawn((
            Name::new("Attacker"),
            Combatant::new(),
            AttributeScores::new(),
        ));

        let defender = world.spawn((
            Name::new("Defender"),
            Combatant::new(),
            AttributeScores::new(),
        ));

        let result = system.attack(&mut world, attacker, defender);
        assert!(result.is_some());

        let result = result.unwrap();
        assert!(result.hit);
        assert!(result.damage > 0);

        let health = world.get::<&AttributeScores>(defender).unwrap();
        assert!(health.health_current < 100.0);
    }

    #[test]
    fn test_death() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = CombatSystem::new(event_bus);

        let attacker = world.spawn((
            Name::new("Attacker"),
            Combatant::new(),
            AttributeScores::new(),
        ));

        let mut defender_attrs = AttributeScores::new();
        defender_attrs.health_maximum = 1.0;
        defender_attrs.health_current = 1.0;
        let defender = world.spawn((Name::new("Defender"), Combatant::new(), defender_attrs));

        system.start_combat(&mut world, attacker, defender);
        system.attack(&mut world, attacker, defender);

        let health = world.get::<&AttributeScores>(defender).unwrap();
        assert!(health.health_current <= 0.0);

        // Combat should end after death
        let attacker_combat = world.get::<&Combatant>(attacker).unwrap();
        assert!(!attacker_combat.in_combat);
    }

    #[test]
    fn test_combat_update() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = CombatSystem::new(event_bus);

        let attacker = world.spawn((
            Name::new("Attacker"),
            Combatant::new(),
            AttributeScores::new(),
            EntityUuid::new(),
        ));

        let defender = world.spawn((
            Name::new("Defender"),
            Combatant::new(),
            AttributeScores::new(),
            EntityUuid::new(),
        ));

        system.start_combat(&mut world, attacker, defender);

        // Update should trigger attacks after cooldown
        system.update(&mut world, 1.0);

        let health = world.get::<&AttributeScores>(defender).unwrap();
        assert!(health.health_current < 100.0);
    }

    #[test]
    fn test_initiative() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let system = CombatSystem::new(event_bus);

        let entity = world.spawn((Name::new("Test"), AttributeScores::new()));

        let initiative = system.calculate_initiative(&world, entity);
        assert!(initiative > 0);
    }
}

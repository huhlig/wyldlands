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

//! Combat system integration tests

use wyldlands_server::ecs::components::*;
use wyldlands_server::ecs::events::EventBus;
use wyldlands_server::ecs::registry::EntityRegistry;
use wyldlands_server::ecs::systems::CombatSystem;
use wyldlands_server::ecs::GameWorld;

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

    let attacker = world.spawn((
        Name::new("Attacker"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    let defender = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    system.start_combat(&mut world, attacker, defender);

    let attacker_combat = world.get::<&Combatant>(attacker).unwrap();
    assert!(attacker_combat.in_combat);
    assert!(attacker_combat.target_id.is_some());

    let defender_combat = world.get::<&Combatant>(defender).unwrap();
    assert!(defender_combat.in_combat);
    assert!(defender_combat.target_id.is_some());
}

#[test]
fn test_attack_deals_damage() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let attacker = world.spawn((
        Name::new("Attacker"),
        Combatant::new(),
        BodyAttributes::new(),
    ));

    let defender = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        BodyAttributes::new(),
    ));

    let initial_health = world.get::<&BodyAttributes>(defender).unwrap().health_current;

    let result = system.attack(&mut world, attacker, defender);
    assert!(result.is_some());

    let result = result.unwrap();
    assert!(result.hit);
    assert!(result.damage > 0);

    let health = world.get::<&BodyAttributes>(defender).unwrap();
    assert!(health.health_current < initial_health);
}

#[test]
fn test_defend_increases_defense() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let entity = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        BodyAttributes::new(),
    ));

    // Start combat first
    let enemy = world.spawn((
        Name::new("Enemy"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    system.start_combat(&mut world, entity, enemy);

    // Now defend
    let result = system.defend(&mut world, entity);
    assert!(result.is_ok());

    let combatant = world.get::<&Combatant>(entity).unwrap();
    assert!(combatant.is_defending);
    assert!(combatant.defense_bonus > 0);

    // Check status effect was added
    let status_effects = world.get::<&StatusEffects>(entity).unwrap();
    assert!(status_effects.has_effect(StatusEffectType::Defending));
}

#[test]
fn test_flee_mechanics() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let entity = world.spawn((
        Name::new("Fleeing"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    let enemy = world.spawn((
        Name::new("Enemy"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    // Test that flee fails when not in combat
    let result = system.flee(&mut world, entity);
    assert!(result.is_err());

    // Start combat
    system.start_combat(&mut world, entity, enemy);

    // Test that flee returns a result (success or failure)
    let result = system.flee(&mut world, entity);
    assert!(result.is_ok());
    
    // If flee succeeded, entity should not be in combat
    let combatant = world.get::<&Combatant>(entity).unwrap();
    if !combatant.in_combat {
        // Flee was successful
        assert!(combatant.target_id.is_none());
    }
}

#[test]
fn test_status_effects_update() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let entity = world.spawn((
        Name::new("Test"),
        Combatant::new(),
        BodyAttributes::new(),
    ));

    // Add a status effect
    let mut status_effects = StatusEffects::new();
    status_effects.add_effect(StatusEffect::new(
        StatusEffectType::Defending,
        1.0,
        5,
    ));
    world.insert_one(entity, status_effects).unwrap();

    // Update for 0.5 seconds
    system.update_status_effects(&mut world, 0.5);

    // Effect should still be active
    {
        let status_effects = world.get::<&StatusEffects>(entity).unwrap();
        assert!(status_effects.has_effect(StatusEffectType::Defending));
    }

    // Update for another 0.6 seconds (total 1.1 seconds)
    system.update_status_effects(&mut world, 0.6);

    // Effect should have expired
    {
        let status_effects = world.get::<&StatusEffects>(entity).unwrap();
        assert!(!status_effects.has_effect(StatusEffectType::Defending));
    }
}

#[test]
fn test_combat_death() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let attacker = world.spawn((
        Name::new("Attacker"),
        Combatant::new(),
        BodyAttributes::new(),
    ));

    let mut defender_attrs = BodyAttributes::new();
    defender_attrs.health_maximum = 1.0;
    defender_attrs.health_current = 1.0;
    let defender = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        defender_attrs,
    ));

    system.start_combat(&mut world, attacker, defender);
    system.attack(&mut world, attacker, defender);

    let health = world.get::<&BodyAttributes>(defender).unwrap();
    assert!(health.health_current <= 0.0);

    // Combat should end after death
    let attacker_combat = world.get::<&Combatant>(attacker).unwrap();
    assert!(!attacker_combat.in_combat);
}

#[test]
fn test_combat_with_registry() {
    let mut world = GameWorld::new();
    let mut registry = EntityRegistry::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let attacker_uuid = uuid::Uuid::new_v4();
    let attacker = world.spawn((
        Name::new("Attacker"),
        Combatant::new(),
        EntityUuid(attacker_uuid),
        BodyAttributes::new(),
    ));
    registry.register(attacker, attacker_uuid).unwrap();

    let defender_uuid = uuid::Uuid::new_v4();
    let defender = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        EntityUuid(defender_uuid),
        BodyAttributes::new(),
    ));
    registry.register(defender, defender_uuid).unwrap();

    let result = system.start_combat_with_registry(&mut world, &registry, attacker, defender);
    assert!(result.is_ok());

    let attacker_combat = world.get::<&Combatant>(attacker).unwrap();
    assert!(attacker_combat.in_combat);
    assert!(attacker_combat.target_id.is_some());
}

#[test]
fn test_initiative_calculation() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let system = CombatSystem::new(event_bus);

    let entity = world.spawn((
        Name::new("Test"),
        BodyAttributes::new(),
    ));

    let initiative = system.calculate_initiative(&world, entity);
    assert!(initiative > 0);
}

#[test]
fn test_combat_rounds() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let attacker = world.spawn((
        Name::new("Attacker"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    let defender = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    system.start_combat(&mut world, attacker, defender);

    // Simulate several combat rounds
    for _ in 0..5 {
        system.update(&mut world, 1.0);
    }

    // Defender should have taken damage
    let health = world.get::<&BodyAttributes>(defender).unwrap();
    assert!(health.health_current < 100.0);
}

#[test]
fn test_status_effect_types() {
    let mut status_effects = StatusEffects::new();

    // Add multiple effects
    status_effects.add_effect(StatusEffect::new(StatusEffectType::Stunned, 2.0, 0));
    status_effects.add_effect(StatusEffect::new(StatusEffectType::Poisoned, 5.0, 2));
    status_effects.add_effect(StatusEffect::new(StatusEffectType::Burning, 3.0, 5));

    assert!(status_effects.has_effect(StatusEffectType::Stunned));
    assert!(status_effects.has_effect(StatusEffectType::Poisoned));
    assert!(status_effects.has_effect(StatusEffectType::Burning));
    assert!(!status_effects.has_effect(StatusEffectType::Defending));

    // Remove one effect
    status_effects.remove_effect(StatusEffectType::Stunned);
    assert!(!status_effects.has_effect(StatusEffectType::Stunned));
    assert!(status_effects.has_effect(StatusEffectType::Poisoned));
}

#[test]
fn test_defending_stops_after_duration() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut system = CombatSystem::new(event_bus);

    let entity = world.spawn((
        Name::new("Defender"),
        Combatant::new(),
        BodyAttributes::new(),
    ));

    let enemy = world.spawn((
        Name::new("Enemy"),
        Combatant::new(),
        EntityUuid::new(),
        BodyAttributes::new(),
    ));

    system.start_combat(&mut world, entity, enemy);
    system.defend(&mut world, entity).unwrap();

    // Defending should be active
    {
        let combatant = world.get::<&Combatant>(entity).unwrap();
        assert!(combatant.is_defending);
    }

    // Update for more than 1 second
    system.update_status_effects(&mut world, 1.5);

    // Defending should have stopped
    {
        let combatant = world.get::<&Combatant>(entity).unwrap();
        assert!(!combatant.is_defending);
        assert_eq!(combatant.defense_bonus, 0);
    }
}



//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

use crate::ecs::components::*;
use crate::ecs::context::WorldContext;
use crate::ecs::systems::CommandResult;
use crate::ecs::{EcsEntity, GameWorld};
use std::sync::Arc;

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn world_save_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!(
        "World Save Command from {}: {}",
        entity.id(),
        args.join(" ")
    );

    // Access world for read to count persistent entities
    let world = context.entities().read().await;

    // Count all persistent entities
    let mut persistent_count = 0;
    for (_ent, _) in world.query::<&Persistent>().iter() {
        persistent_count += 1;
    }

    drop(world); // Release the read lock

    // Note: Actual saving happens via the PersistenceManager's auto-save mechanism
    // or when entities are marked dirty. This command provides feedback to the admin.
    let output = format!(
        "\r\nWorld Save Initiated\r\n{}\r\n\r\n\
         Found {} persistent entities in the world.\r\n\r\n\
         Note: Entities are automatically saved via the auto-save system.\r\n\
         To force immediate save, use the persistence manager directly.\r\n\r\n\
         {}\r\n",
        "=".repeat(80),
        persistent_count,
        "=".repeat(80)
    );

    CommandResult::Success(output)
}

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn world_reload_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::info!(
        "World Reload Command from {}: {}",
        entity.id(),
        args.join(" ")
    );

    // First, get the admin's account_id to preserve them
    let admin_account_id = {
        let world = context.entities().read().await;
        let avatar_result = world.get::<&Avatar>(entity).map(|avatar| avatar.account_id);
        drop(world); // Release lock before checking result

        match avatar_result {
            Ok(account_id) => account_id,
            Err(_) => {
                return CommandResult::Failure(
                    "Error: Cannot reload world - you are not an avatar entity".to_string()
                );
            }
        }
    };

    // Count entities before reload
    let entities_before = {
        let world = context.entities().read().await;
        world.query::<&EntityUuid>().iter().count()
    };

    // Perform the reload
    let reload_result = {
        // Get write access to world
        let mut world = context.entities().write().await;

            // Despawn all entities except the admin
            let mut entities_to_despawn = Vec::new();
            for (ent, _) in world.query::<&EntityUuid>().iter() {
                // Check if this is the admin
                if let Ok(avatar) = world.get::<&Avatar>(ent) {
                    if avatar.account_id == admin_account_id {
                        continue; // Skip the admin
                    }
                }
                entities_to_despawn.push(ent);
            }

            let despawned_count = entities_to_despawn.len();
            for ent in entities_to_despawn {
                let _ = world.despawn(ent);
            }

        drop(world); // Release write lock before loading

        // Load entities from database
        context.load().await.map(|loaded_count| (despawned_count, loaded_count))
    };

    match reload_result {
        Ok((despawned, loaded)) => {
            let output = format!(
                "\r\nWorld Reload Completed\r\n{}\r\n\r\n\
                 Entities before reload: {}\r\n\
                 Entities despawned: {}\r\n\
                 Entities loaded from database: {}\r\n\
                 Final entity count: {}\r\n\r\n\
                 World has been reloaded from the database.\r\n\
                 Your avatar was preserved during the reload.\r\n\r\n\
                 {}\r\n",
                "=".repeat(80),
                entities_before,
                despawned,
                loaded,
                {
                    let world = context.entities().read().await;
                    world.query::<&EntityUuid>().iter().count()
                },
                "=".repeat(80)
            );
            CommandResult::Success(output)
        }
        Err(err) => {
            let output = format!(
                "\r\nWorld Reload Failed\r\n{}\r\n\r\n\
                 Error: {}\r\n\r\n\
                 {}\r\n",
                "=".repeat(80),
                err,
                "=".repeat(80)
            );
            CommandResult::Failure(output)
        }
    }
}

/// Helper function to get component type names for an entity
fn get_component_types(world: &GameWorld, entity: EcsEntity) -> Vec<String> {
    let mut components = Vec::new();

    if world.get::<&EntityUuid>(entity).is_ok() {
        components.push("EntityUuid");
    }
    if world.get::<&Name>(entity).is_ok() {
        components.push("Name");
    }
    if world.get::<&Description>(entity).is_ok() {
        components.push("Description");
    }
    if world.get::<&Location>(entity).is_ok() {
        components.push("Location");
    }
    if world.get::<&EntityType>(entity).is_ok() {
        components.push("EntityType");
    }
    if world.get::<&Avatar>(entity).is_ok() {
        components.push("Avatar");
    }
    if world.get::<&BodyAttributes>(entity).is_ok() {
        components.push("BodyAttributes");
    }
    if world.get::<&MindAttributes>(entity).is_ok() {
        components.push("MindAttributes");
    }
    if world.get::<&SoulAttributes>(entity).is_ok() {
        components.push("SoulAttributes");
    }
    if world.get::<&Skills>(entity).is_ok() {
        components.push("Skills");
    }
    if world.get::<&Combatant>(entity).is_ok() {
        components.push("Combatant");
    }
    if world.get::<&Equipment>(entity).is_ok() {
        components.push("Equipment");
    }
    if world.get::<&Area>(entity).is_ok() {
        components.push("Area");
    }
    if world.get::<&Room>(entity).is_ok() {
        components.push("Room");
    }
    if world.get::<&Exits>(entity).is_ok() {
        components.push("Exits");
    }
    if world.get::<&Container>(entity).is_ok() {
        components.push("Container");
    }
    if world.get::<&Containable>(entity).is_ok() {
        components.push("Containable");
    }
    if world.get::<&Enterable>(entity).is_ok() {
        components.push("Enterable");
    }
    if world.get::<&Equipable>(entity).is_ok() {
        components.push("Equipable");
    }
    if world.get::<&Weapon>(entity).is_ok() {
        components.push("Weapon");
    }
    if world.get::<&Material>(entity).is_ok() {
        components.push("Material");
    }
    if world.get::<&ArmorDefense>(entity).is_ok() {
        components.push("ArmorDefense");
    }
    if world.get::<&Commandable>(entity).is_ok() {
        components.push("Commandable");
    }
    if world.get::<&Interactable>(entity).is_ok() {
        components.push("Interactable");
    }
    if world.get::<&Persistent>(entity).is_ok() {
        components.push("Persistent");
    }
    if world.get::<&AIController>(entity).is_ok() {
        components.push("AIController");
    }
    if world.get::<&Personality>(entity).is_ok() {
        components.push("Personality");
    }

    components.into_iter().map(|s| s.to_string()).collect()
}

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn list_entities_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!(
        "List Entities Command from {}: {}",
        entity.id(),
        args.join(" ")
    );

    // Access world for read
    let world = context.entities().read().await;

    let mut output = String::from("\r\nAll Entities in World\r\n");
    output.push_str(&"=".repeat(80));
    output.push_str("\r\n\r\n");

    // Collect all entities with UUIDs
    let mut entities: Vec<(EcsEntity, uuid::Uuid, Option<String>)> = Vec::new();

    for (ent, entity_uuid) in world.query::<&EntityUuid>().iter() {
        let name = world.get::<&Name>(ent).ok().map(|n| n.display.clone());
        entities.push((ent, entity_uuid.0, name));
    }

    // Sort by name (or UUID if no name)
    entities.sort_by(|a, b| match (&a.2, &b.2) {
        (Some(name_a), Some(name_b)) => name_a.cmp(name_b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.1.cmp(&b.1),
    });

    output.push_str(&format!("Total Entities: {}\r\n\r\n", entities.len()));

    for (ent, uuid, name_opt) in entities {
        let name_display = name_opt.unwrap_or_else(|| "(unnamed)".to_string());
        let components = get_component_types(&world, ent);

        output.push_str(&format!("UUID: {}\r\n", uuid));
        output.push_str(&format!("  Name: {}\r\n", name_display));
        output.push_str(&format!("  ECS ID: {:?}\r\n", ent));
        output.push_str(&format!(
            "  Components ({}): {}\r\n",
            components.len(),
            components.join(", ")
        ));
        output.push_str("\r\n");
    }

    output.push_str(&"=".repeat(80));
    output.push_str("\r\n");

    drop(world); // Release the read lock

    CommandResult::Success(output)
}

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn query_entity_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Query Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: query <uuid>".to_string());
    }

    let uuid_str = &args[0];
    let target_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid UUID: {}", uuid_str)),
    };

    // Access world for read
    let world = context.entities().read().await;

    // Find the entity with the matching UUID
    let mut found_entity = None;
    for (ent, entity_uuid) in world.query::<&EntityUuid>().iter() {
        if entity_uuid.0 == target_uuid {
            found_entity = Some(ent);
            break;
        }
    }

    let target_entity = match found_entity {
        Some(e) => e,
        None => {
            drop(world);
            return CommandResult::Failure(format!("No entity found with UUID: {}", target_uuid));
        }
    };

    // Build output showing all components
    let mut output = format!("\r\nEntity Query: {}\r\n", target_uuid);
    output.push_str(&format!("ECS Entity ID: {:?}\r\n", target_entity));
    output.push_str("=".repeat(60).as_str());
    output.push_str("\r\n\r\nComponents:\r\n");

    // EntityUuid (we know it exists)
    output.push_str(&format!(
        "\r\n  EntityUuid:\r\n    UUID: {}\r\n",
        target_uuid
    ));

    // Name
    if let Ok(name) = world.get::<&Name>(target_entity) {
        output.push_str(&format!("\r\n  Name:\r\n    Display: {}\r\n", name.display));
        output.push_str(&format!("    Keywords: {:?}\r\n", name.keywords));
    }

    // Description
    if let Ok(desc) = world.get::<&Description>(target_entity) {
        output.push_str(&format!(
            "\r\n  Description:\r\n    Short: {}\r\n",
            desc.short
        ));
        output.push_str(&format!("    Long: {}\r\n", desc.long));
    }

    // Location
    if let Ok(loc) = world.get::<&Location>(target_entity) {
        output.push_str(&format!(
            "\r\n  Location:\r\n    Area UUID: {}\r\n",
            loc.area_id.uuid()
        ));
        output.push_str(&format!("    Room UUID: {}\r\n", loc.room_id.uuid()));
    }

    // EntityType
    if let Ok(entity_type) = world.get::<&EntityType>(target_entity) {
        output.push_str(&format!("\r\n  EntityType: {:?}\r\n", entity_type));
    }

    // Avatar
    if let Ok(avatar) = world.get::<&Avatar>(target_entity) {
        output.push_str(&format!(
            "\r\n  Avatar:\r\n    Account ID: {}\r\n",
            avatar.account_id
        ));
        output.push_str(&format!("    Available: {}\r\n", avatar.available));
    }

    // BodyAttributes
    if let Ok(body) = world.get::<&BodyAttributes>(target_entity) {
        output.push_str(&format!(
            "\r\n  BodyAttributes:\r\n    Offence: {}\r\n",
            body.score_offence
        ));
        output.push_str(&format!("    Finesse: {}\r\n", body.score_finesse));
        output.push_str(&format!("    Defence: {}\r\n", body.score_defence));
        output.push_str(&format!(
            "    Health: {}/{} (regen: {})\r\n",
            body.health_current, body.health_maximum, body.health_regen
        ));
        output.push_str(&format!(
            "    Energy: {}/{} (regen: {})\r\n",
            body.energy_current, body.energy_maximum, body.energy_regen
        ));
    }

    // MindAttributes
    if let Ok(mind) = world.get::<&MindAttributes>(target_entity) {
        output.push_str(&format!(
            "\r\n  MindAttributes:\r\n    Offence: {}\r\n",
            mind.score_offence
        ));
        output.push_str(&format!("    Finesse: {}\r\n", mind.score_finesse));
        output.push_str(&format!("    Defence: {}\r\n", mind.score_defence));
        output.push_str(&format!(
            "    Health: {}/{} (regen: {})\r\n",
            mind.health_current, mind.health_maximum, mind.health_regen
        ));
        output.push_str(&format!(
            "    Energy: {}/{} (regen: {})\r\n",
            mind.energy_current, mind.energy_maximum, mind.energy_regen
        ));
    }

    // SoulAttributes
    if let Ok(soul) = world.get::<&SoulAttributes>(target_entity) {
        output.push_str(&format!(
            "\r\n  SoulAttributes:\r\n    Offence: {}\r\n",
            soul.score_offence
        ));
        output.push_str(&format!("    Finesse: {}\r\n", soul.score_finesse));
        output.push_str(&format!("    Defence: {}\r\n", soul.score_defence));
        output.push_str(&format!(
            "    Health: {}/{} (regen: {})\r\n",
            soul.health_current, soul.health_maximum, soul.health_regen
        ));
        output.push_str(&format!(
            "    Energy: {}/{} (regen: {})\r\n",
            soul.energy_current, soul.energy_maximum, soul.energy_regen
        ));
    }

    // Skills
    if let Ok(skills) = world.get::<&Skills>(target_entity) {
        output.push_str("\r\n  Skills:\r\n");
        if skills.skills.is_empty() {
            output.push_str("    (none)\r\n");
        } else {
            for (skill_id, skill) in &skills.skills {
                let level = skills.level(*skill_id);
                output.push_str(&format!(
                    "    {}: Level {} (exp: {}, knowledge: {})\r\n",
                    skill_id.to_string(),
                    level,
                    skill.experience,
                    skill.knowledge
                ));
            }
        }
    }

    // Combatant
    if let Ok(combatant) = world.get::<&Combatant>(target_entity) {
        output.push_str(&format!(
            "\r\n  Combatant:\r\n    In Combat: {}\r\n",
            combatant.in_combat
        ));
        if let Some(target_id) = combatant.target_id {
            output.push_str(&format!("    Target UUID: {}\r\n", target_id.uuid()));
        }
        output.push_str(&format!("    Initiative: {}\r\n", combatant.initiative));
        output.push_str(&format!(
            "    Action Cooldown: {}\r\n",
            combatant.action_cooldown
        ));
        output.push_str(&format!(
            "    Time Since Action: {}\r\n",
            combatant.time_since_action
        ));
    }

    // Equipment
    if let Ok(equipment) = world.get::<&Equipment>(target_entity) {
        output.push_str("\r\n  Equipment:\r\n");
        if equipment.slots.is_empty() {
            output.push_str("    (no items equipped)\r\n");
        } else {
            for (slot, item_id) in &equipment.slots {
                output.push_str(&format!("    {:?}: UUID {}\r\n", slot, item_id.uuid()));
            }
        }
    }

    // Area
    if let Ok(area) = world.get::<&Area>(target_entity) {
        output.push_str(&format!(
            "\r\n  Area:\r\n    Kind: {:?}\r\n",
            area.area_kind
        ));
        output.push_str(&format!("    Flags: {:?}\r\n", area.area_flags));
    }

    // Room
    if let Ok(room) = world.get::<&Room>(target_entity) {
        output.push_str(&format!(
            "\r\n  Room:\r\n    Area UUID: {}\r\n",
            room.area_id.uuid()
        ));
        output.push_str(&format!("    Flags: {:?}\r\n", room.room_flags));
    }

    // Exits
    if let Ok(exits) = world.get::<&Exits>(target_entity) {
        output.push_str("\r\n  Exits:\r\n");
        if exits.exits.is_empty() {
            output.push_str("    (no exits)\r\n");
        } else {
            for exit in &exits.exits {
                output.push_str(&format!(
                    "    {} -> UUID {}",
                    exit.direction,
                    exit.dest_id.uuid()
                ));
                if exit.closeable {
                    output.push_str(&format!(
                        " (door: {}{})",
                        if exit.closed { "closed" } else { "open" },
                        if exit.lockable {
                            if exit.locked {
                                ", locked"
                            } else {
                                ", unlocked"
                            }
                        } else {
                            ""
                        }
                    ));
                }
                output.push_str("\r\n");
            }
        }
    }

    // Container
    if let Ok(container) = world.get::<&Container>(target_entity) {
        output.push_str("\r\n  Container:\r\n");
        if let Some(cap) = container.capacity {
            output.push_str(&format!("    Capacity: {}\r\n", cap));
        }
        if let Some(weight) = container.max_weight {
            output.push_str(&format!("    Max Weight: {}\r\n", weight));
        }
        output.push_str(&format!(
            "    Closeable: {} ({})\r\n",
            container.closeable,
            if container.closed { "closed" } else { "open" }
        ));
        output.push_str(&format!(
            "    Lockable: {} ({})\r\n",
            container.lockable,
            if container.locked {
                "locked"
            } else {
                "unlocked"
            }
        ));
    }

    // Containable
    if let Ok(containable) = world.get::<&Containable>(target_entity) {
        output.push_str(&format!(
            "\r\n  Containable:\r\n    Weight: {}\r\n",
            containable.weight
        ));
        output.push_str(&format!("    Size: {:?}\r\n", containable.size));
        output.push_str(&format!(
            "    Stackable: {} (stack size: {})\r\n",
            containable.stackable, containable.stack_size
        ));
    }

    // Enterable
    if let Ok(enterable) = world.get::<&Enterable>(target_entity) {
        output.push_str(&format!(
            "\r\n  Enterable:\r\n    Destination UUID: {}\r\n",
            enterable.dest_id.uuid()
        ));
        output.push_str(&format!(
            "    Closeable: {} ({})\r\n",
            enterable.closeable,
            if enterable.closed { "closed" } else { "open" }
        ));
        output.push_str(&format!(
            "    Lockable: {} ({})\r\n",
            enterable.lockable,
            if enterable.locked {
                "locked"
            } else {
                "unlocked"
            }
        ));
    }

    // Equipable
    if let Ok(equipable) = world.get::<&Equipable>(target_entity) {
        output.push_str("\r\n  Equipable:\r\n    Slots: ");
        output.push_str(&format!("{:?}\r\n", equipable.slots));
    }

    // Weapon
    if let Ok(weapon) = world.get::<&Weapon>(target_entity) {
        output.push_str(&format!(
            "\r\n  Weapon:\r\n    Damage: {}-{} (cap: {})\r\n",
            weapon.damage_min, weapon.damage_max, weapon.damage_cap
        ));
        output.push_str(&format!("    Type: {:?}\r\n", weapon.damage_type));
        output.push_str(&format!("    Attack Speed: {}\r\n", weapon.attack_speed));
        output.push_str(&format!("    Range: {}\r\n", weapon.range));
    }

    // Material
    if let Ok(material) = world.get::<&Material>(target_entity) {
        output.push_str(&format!("\r\n  Material: {:?}\r\n", material.material_kind));
    }

    // ArmorDefense
    if let Ok(armor) = world.get::<&ArmorDefense>(target_entity) {
        output.push_str("\r\n  ArmorDefense:\r\n");
        if armor.defenses.is_empty() {
            output.push_str("    (no defenses)\r\n");
        } else {
            for (damage_type, defense) in &armor.defenses {
                output.push_str(&format!("    {:?}: {}\r\n", damage_type, defense));
            }
        }
    }

    // Commandable
    if let Ok(commandable) = world.get::<&Commandable>(target_entity) {
        output.push_str(&format!(
            "\r\n  Commandable:\r\n    Max Queue Size: {}\r\n",
            commandable.max_queue_size
        ));
        output.push_str(&format!(
            "    Queue Length: {}\r\n",
            commandable.command_queue.len()
        ));
    }

    // Interactable
    if world.get::<&Interactable>(target_entity).is_ok() {
        output.push_str("\r\n  Interactable: (marker component)\r\n");
    }

    // Persistent
    if world.get::<&Persistent>(target_entity).is_ok() {
        output.push_str("\r\n  Persistent: (marker component)\r\n");
    }

    // AIController
    if let Ok(ai) = world.get::<&AIController>(target_entity) {
        output.push_str(&format!(
            "\r\n  AIController:\r\n    Behavior: {:?}\r\n",
            ai.behavior_type
        ));
        output.push_str(&format!("    State: {:?}\r\n", ai.state_type));
        if let Some(goal) = &ai.current_goal {
            output.push_str(&format!("    Goal: {}\r\n", goal));
        }
        if let Some(target) = ai.state_target_id {
            output.push_str(&format!("    Target UUID: {}\r\n", target.uuid()));
        }
        output.push_str(&format!("    Update Interval: {}\r\n", ai.update_interval));
    }

    // Personality
    if let Ok(personality) = world.get::<&Personality>(target_entity) {
        output.push_str(&format!(
            "\r\n  Personality:\r\n    Background: {}\r\n",
            personality.background
        ));
        output.push_str(&format!(
            "    Speaking Style: {}\r\n",
            personality.speaking_style
        ));
    }

    output.push_str("\r\n");
    output.push_str("=".repeat(60).as_str());
    output.push_str("\r\n");

    drop(world); // Release the read lock

    CommandResult::Success(output)
}

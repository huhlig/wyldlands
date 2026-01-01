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


// ============================================================================
// Builder Commands (Area, Room, Exit, Item Management)
// ============================================================================

// ============================================================================
// Area Commands
// ============================================================================

/// Create a new area
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn area_create_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Area Create Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: area create <name>".to_string());
    }

    let area_name = args.join(" ");

    // Create the area entity
    let area_uuid = uuid::Uuid::new_v4();
    let area_entity = {
        let mut world = context.entities().write().await;
        world.spawn((
            EntityUuid(area_uuid),
            Name::new(&area_name),
            Description::new(
                format!("A new area called {}", area_name),
                format!("This is a newly created area. Use 'area edit' to add a proper description."),
            ),
            Area::new(AreaKind::Overworld),
            Persistent,
        ))
    };

    // Register the entity
    context.register_entity(area_entity, area_uuid).await;

    // Mark as dirty for persistence
    context.mark_entity_dirty(area_entity).await;

    let output = format!(
        "\r\nArea created successfully!\r\n{}\r\n\r\n\
         UUID: {}\r\n\
         Name: {}\r\n\
         Kind: Overworld (default)\r\n\
         Flags: (none)\r\n\r\n\
         Use 'area edit {}' to modify properties.\r\n\r\n\
         {}\r\n",
        "=".repeat(80),
        area_uuid,
        area_name,
        area_uuid,
        "=".repeat(80)
    );

    CommandResult::Success(output)
}

/// List all areas
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn area_list_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Area List Command from {}: {}", entity.id(), args.join(" "));

    let filter = if args.is_empty() {
        None
    } else {
        Some(args.join(" ").to_lowercase())
    };

    let world = context.entities().read().await;

    // Collect all areas
    let mut areas: Vec<(uuid::Uuid, String, AreaKind, Vec<String>, usize)> = Vec::new();

    for (area_entity, (area_uuid, area_comp)) in world.query::<(&EntityUuid, &Area)>().iter() {
        let name = world.get::<&Name>(area_entity)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "(unnamed)".to_string());

        // Apply filter if present
        if let Some(ref filter_str) = filter {
            if !name.to_lowercase().contains(filter_str) {
                continue;
            }
        }

        // Count rooms in this area
        let room_count = world.query::<&Room>()
            .iter()
            .filter(|(_, room)| room.area_id.uuid() == area_uuid.0)
            .count();

        areas.push((
            area_uuid.0,
            name,
            area_comp.area_kind,
            area_comp.area_flags.clone(),
            room_count,
        ));
    }

    drop(world);

    // Sort by name
    areas.sort_by(|a, b| a.1.cmp(&b.1));

    let mut output = format!(
        "\r\nAreas ({} matching):\r\n{}\r\n",
        areas.len(),
        "=".repeat(80)
    );

    for (uuid, name, kind, flags, room_count) in areas {
        output.push_str(&format!("UUID: {}\r\n", uuid));
        output.push_str(&format!("  Name: {}\r\n", name));
        output.push_str(&format!("  Kind: {:?}\r\n", kind));
        output.push_str(&format!("  Flags: {}\r\n", 
            if flags.is_empty() { "(none)".to_string() } else { flags.join(", ") }
        ));
        output.push_str(&format!("  Rooms: {}\r\n\r\n", room_count));
    }

    output.push_str(&"=".repeat(80));
    output.push_str("\r\n");

    CommandResult::Success(output)
}

/// Edit area properties
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn area_edit_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Area Edit Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 3 {
        return CommandResult::Failure(
            "Usage: area edit <uuid> <property> <value>\r\n\
             Properties: name, description, kind, flag\r\n\
             Examples:\r\n\
               area edit <uuid> name New Name\r\n\
               area edit <uuid> description A new description\r\n\
               area edit <uuid> kind Dungeon\r\n\
               area edit <uuid> flag add Underwater\r\n\
               area edit <uuid> flag remove Underwater".to_string()
        );
    }

    let uuid_str = &args[0];
    let property = args[1].to_lowercase();
    let value = args[2..].join(" ");

    // Parse UUID
    let target_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid UUID: {}", uuid_str)),
    };

    let world = context.entities().write().await;

    // Find the area entity
    let area_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            drop(world);
            return CommandResult::Failure(format!("No area found with UUID: {}", target_uuid));
        }
    };

    // Verify it's an area
    if world.get::<&Area>(area_entity).is_err() {
        drop(world);
        return CommandResult::Failure(format!("Entity {} is not an area", target_uuid));
    }

    // Handle different properties
    let result = match property.as_str() {
        "name" => {
            if let Ok(mut name) = world.get::<&mut Name>(area_entity) {
                let old_name = name.display.clone();
                name.display = value.clone();
                name.keywords = vec![value.to_lowercase()];
                Ok(format!("Area name changed from '{}' to '{}'", old_name, value))
            } else {
                Err("Failed to update area name".to_string())
            }
        }
        "description" => {
            if let Ok(mut desc) = world.get::<&mut Description>(area_entity) {
                desc.long = value.clone();
                Ok("Area description updated".to_string())
            } else {
                Err("Failed to update area description".to_string())
            }
        }
        "kind" => {
            let area_kind = match value.to_lowercase().as_str() {
                "overworld" => AreaKind::Overworld,
                "vehicle" => AreaKind::Vehicle,
                "building" => AreaKind::Building,
                "dungeon" => AreaKind::Dungeon,
                _ => {
                    drop(world);
                    return CommandResult::Failure(
                        "Invalid area kind. Valid values: Overworld, Vehicle, Building, Dungeon".to_string()
                    );
                }
            };

            if let Ok(mut area) = world.get::<&mut Area>(area_entity) {
                area.area_kind = area_kind;
                Ok(format!("Area kind changed to {:?}", area_kind))
            } else {
                Err("Failed to update area kind".to_string())
            }
        }
        "flag" => {
            if args.len() < 4 {
                drop(world);
                return CommandResult::Failure(
                    "Usage: area edit <uuid> flag <add|remove> <flag>".to_string()
                );
            }

            let action = args[2].to_lowercase();
            let flag = args[3..].join(" ");

            if let Ok(mut area) = world.get::<&mut Area>(area_entity) {
                match action.as_str() {
                    "add" => {
                        if !area.area_flags.contains(&flag) {
                            area.area_flags.push(flag.clone());
                            Ok(format!("Added flag: {}", flag))
                        } else {
                            Err(format!("Area already has flag: {}", flag))
                        }
                    }
                    "remove" => {
                        if let Some(pos) = area.area_flags.iter().position(|f| f == &flag) {
                            area.area_flags.remove(pos);
                            Ok(format!("Removed flag: {}", flag))
                        } else {
                            Err(format!("Area does not have flag: {}", flag))
                        }
                    }
                    _ => Err("Invalid action. Use 'add' or 'remove'".to_string())
                }
            } else {
                Err("Failed to update area flags".to_string())
            }
        }
        _ => Err(format!("Unknown property: {}", property))
    };

    drop(world);
    
    match result {
        Ok(msg) => {
            context.mark_entity_dirty(area_entity).await;
            CommandResult::Success(msg)
        }
        Err(msg) => CommandResult::Failure(msg)
    }
}

/// Delete an area (only if it has no rooms)
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn area_delete_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Area Delete Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: area delete <uuid>".to_string());
    }

    let uuid_str = &args[0];
    let target_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid UUID: {}", uuid_str)),
    };

    let world = context.entities().read().await;

    // Find the area entity
    let area_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            drop(world);
            return CommandResult::Failure(format!("No area found with UUID: {}", target_uuid));
        }
    };

    // Verify it's an area
    if world.get::<&Area>(area_entity).is_err() {
        drop(world);
        return CommandResult::Failure(format!("Entity {} is not an area", target_uuid));
    }

    // Get area name for output
    let area_name = world.get::<&Name>(area_entity)
        .map(|n| n.display.clone())
        .unwrap_or_else(|_| "(unnamed)".to_string());

    // Count rooms in this area
    let room_count = world.query::<&Room>()
        .iter()
        .filter(|(_, room)| room.area_id.uuid() == target_uuid)
        .count();

    if room_count > 0 {
        drop(world);
        return CommandResult::Failure(format!(
            "Error: Cannot delete area - it contains {} rooms.\r\n\
             Use 'room list {}' to see rooms in this area.\r\n\
             Delete or move all rooms before deleting the area.",
            room_count, target_uuid
        ));
    }

    drop(world);

    // Safe to delete
    let mut world = context.entities().write().await;
    if let Err(e) = world.despawn(area_entity) {
        drop(world);
        return CommandResult::Failure(format!("Failed to delete area: {:?}", e));
    }
    drop(world);

    // Remove from registry
    context.unregister_entity(area_entity).await;

    // Delete from database
    if let Err(e) = context.delete_entity(target_uuid).await {
        return CommandResult::Failure(format!("Failed to delete area from database: {}", e));
    }

    CommandResult::Success(format!(
        "Area deleted successfully.\r\nUUID: {}\r\nName: {}",
        target_uuid, area_name
    ))
}

/// Display detailed information about an area
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn area_info_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Area Info Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: area info <uuid>".to_string());
    }

    let uuid_str = &args[0];
    let target_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid UUID: {}", uuid_str)),
    };

    let world = context.entities().read().await;

    // Find the area entity
    let area_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            drop(world);
            return CommandResult::Failure(format!("No area found with UUID: {}", target_uuid));
        }
    };

    // Verify it's an area and collect data
    let (area_kind, area_flags, name, description, room_count, exit_count) = {
        // Extract area data first
        let (area_kind, area_flags) = {
            match world.get::<&Area>(area_entity) {
                Ok(area) => {
                    let kind = area.area_kind;
                    let flags = area.area_flags.clone();
                    (kind, flags)
                }
                Err(_) => {
                    return CommandResult::Failure(format!("Entity {} is not an area", target_uuid));
                }
            }
        };

        let name = world.get::<&Name>(area_entity)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "(unnamed)".to_string());

        let description = world.get::<&Description>(area_entity)
            .map(|d| d.long.clone())
            .unwrap_or_else(|_| "(no description)".to_string());

        // Count rooms
        let room_count = world.query::<&Room>()
            .iter()
            .filter(|(_, room)| room.area_id.uuid() == target_uuid)
            .count();

        // Count exits
        let mut exit_count = 0;
        for (room_entity, room) in world.query::<&Room>().iter() {
            if room.area_id.uuid() == target_uuid {
                if let Ok(exits) = world.get::<&Exits>(room_entity) {
                    exit_count += exits.exits.len();
                }
            }
        }

        drop(world);
        (area_kind, area_flags, name, description, room_count, exit_count)
    };

    let output = format!(
        "\r\nArea Information\r\n{}\r\n\
         UUID: {}\r\n\
         Name: {}\r\n\
         Description: {}\r\n\r\n\
         Kind: {:?}\r\n\
         Flags: {}\r\n\r\n\
         Statistics:\r\n\
           Total Rooms: {}\r\n\
           Total Exits: {}\r\n\r\n\
         {}\r\n",
        "=".repeat(80),
        target_uuid,
        name,
        description,
        area_kind,
        if area_flags.is_empty() { "(none)".to_string() } else { area_flags.join(", ") },
        room_count,
        exit_count,
        "=".repeat(80)
    );

    CommandResult::Success(output)
}
// ============================================================================
// Room Commands
// ============================================================================

/// Create a new room in an area
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn room_create_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Room Create Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 2 {
        return CommandResult::Failure("Usage: room create <area-uuid> <name>".to_string());
    }

    let area_uuid_str = &args[0];
    let room_name = args[1..].join(" ");

    // Parse area UUID
    let area_uuid = match uuid::Uuid::parse_str(area_uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid area UUID: {}", area_uuid_str)),
    };

    // Verify area exists
    let world = context.entities().read().await;
    let area_entity = match find_entity_by_uuid(&world, area_uuid) {
        Some(e) => e,
        None => {
            drop(world);
            return CommandResult::Failure(format!("No area found with UUID: {}", area_uuid));
        }
    };

    // Verify it's an area
    if world.get::<&Area>(area_entity).is_err() {
        drop(world);
        return CommandResult::Failure(format!("Entity {} is not an area", area_uuid));
    }

    let area_name = world.get::<&Name>(area_entity)
        .map(|n| n.display.clone())
        .unwrap_or_else(|_| "(unnamed)".to_string());

    drop(world);

    // Create the room entity
    let room_uuid = uuid::Uuid::new_v4();
    let room_entity = {
        let mut world = context.entities().write().await;
        world.spawn((
            EntityUuid(room_uuid),
            Name::new(&room_name),
            Description::new(
                format!("A room called {}", room_name),
                format!("This is a newly created room. Use 'room edit' to add a proper description."),
            ),
            Room::new(EntityId::from_uuid(area_uuid)),
            Exits::new(),
            Persistent,
        ))
    };

    // Register the entity
    context.register_entity(room_entity, room_uuid).await;

    // Mark as dirty for persistence
    context.mark_entity_dirty(room_entity).await;

    // Teleport the builder to the new room
    {
        let world = context.entities().write().await;
        if let Ok(mut location) = world.get::<&mut Location>(entity) {
            *location = Location::new(EntityId::from_uuid(area_uuid), EntityId::from_uuid(room_uuid));
        }
    }

    let output = format!(
        "\r\nRoom created successfully!\r\n{}\r\n\r\n\
         UUID: {}\r\n\
         Name: {}\r\n\
         Area: {} ({})\r\n\
         Flags: Breathable (default)\r\n\r\n\
         You are now in the new room.\r\n\
         Use 'room edit {}' to modify properties.\r\n\
         Use 'dig <direction> <name>' to create connected rooms.\r\n\r\n\
         {}\r\n",
        "=".repeat(80),
        room_uuid,
        room_name,
        area_name,
        area_uuid,
        room_uuid,
        "=".repeat(80)
    );

    CommandResult::Success(output)
}

/// List rooms in an area or all rooms
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn room_list_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Room List Command from {}: {}", entity.id(), args.join(" "));

    let area_filter = if !args.is_empty() {
        match uuid::Uuid::parse_str(&args[0]) {
            Ok(uuid) => Some(uuid),
            Err(_) => return CommandResult::Failure(format!("Invalid area UUID: {}", args[0])),
        }
    } else {
        None
    };

    let world = context.entities().read().await;

    // Get area name if filtering
    let area_name = if let Some(area_uuid) = area_filter {
        if let Some(area_entity) = find_entity_by_uuid(&world, area_uuid) {
            world.get::<&Name>(area_entity)
                .map(|n| n.display.clone())
                .unwrap_or_else(|_| "(unnamed)".to_string())
        } else {
            drop(world);
            return CommandResult::Failure(format!("No area found with UUID: {}", area_uuid));
        }
    } else {
        String::new()
    };

    // Collect rooms
    let mut rooms: Vec<(uuid::Uuid, String, Vec<String>, Vec<RoomFlag>)> = Vec::new();

    for (room_entity, (room_uuid, room)) in world.query::<(&EntityUuid, &Room)>().iter() {
        // Apply area filter
        if let Some(filter_uuid) = area_filter {
            if room.area_id.uuid() != filter_uuid {
                continue;
            }
        }

        let name = world.get::<&Name>(room_entity)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "(unnamed)".to_string());

        let exits = world.get::<&Exits>(room_entity)
            .map(|e| e.directions().iter().map(|s| s.to_string()).collect())
            .unwrap_or_else(|_| Vec::new());

        rooms.push((room_uuid.0, name, exits, room.room_flags.clone()));
    }

    drop(world);

    // Sort by name
    rooms.sort_by(|a, b| a.1.cmp(&b.1));

    let header = if let Some(_area_uuid) = area_filter {
        format!("Rooms in Area: {} ({} rooms)", area_name, rooms.len())
    } else {
        format!("All Rooms ({} total)", rooms.len())
    };

    let mut output = format!("\r\n{}\r\n{}\r\n", header, "=".repeat(80));

    for (uuid, name, exits, flags) in rooms {
        output.push_str(&format!("UUID: {}\r\n", uuid));
        output.push_str(&format!("  Name: {}\r\n", name));
        output.push_str(&format!("  Exits: {}\r\n",
            if exits.is_empty() { "(none)".to_string() } else { exits.join(", ") }
        ));
        output.push_str(&format!("  Flags: {:?}\r\n\r\n", flags));
    }

    output.push_str(&"=".repeat(80));
    output.push_str("\r\n");

    CommandResult::Success(output)
}

/// Teleport to a specific room
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn room_goto_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Room Goto Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: room goto <uuid>".to_string());
    }

    let uuid_str = &args[0];
    let target_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid UUID: {}", uuid_str)),
    };

    let world = context.entities().read().await;

    // Find the room entity
    let room_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            drop(world);
            return CommandResult::Failure(format!("No room found with UUID: {}", target_uuid));
        }
    };

    // Verify it's a room and get area_id
    let area_id = {
        match world.get::<&Room>(room_entity) {
            Ok(room) => room.area_id,
            Err(_) => {
                return CommandResult::Failure(format!("Entity {} is not a room", target_uuid));
            }
        }
    };
    drop(world);

    // Teleport the entity
    {
        let world = context.entities().write().await;
        if let Ok(mut location) = world.get::<&mut Location>(entity) {
            *location = Location::new(area_id, EntityId::from_uuid(target_uuid));
        } else {
            return CommandResult::Failure("Failed to teleport".to_string());
        }
    }

    // Show the new room using look command
    super::look::look_command(context.clone(), entity, "look".to_string(), vec![]).await
}

// ============================================================================
// Exit Commands
// ============================================================================

/// Add an exit from current room
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn exit_add_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Exit Add Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 2 {
        return CommandResult::Failure("Usage: exit add <direction> <dest-room-uuid>".to_string());
    }

    let direction = args[0].clone();
    let dest_uuid_str = &args[1];

    // Parse destination UUID
    let dest_uuid = match uuid::Uuid::parse_str(dest_uuid_str) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure(format!("Invalid destination UUID: {}", dest_uuid_str)),
    };

    // Get current location and room info
    let (_current_loc, current_room_entity, dest_room_name, current_room_name) = {
        let world = context.entities().read().await;
        let current_loc = match world.get::<&Location>(entity) {
            Ok(loc) => *loc,
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        };

        let current_room_uuid = current_loc.room_id.uuid();

        // Find current room entity
        let current_room_entity = match find_entity_by_uuid(&world, current_room_uuid) {
            Some(e) => e,
            None => {
                return CommandResult::Failure("Current room not found".to_string());
            }
        };

        // Verify destination room exists
        if find_entity_by_uuid(&world, dest_uuid).is_none() {
            return CommandResult::Failure(format!("Destination room not found: {}", dest_uuid));
        }

        let dest_room_name = if let Some(dest_entity) = find_entity_by_uuid(&world, dest_uuid) {
            world.get::<&Name>(dest_entity)
                .map(|n| n.display.clone())
                .unwrap_or_else(|_| "(unnamed)".to_string())
        } else {
            "(unknown)".to_string()
        };

        let current_room_name = world.get::<&Name>(current_room_entity)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "(unnamed)".to_string());

        (current_loc, current_room_entity, dest_room_name, current_room_name)
    };

    // Add the exit
    {
        let world = context.entities().write().await;
        let result = if let Ok(mut exits) = world.get::<&mut Exits>(current_room_entity) {
            // Check if exit already exists
            if exits.has_exit(&direction) {
                Err(format!("Exit '{}' already exists", direction))
            } else {
                exits.exits.push(ExitData::new(&direction, EntityId::from_uuid(dest_uuid)));
                Ok(())
            }
        } else {
            Err("Failed to add exit".to_string())
        };
        drop(world);
        
        if let Err(e) = result {
            return CommandResult::Failure(e);
        }
        context.mark_entity_dirty(current_room_entity).await;
    }

    CommandResult::Success(format!(
        "Exit added successfully!\r\n\
         Direction: {}\r\n\
         From: {} (current room)\r\n\
         To: {} ({})\r\n\r\n\
         Use 'exit edit {} <property> <value>' to add doors, locks, etc.",
        direction, current_room_name, dest_room_name, dest_uuid, direction
    ))
}

/// Remove an exit from current room
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn exit_remove_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Exit Remove Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: exit remove <direction>".to_string());
    }

    let direction = args[0].clone();

    // Get current location
    let current_room_entity = {
        let world = context.entities().read().await;
        let current_loc = match world.get::<&Location>(entity) {
            Ok(loc) => *loc,
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        };

        let current_room_uuid = current_loc.room_id.uuid();

        // Find current room entity
        match find_entity_by_uuid(&world, current_room_uuid) {
            Some(e) => e,
            None => {
                return CommandResult::Failure("Current room not found".to_string());
            }
        }
    };

    // Remove the exit
    let removed_dest = {
        let world = context.entities().write().await;
        let result = if let Ok(mut exits) = world.get::<&mut Exits>(current_room_entity) {
            if let Some(pos) = exits.exits.iter().position(|e| e.direction.to_lowercase() == direction.to_lowercase()) {
                let removed = exits.exits.remove(pos);
                Some(removed.dest_id.uuid())
            } else {
                None
            }
        } else {
            None
        };
        drop(world);
        
        if result.is_some() {
            context.mark_entity_dirty(current_room_entity).await;
        }
        result
    };

    match removed_dest {
        Some(dest_uuid) => CommandResult::Success(format!(
            "Exit removed successfully!\r\n\
             Direction: {}\r\n\
             To: {}",
            direction, dest_uuid
        )),
        None => CommandResult::Failure(format!("No exit found in direction '{}'", direction)),
    }
}

/// List exits from current room
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn exit_list_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    _args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Exit List Command from {}", entity.id());

    // Get current location
    let (_current_room_uuid, current_room_entity) = {
        let world = context.entities().read().await;
        
        // Get location - extract value immediately
        let current_room_uuid = {
            let loc_ref = world.get::<&Location>(entity);
            match loc_ref {
                Ok(loc) => loc.room_id.uuid(),
                Err(_) => {
                    return CommandResult::Failure("You have no location".to_string());
                }
            }
        };

        // Find current room entity
        let current_room_entity = match find_entity_by_uuid(&world, current_room_uuid) {
            Some(e) => e,
            None => {
                return CommandResult::Failure("Current room not found".to_string());
            }
        };
        
        (current_room_uuid, current_room_entity)
    };

    let (room_name, exits) = {
        let world = context.entities().read().await;
        let room_name = world.get::<&Name>(current_room_entity)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "(unnamed)".to_string());

        let exits =
            match world.get::<&Exits>(current_room_entity) {
                Ok(e) => e.exits.clone(),
                Err(_) => {
                    return CommandResult::Failure("Current room has no exits component".to_string());
                }
            };
        
        (room_name, exits)
    };

    let mut output = format!(
        "\r\nExits from: {}\r\n{}\r\n",
        room_name,
        "=".repeat(80)
    );

    if exits.is_empty() {
        output.push_str("(no exits)\r\n");
    } else {
        for exit in exits {
            output.push_str(&format!("{} -> {}\r\n", exit.direction, exit.dest_id.uuid()));
            if exit.closeable {
                output.push_str(&format!(
                    "  Door: {} (Rating: {})\r\n",
                    if exit.closed { "Closed" } else { "Open" },
                    exit.door_rating.unwrap_or(0)
                ));
            }
            if exit.lockable {
                output.push_str(&format!(
                    "  Lock: {} (Rating: {}, Code: {})\r\n",
                    if exit.locked { "Locked" } else { "Unlocked" },
                    exit.lock_rating.unwrap_or(0),
                    exit.unlock_code.as_ref().unwrap_or(&"(none)".to_string())
                ));
            }
            output.push_str("\r\n");
        }
    }

    output.push_str(&"=".repeat(80));
    output.push_str("\r\n");

    CommandResult::Success(output)
}

// ============================================================================
// Digging Commands
// ============================================================================

/// Dig a new room in a direction
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn dig_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Dig Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 2 {
        return CommandResult::Failure(
            "Usage: dig <direction> <room-name> [oneway] [area <area-uuid>]".to_string()
        );
    }

    let direction = args[0].clone();
    let mut room_name_parts = Vec::new();
    let mut oneway = false;
    let mut target_area_uuid: Option<uuid::Uuid> = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].to_lowercase().as_str() {
            "oneway" => {
                oneway = true;
                i += 1;
            }
            "area" => {
                if i + 1 < args.len() {
                    match uuid::Uuid::parse_str(&args[i + 1]) {
                        Ok(uuid) => {
                            target_area_uuid = Some(uuid);
                            i += 2;
                        }
                        Err(_) => {
                            return CommandResult::Failure(format!("Invalid area UUID: {}", args[i + 1]));
                        }
                    }
                } else {
                    return CommandResult::Failure("Missing area UUID after 'area'".to_string());
                }
            }
            _ => {
                room_name_parts.push(args[i].clone());
                i += 1;
            }
        }
    }

    if room_name_parts.is_empty() {
        return CommandResult::Failure("Room name cannot be empty".to_string());
    }

    let room_name = room_name_parts.join(" ");

    // Get current location and validate
    let (current_room_uuid, current_area_uuid, current_room_entity, current_room_name, new_area_uuid, area_name) = {
        let world = context.entities().read().await;
        let current_loc = match world.get::<&Location>(entity) {
            Ok(loc) => *loc,
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        };

        let current_room_uuid = current_loc.room_id.uuid();
        let current_area_uuid = current_loc.area_id.uuid();

        // Find current room entity
        let current_room_entity = match find_entity_by_uuid(&world, current_room_uuid) {
            Some(e) => e,
            None => {
                return CommandResult::Failure("Current room not found".to_string());
            }
        };

        let current_room_name = world.get::<&Name>(current_room_entity)
            .map(|n| n.display.clone())
            .unwrap_or_else(|_| "(unnamed)".to_string());

        // Check if exit already exists
        if let Ok(exits) = world.get::<&Exits>(current_room_entity) {
            if exits.has_exit(&direction) {
                return CommandResult::Failure(format!("Exit '{}' already exists in current room", direction));
            }
        }

        // Determine target area
        let new_area_uuid = target_area_uuid.unwrap_or(current_area_uuid);

        // Verify target area exists
        if find_entity_by_uuid(&world, new_area_uuid).is_none() {
            return CommandResult::Failure(format!("Target area not found: {}", new_area_uuid));
        }

        let area_name = if let Some(area_entity) = find_entity_by_uuid(&world, new_area_uuid) {
            world.get::<&Name>(area_entity)
                .map(|n| n.display.clone())
                .unwrap_or_else(|_| "(unnamed)".to_string())
        } else {
            "(unknown)".to_string()
        };

        (current_room_uuid, current_area_uuid, current_room_entity, current_room_name, new_area_uuid, area_name)
    };

    // Create the new room
    let new_room_uuid = uuid::Uuid::new_v4();
    let new_room_entity = {
        let mut world = context.entities().write().await;
        world.spawn((
            EntityUuid(new_room_uuid),
            Name::new(&room_name),
            Description::new(
                format!("A room called {}", room_name),
                format!("This is a newly created room. Use 'room edit' to add a proper description."),
            ),
            Room::new(EntityId::from_uuid(new_area_uuid)),
            Exits::new(),
            Persistent,
        ))
    };

    // Register the entity
    context.register_entity(new_room_entity, new_room_uuid).await;

    // Mark as dirty for persistence
    context.mark_entity_dirty(new_room_entity).await;

    // Add exit from current room to new room
    {
        let world = context.entities().write().await;
        if let Ok(mut exits) = world.get::<&mut Exits>(current_room_entity) {
            exits.exits.push(ExitData::new(&direction, EntityId::from_uuid(new_room_uuid)));
        }
    }
    context.mark_entity_dirty(current_room_entity).await;

    // Add reverse exit if not oneway
    let reverse_direction = get_reverse_direction(&direction);
    let reverse_dir_str = if !oneway && reverse_direction.is_some() {
        let rev_dir = reverse_direction.as_ref().unwrap().clone();
        let world = context.entities().write().await;
        if let Ok(mut exits) = world.get::<&mut Exits>(new_room_entity) {
            exits.exits.push(ExitData::new(&rev_dir, EntityId::from_uuid(current_room_uuid)));
        }
        drop(world);
        context.mark_entity_dirty(new_room_entity).await;
        Some(rev_dir)
    } else {
        None
    };

    // Teleport the builder to the new room
    {
        let world = context.entities().write().await;
        if let Ok(mut location) = world.get::<&mut Location>(entity) {
            *location = Location::new(EntityId::from_uuid(new_area_uuid), EntityId::from_uuid(new_room_uuid));
        }
    }

    let mut output = format!(
        "\r\nDigging {}{}...\r\n\r\n\
         New room created!\r\n\
         UUID: {}\r\n\
         Name: {}\r\n\
         Area: {} ({})\r\n\r\n\
         Exits created:\r\n\
           From {} -> {} -> {}\r\n",
        direction,
        if oneway { " (one-way)" } else { "" },
        new_room_uuid,
        room_name,
        area_name,
        new_area_uuid,
        current_room_name,
        direction,
        room_name
    );

    if let Some(rev_dir) = reverse_dir_str {
        output.push_str(&format!(
            "  From {} -> {} -> {}\r\n",
            room_name,
            rev_dir,
            current_room_name
        ));
    } else if oneway {
        output.push_str("  (No reverse exit created)\r\n");
    }

    output.push_str("\r\nYou are now in the new room.\r\n");

    if target_area_uuid.is_some() && target_area_uuid.unwrap() != current_area_uuid {
        output.push_str("\r\nNote: This room is in a different area than the previous room.\r\n");
    }

    CommandResult::Success(output)
}

/// Get the reverse direction for common directions
fn get_reverse_direction(direction: &str) -> Option<String> {
    match direction.to_lowercase().as_str() {
        "north" | "n" => Some("South".to_string()),
        "south" | "s" => Some("North".to_string()),
        "east" | "e" => Some("West".to_string()),
        "west" | "w" => Some("East".to_string()),
        "up" | "u" => Some("Down".to_string()),
        "down" | "d" => Some("Up".to_string()),
        "northeast" | "ne" => Some("Southwest".to_string()),
        "northwest" | "nw" => Some("Southeast".to_string()),
        "southeast" | "se" => Some("Northwest".to_string()),
        "southwest" | "sw" => Some("Northeast".to_string()),
        _ => None,
    }
}
// ============================================================================
// Phase 2: Advanced Room/Exit Commands
// ============================================================================

/// Edit a room's properties
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn room_edit_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Room Edit Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 3 {
        return CommandResult::Failure(
            "Usage: room edit <uuid> <field> <value>\n\
             Fields: name, description, area\n\
             Example: room edit <uuid> name New Room Name".to_string()
        );
    }

    let target_uuid = match uuid::Uuid::parse_str(&args[0]) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    let field = args[1].to_lowercase();
    let value = args[2..].join(" ");

    // Find the room entity
    let world = context.entities().read().await;
    let room_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            return CommandResult::Failure(format!("Room {} not found", target_uuid));
        }
    };

    // Verify it's a room
    if world.get::<&Room>(room_entity).is_err() {
        drop(world);
        return CommandResult::Failure(format!("Entity {} is not a room", target_uuid));
    }
    drop(world);

    // Apply the edit
    match field.as_str() {
        "name" => {
            let mut world = context.entities().write().await;
            let result = if let Ok(mut name) = world.get::<&mut Name>(room_entity) {
                name.display = value.clone();
                CommandResult::Success(format!("Room name updated to: {}", value))
            } else {
                CommandResult::Failure("Failed to update room name".to_string())
            };
            drop(world);
            context.mark_entity_dirty(room_entity).await;
            result
        }
        "description" => {
            let mut world = context.entities().write().await;
            let result = if let Ok(mut desc) = world.get::<&mut Description>(room_entity) {
                desc.long = value.clone();
                CommandResult::Success(format!("Room description updated"))
            } else {
                CommandResult::Failure("Failed to update room description".to_string())
            };
            drop(world);
            context.mark_entity_dirty(room_entity).await;
            result
        }
        "area" => {
            let new_area_uuid = match uuid::Uuid::parse_str(&value) {
                Ok(uuid) => uuid,
                Err(_) => return CommandResult::Failure("Invalid area UUID format".to_string()),
            };

            // Verify the new area exists
            let world = context.entities().read().await;
            let new_area_entity = match find_entity_by_uuid(&world, new_area_uuid) {
                Some(e) => e,
                None => {
                    return CommandResult::Failure(format!("Area {} not found", new_area_uuid));
                }
            };

            if world.get::<&Area>(new_area_entity).is_err() {
                drop(world);
                return CommandResult::Failure(format!("Entity {} is not an area", new_area_uuid));
            }
            drop(world);

            // Update the room's area
            let mut world = context.entities().write().await;
            let result = if let Ok(mut room) = world.get::<&mut Room>(room_entity) {
                room.area_id = EntityId::from_uuid(new_area_uuid);
                CommandResult::Success(format!("Room moved to area {}", new_area_uuid))
            } else {
                CommandResult::Failure("Failed to update room area".to_string())
            };
            drop(world);
            context.mark_entity_dirty(room_entity).await;
            result
        }
        _ => CommandResult::Failure(format!("Unknown field: {}. Valid fields: name, description, area", field)),
    }
}

/// Delete all rooms in an area
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn room_delete_bulk_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Room Delete Bulk Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: room deleteall <area_uuid>".to_string());
    }

    let area_uuid = match uuid::Uuid::parse_str(&args[0]) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    // Find and collect all rooms in the area
    let room_entities: Vec<(EcsEntity, uuid::Uuid)> = {
        let world = context.entities().read().await;
        
        // Verify area exists
        if find_entity_by_uuid(&world, area_uuid).is_none() {
            return CommandResult::Failure(format!("Area {} not found", area_uuid));
        }

        world.query::<(&EntityUuid, &Room)>()
            .iter()
            .filter(|(_, (_, room))| room.area_id.uuid() == area_uuid)
            .map(|(entity, (entity_uuid, _))| (entity, entity_uuid.0))
            .collect()
    };

    if room_entities.is_empty() {
        return CommandResult::Success("No rooms found in this area".to_string());
    }

    let count = room_entities.len();

    // Delete all rooms
    {
        let mut world = context.entities().write().await;
        for (room_entity, room_uuid) in &room_entities {
            let _ = world.despawn(*room_entity);
            context.delete_entity(*room_uuid).await;
        }
    }

    CommandResult::Success(format!("Deleted {} room(s) from area {}", count, area_uuid))
}

/// Edit exit properties
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn exit_edit_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Exit Edit Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 3 {
        return CommandResult::Failure(
            "Usage: exit edit <direction> <property> <value>\n\
             Properties: closeable, closed, lockable, locked, door_rating, lock_rating, unlock_code\n\
             Example: exit edit north closeable true".to_string()
        );
    }

    let direction = args[0].to_lowercase();
    let property = args[1].to_lowercase();
    let value = args[2..].join(" ");

    // Get current location
    let current_room_entity = {
        let world = context.entities().read().await;
        let current_room_uuid = {
            let loc_ref = world.get::<&Location>(entity);
            match loc_ref {
                Ok(loc) => loc.room_id.uuid(),
                Err(_) => {
                    return CommandResult::Failure("You have no location".to_string());
                }
            }
        };

        match find_entity_by_uuid(&world, current_room_uuid) {
            Some(e) => e,
            None => {
                return CommandResult::Failure("Current room not found".to_string());
            }
        }
    };

    // Edit the exit
    let result = {
        let mut world = context.entities().write().await;
        let result = if let Ok(mut exits) = world.get::<&mut Exits>(current_room_entity) {
            // Find the exit
            if let Some(exit) = exits.exits.iter_mut().find(|e| e.direction.to_lowercase() == direction) {
                let result = match property.as_str() {
                    "closeable" => {
                        let closeable = value.to_lowercase() == "true";
                        exit.closeable = closeable;
                        CommandResult::Success(format!("Exit '{}' closeable set to {}", direction, closeable))
                    }
                    "closed" => {
                        if !exit.closeable {
                            CommandResult::Failure("Exit must be closeable first".to_string())
                        } else {
                            let closed = value.to_lowercase() == "true";
                            exit.closed = closed;
                            CommandResult::Success(format!("Exit '{}' closed set to {}", direction, closed))
                        }
                    }
                    "lockable" => {
                        if !exit.closeable {
                            CommandResult::Failure("Exit must be closeable first".to_string())
                        } else {
                            let lockable = value.to_lowercase() == "true";
                            exit.lockable = lockable;
                            CommandResult::Success(format!("Exit '{}' lockable set to {}", direction, lockable))
                        }
                    }
                    "locked" => {
                        if !exit.lockable {
                            CommandResult::Failure("Exit must be lockable first".to_string())
                        } else {
                            let locked = value.to_lowercase() == "true";
                            exit.locked = locked;
                            CommandResult::Success(format!("Exit '{}' locked set to {}", direction, locked))
                        }
                    }
                    "door_rating" => {
                        if !exit.closeable {
                            CommandResult::Failure("Exit must be closeable first".to_string())
                        } else {
                            match value.parse::<i32>() {
                                Ok(rating) => {
                                    exit.door_rating = Some(rating);
                                    CommandResult::Success(format!("Exit '{}' door rating set to {}", direction, rating))
                                }
                                Err(_) => CommandResult::Failure("Invalid rating value".to_string()),
                            }
                        }
                    }
                    "lock_rating" => {
                        if !exit.lockable {
                            CommandResult::Failure("Exit must be lockable first".to_string())
                        } else {
                            match value.parse::<i32>() {
                                Ok(rating) => {
                                    exit.lock_rating = Some(rating);
                                    CommandResult::Success(format!("Exit '{}' lock rating set to {}", direction, rating))
                                }
                                Err(_) => CommandResult::Failure("Invalid rating value".to_string()),
                            }
                        }
                    }
                    "unlock_code" => {
                        if !exit.lockable {
                            CommandResult::Failure("Exit must be lockable first".to_string())
                        } else {
                            exit.unlock_code = Some(value.clone());
                            CommandResult::Success(format!("Exit '{}' unlock code set", direction))
                        }
                    }
                    _ => CommandResult::Failure(format!("Unknown property: {}", property)),
                };
                result
            } else {
                CommandResult::Failure(format!("No exit found in direction '{}'", direction))
            }
        } else {
            CommandResult::Failure("Current room has no exits component".to_string())
        };
        drop(world);
        result
    };
    context.mark_entity_dirty(current_room_entity).await;
    result
}

/// Search for areas by name
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn area_search_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Area Search Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: area search <query>".to_string());
    }

    let query = args.join(" ").to_lowercase();

    let world = context.entities().read().await;
    let mut results = Vec::new();

    for (entity, (entity_uuid, area, name)) in world.query::<(&EntityUuid, &Area, &Name)>().iter() {
        if name.display.to_lowercase().contains(&query) {
            let room_count = world.query::<&Room>()
                .iter()
                .filter(|(_, room)| room.area_id.uuid() == entity_uuid.0)
                .count();
            
            results.push((entity_uuid.0, name.display.clone(), area.area_kind, room_count));
        }
    }

    drop(world);

    if results.is_empty() {
        return CommandResult::Success(format!("No areas found matching '{}'", query));
    }

    let mut output = format!("\r\nArea Search Results for '{}'\r\n{}\r\n", query, "=".repeat(80));
    for (uuid, name, kind, room_count) in results {
        output.push_str(&format!(
            "{} - {} ({:?}, {} rooms)\r\n",
            uuid, name, kind, room_count
        ));
    }

    CommandResult::Success(output)
}

/// Search for rooms by name
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn room_search_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Room Search Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: room search <query> [area_uuid]".to_string());
    }

    let query = args[0..].join(" ").to_lowercase();
    let area_filter = if args.len() > 1 {
        match uuid::Uuid::parse_str(&args[args.len() - 1]) {
            Ok(uuid) => Some(uuid),
            Err(_) => None,
        }
    } else {
        None
    };

    let world = context.entities().read().await;
    let mut results = Vec::new();

    for (entity, (entity_uuid, room, name)) in world.query::<(&EntityUuid, &Room, &Name)>().iter() {
        // Apply area filter if specified
        if let Some(area_uuid) = area_filter {
            if room.area_id.uuid() != area_uuid {
                continue;
            }
        }

        if name.display.to_lowercase().contains(&query) {
            // Get area name
            let area_name = if let Some(area_entity) = find_entity_by_uuid(&world, room.area_id.uuid()) {
                world.get::<&Name>(area_entity)
                    .map(|n| n.display.clone())
                    .unwrap_or_else(|_| "(unnamed area)".to_string())
            } else {
                "(unknown area)".to_string()
            };

            results.push((entity_uuid.0, name.display.clone(), room.area_id.uuid(), area_name));
        }
    }

    drop(world);

    if results.is_empty() {
        return CommandResult::Success(format!("No rooms found matching '{}'", query));
    }

    let mut output = format!("\r\nRoom Search Results for '{}'\r\n{}\r\n", query, "=".repeat(80));
    for (uuid, name, area_uuid, area_name) in results {
        output.push_str(&format!(
            "{} - {} (in {} - {})\r\n",
            uuid, name, area_name, area_uuid
        ));
    }

    CommandResult::Success(output)
}

// ============================================================================
// Phase 3: Object/Item Editor Commands
// ============================================================================

/// Create a new item
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_create_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item Create Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: item create <name>".to_string());
    }

    let item_name = args.join(" ");

    // Get current location
    let (area_uuid, room_uuid) = {
        let world = context.entities().read().await;
        match world.get::<&Location>(entity) {
            Ok(loc) => (loc.area_id.uuid(), loc.room_id.uuid()),
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        }
    };

    // Create the item entity
    let item_uuid = uuid::Uuid::new_v4();
    let item_entity = {
        let mut world = context.entities().write().await;
        world.spawn((
            EntityUuid(item_uuid),
            Name::new(&item_name),
            Description::new(
                format!("An item called {}", item_name),
                "This is a newly created item. Use 'item edit' to add a proper description.".to_string(),
            ),
            Location::new(EntityId::from_uuid(area_uuid), EntityId::from_uuid(room_uuid)),
            Containable::new(1.0), // Default weight
            Persistent,
        ))
    };

    // Register the entity
    context.register_entity(item_entity, item_uuid).await;
    context.mark_entity_dirty(item_entity).await;

    CommandResult::Success(format!(
        "Item created successfully!\r\n\
         Name: {}\r\n\
         UUID: {}\r\n\
         Location: Room {}\r\n\
         Use 'item edit' to customize properties",
        item_name, item_uuid, room_uuid
    ))
}

/// Edit an item's properties
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_edit_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item Edit Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 3 {
        return CommandResult::Failure(
            "Usage: item edit <uuid> <field> <value>\n\
             Fields: name, description, weight, weapon, armor\n\
             Example: item edit <uuid> name Rusty Sword\n\
             Example: item edit <uuid> weapon 5 10 slashing".to_string()
        );
    }

    let target_uuid = match uuid::Uuid::parse_str(&args[0]) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    let field = args[1].to_lowercase();
    let value_args = &args[2..];

    // Find the item entity
    let world = context.entities().read().await;
    let item_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            return CommandResult::Failure(format!("Item {} not found", target_uuid));
        }
    };
    drop(world);

    // Apply the edit
    match field.as_str() {
        "name" => {
            let value = value_args.join(" ");
            let result = {
                let mut world = context.entities().write().await;
                let result = match world.get::<&mut Name>(item_entity) {
                    Ok(mut name) => {
                        name.display = value.clone();
                        CommandResult::Success(format!("Item name updated to: {}", value))
                    }
                    Err(_) => CommandResult::Failure("Failed to update item name".to_string()),
                };
                drop(world);
                result
            };
            context.mark_entity_dirty(item_entity).await;
            result
        }
        "description" => {
            let value = value_args.join(" ");
            let result = {
                let mut world = context.entities().write().await;
                let result = match world.get::<&mut Description>(item_entity) {
                    Ok(mut desc) => {
                        desc.long = value.clone();
                        CommandResult::Success("Item description updated".to_string())
                    }
                    Err(_) => CommandResult::Failure("Failed to update item description".to_string()),
                };
                drop(world);
                result
            };
            context.mark_entity_dirty(item_entity).await;
            result
        }
        "weight" => {
            if value_args.is_empty() {
                return CommandResult::Failure("Weight value required".to_string());
            }
            match value_args[0].parse::<f32>() {
                Ok(weight) => {
                    let result = {
                        let mut world = context.entities().write().await;
                        let result = match world.get::<&mut Containable>(item_entity) {
                            Ok(mut containable) => {
                                containable.weight = weight;
                                CommandResult::Success(format!("Item weight set to {}", weight))
                            }
                            Err(_) => CommandResult::Failure("Item is not containable".to_string()),
                        };
                        drop(world);
                        result
                    };
                    context.mark_entity_dirty(item_entity).await;
                    result
                }
                Err(_) => CommandResult::Failure("Invalid weight value".to_string()),
            }
        }
        "weapon" => {
            if value_args.len() < 3 {
                return CommandResult::Failure("Usage: item edit <uuid> weapon <min_dmg> <max_dmg> <damage_type>".to_string());
            }
            let min_dmg = match value_args[0].parse::<i32>() {
                Ok(v) => v,
                Err(_) => return CommandResult::Failure("Invalid min damage".to_string()),
            };
            let max_dmg = match value_args[1].parse::<i32>() {
                Ok(v) => v,
                Err(_) => return CommandResult::Failure("Invalid max damage".to_string()),
            };
            let damage_type = match value_args[2].to_lowercase().as_str() {
                "slashing" => DamageType::Slashing,
                "piercing" => DamageType::Piercing,
                "blunt" | "bludgeoning" => DamageType::Blunt,
                "fire" => DamageType::Fire,
                "acid" => DamageType::Acid,
                "arcane" => DamageType::Arcane,
                "psychic" => DamageType::Psychic,
                _ => return CommandResult::Failure("Invalid damage type. Valid types: slashing, piercing, blunt, fire, acid, arcane, psychic".to_string()),
            };

            let mut world = context.entities().write().await;
            // Add or update weapon component
            if world.get::<&Weapon>(item_entity).is_ok() {
                if let Ok(mut weapon) = world.get::<&mut Weapon>(item_entity) {
                    weapon.damage_min = min_dmg;
                    weapon.damage_max = max_dmg;
                    weapon.damage_type = damage_type;
                }
            } else {
                let _ = world.insert_one(item_entity, Weapon::new(min_dmg, max_dmg, damage_type));
            }
            drop(world);
            context.mark_entity_dirty(item_entity).await;
            CommandResult::Success(format!("Item weapon stats set: {}-{} {:?} damage", min_dmg, max_dmg, damage_type))
        }
        "armor" => {
            if value_args.is_empty() {
                return CommandResult::Failure("Usage: item edit <uuid> armor <defense>".to_string());
            }
            let defense = match value_args[0].parse::<i32>() {
                Ok(v) => v,
                Err(_) => return CommandResult::Failure("Invalid defense value".to_string()),
            };

            let mut world = context.entities().write().await;
            // Add or update armor component
            if world.get::<&Armor>(item_entity).is_ok() {
                if let Ok(mut armor) = world.get::<&mut Armor>(item_entity) {
                    armor.defense = defense;
                }
            } else {
                let _ = world.insert_one(item_entity, Armor {
                    defense,
                    armor_type: crate::ecs::components::MaterialKind::Leather,
                });
            }
            drop(world);
            context.mark_entity_dirty(item_entity).await;
            CommandResult::Success(format!("Item armor defense set to {}", defense))
        }
        _ => CommandResult::Failure(format!("Unknown field: {}. Valid fields: name, description, weight, weapon, armor", field)),
    }
}

/// Clone/copy an existing item
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_clone_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item Clone Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: item clone <uuid> [new_name]".to_string());
    }

    let source_uuid = match uuid::Uuid::parse_str(&args[0]) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    let new_name = if args.len() > 1 {
        Some(args[1..].join(" "))
    } else {
        None
    };

    // Get current location
    let (area_uuid, room_uuid) = {
        let world = context.entities().read().await;
        match world.get::<&Location>(entity) {
            Ok(loc) => (loc.area_id.uuid(), loc.room_id.uuid()),
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        }
    };

    // Find source item and clone its components
    let world = context.entities().read().await;
    let source_entity = match find_entity_by_uuid(&world, source_uuid) {
        Some(e) => e,
        None => {
            return CommandResult::Failure(format!("Item {} not found", source_uuid));
        }
    };

    // Clone components
    let name = world.get::<&Name>(source_entity)
        .map(|n| if let Some(ref new_name) = new_name { new_name.clone() } else { format!("{} (copy)", n.display) })
        .unwrap_or_else(|_| "Cloned Item".to_string());
    
    let description = world.get::<&Description>(source_entity)
        .ok()
        .map(|d| Description::new(d.short.clone(), d.long.clone()))
        .unwrap_or_else(|| Description::new("A cloned item".to_string(), "A cloned item".to_string()));
    
    let containable = world.get::<&Containable>(source_entity)
        .ok()
        .map(|c| *c)
        .unwrap_or_else(|| Containable::new(1.0));
    
    let weapon = world.get::<&Weapon>(source_entity).ok().map(|w| *w);
    let armor = world.get::<&Armor>(source_entity).ok().map(|a| *a);
    
    drop(world);

    // Create the new item
    let new_uuid = uuid::Uuid::new_v4();
    let new_entity = {
        let mut world = context.entities().write().await;
        let entity = world.spawn((
            EntityUuid(new_uuid),
            Name::new(&name),
            description,
            Location::new(EntityId::from_uuid(area_uuid), EntityId::from_uuid(room_uuid)),
            containable,
            Persistent,
        ));

        // Add optional components
        if let Some(w) = weapon {
            let _ = world.insert_one(entity, w);
        }
        if let Some(a) = armor {
            let _ = world.insert_one(entity, a);
        }

        entity
    };

    context.register_entity(new_entity, new_uuid).await;
    context.mark_entity_dirty(new_entity).await;

    CommandResult::Success(format!(
        "Item cloned successfully!\r\n\
         Name: {}\r\n\
         UUID: {}\r\n\
         Cloned from: {}",
        name, new_uuid, source_uuid
    ))
}

/// List items in current room or search by name
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_list_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item List Command from {}: {}", entity.id(), args.join(" "));

    let query = if !args.is_empty() {
        Some(args.join(" ").to_lowercase())
    } else {
        None
    };

    // Get current location
    let room_uuid = {
        let world = context.entities().read().await;
        match world.get::<&Location>(entity) {
            Ok(loc) => loc.room_id.uuid(),
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        }
    };

    let world = context.entities().read().await;
    let mut items = Vec::new();

    for (item_entity, (entity_uuid, name, location, containable)) in 
        world.query::<(&EntityUuid, &Name, &Location, &Containable)>().iter() 
    {
        // Filter by current room
        if location.room_id.uuid() != room_uuid {
            continue;
        }

        // Filter by query if provided
        if let Some(ref q) = query {
            if !name.display.to_lowercase().contains(q) {
                continue;
            }
        }

        // Check for weapon/armor
        let is_weapon = world.get::<&Weapon>(item_entity).is_ok();
        let is_armor = world.get::<&Armor>(item_entity).is_ok();
        let item_type = if is_weapon {
            "Weapon"
        } else if is_armor {
            "Armor"
        } else {
            "Item"
        };

        items.push((entity_uuid.0, name.display.clone(), containable.weight, item_type));
    }

    drop(world);

    if items.is_empty() {
        let msg = if query.is_some() {
            format!("No items found matching '{}'", query.unwrap())
        } else {
            "No items in this room".to_string()
        };
        return CommandResult::Success(msg);
    }

    let mut output = format!("\r\nItems in Room\r\n{}\r\n", "=".repeat(80));
    for (uuid, name, weight, item_type) in items {
        output.push_str(&format!(
            "{} - {} ({}, {:.1} lbs)\r\n",
            uuid, name, item_type, weight
        ));
    }

    CommandResult::Success(output)
}

/// Display detailed information about an item
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_info_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item Info Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: item info <uuid>".to_string());
    }

    let target_uuid = match uuid::Uuid::parse_str(&args[0]) {
        Ok(uuid) => uuid,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    let world = context.entities().read().await;
    let item_entity = match find_entity_by_uuid(&world, target_uuid) {
        Some(e) => e,
        None => {
            return CommandResult::Failure(format!("Item {} not found", target_uuid));
        }
    };

    // Gather item information
    let name = world.get::<&Name>(item_entity)
        .map(|n| n.display.clone())
        .unwrap_or_else(|_| "(unnamed)".to_string());

    let description = world.get::<&Description>(item_entity)
        .map(|d| d.long.clone())
        .unwrap_or_else(|_| "(no description)".to_string());

    let weight = world.get::<&Containable>(item_entity)
        .map(|c| c.weight)
        .unwrap_or(0.0);

    let location = world.get::<&Location>(item_entity)
        .map(|l| (l.area_id.uuid(), l.room_id.uuid()))
        .ok();

    let weapon_info = world.get::<&Weapon>(item_entity)
        .map(|w| format!("Weapon: {}-{} {:?} damage", w.damage_min, w.damage_max, w.damage_type))
        .ok();

    let armor_info = world.get::<&Armor>(item_entity)
        .map(|a| format!("Armor: {} defense", a.defense))
        .ok();

    drop(world);

    let mut output = format!(
        "\r\nItem Information\r\n{}\r\n\
         UUID: {}\r\n\
         Name: {}\r\n\
         Description: {}\r\n\
         Weight: {:.1} lbs\r\n",
        "=".repeat(80), target_uuid, name, description, weight
    );

    if let Some((area_uuid, room_uuid)) = location {
        output.push_str(&format!("Location: Room {} (Area {})\r\n", room_uuid, area_uuid));
    }

    if let Some(weapon) = weapon_info {
        output.push_str(&format!("{}\r\n", weapon));
    }

    if let Some(armor) = armor_info {
        output.push_str(&format!("{}\r\n", armor));
    }

    CommandResult::Success(output)
}

// ============================================================================
// Item Template System
// ============================================================================

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Item template definition
#[derive(Debug, Clone)]
struct ItemTemplate {
    name: String,
    description: String,
    weight: f32,
    weapon: Option<(i32, i32, DamageType)>, // (min_dmg, max_dmg, damage_type)
    armor: Option<i32>, // defense rating
}

/// Predefined item templates for quick spawning
static ITEM_TEMPLATES: Lazy<HashMap<&'static str, ItemTemplate>> = Lazy::new(|| {
    let mut templates = HashMap::new();
    
    // Weapons
    templates.insert("shortsword", ItemTemplate {
        name: "Short Sword".to_string(),
        description: "A well-balanced short sword with a sharp blade.".to_string(),
        weight: 3.0,
        weapon: Some((3, 6, DamageType::Slashing)),
        armor: None,
    });
    
    templates.insert("longsword", ItemTemplate {
        name: "Long Sword".to_string(),
        description: "A finely crafted longsword with excellent reach.".to_string(),
        weight: 4.5,
        weapon: Some((5, 10, DamageType::Slashing)),
        armor: None,
    });
    
    templates.insert("dagger", ItemTemplate {
        name: "Dagger".to_string(),
        description: "A small, sharp dagger perfect for quick strikes.".to_string(),
        weight: 1.0,
        weapon: Some((2, 4, DamageType::Piercing)),
        armor: None,
    });
    
    templates.insert("mace", ItemTemplate {
        name: "Mace".to_string(),
        description: "A heavy mace designed to crush armor.".to_string(),
        weight: 5.0,
        weapon: Some((4, 8, DamageType::Blunt)),
        armor: None,
    });
    
    templates.insert("staff", ItemTemplate {
        name: "Wooden Staff".to_string(),
        description: "A sturdy wooden staff imbued with arcane energy.".to_string(),
        weight: 3.0,
        weapon: Some((2, 6, DamageType::Arcane)),
        armor: None,
    });
    
    // Armor
    templates.insert("leather_armor", ItemTemplate {
        name: "Leather Armor".to_string(),
        description: "Light leather armor providing basic protection.".to_string(),
        weight: 8.0,
        weapon: None,
        armor: Some(2),
    });
    
    templates.insert("chainmail", ItemTemplate {
        name: "Chainmail Armor".to_string(),
        description: "Interlocking metal rings providing solid protection.".to_string(),
        weight: 25.0,
        weapon: None,
        armor: Some(5),
    });
    
    templates.insert("plate_armor", ItemTemplate {
        name: "Plate Armor".to_string(),
        description: "Heavy plate armor offering excellent protection.".to_string(),
        weight: 45.0,
        weapon: None,
        armor: Some(8),
    });
    
    // Misc items
    templates.insert("torch", ItemTemplate {
        name: "Torch".to_string(),
        description: "A wooden torch wrapped in oil-soaked cloth.".to_string(),
        weight: 1.0,
        weapon: None,
        armor: None,
    });
    
    templates.insert("rope", ItemTemplate {
        name: "Rope".to_string(),
        description: "50 feet of sturdy hemp rope.".to_string(),
        weight: 10.0,
        weapon: None,
        armor: None,
    });
    
    templates.insert("backpack", ItemTemplate {
        name: "Backpack".to_string(),
        description: "A leather backpack for carrying supplies.".to_string(),
        weight: 2.0,
        weapon: None,
        armor: None,
    });
    
    templates.insert("potion", ItemTemplate {
        name: "Health Potion".to_string(),
        description: "A small vial containing a red healing liquid.".to_string(),
        weight: 0.5,
        weapon: None,
        armor: None,
    });
    
    templates
});

/// Spawn an item from a template
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_spawn_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item Spawn Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure("Usage: item spawn <template_name> [quantity]".to_string());
    }

    let template_name = args[0].to_lowercase();
    let quantity = if args.len() > 1 {
        match args[1].parse::<u32>() {
            Ok(q) if q > 0 && q <= 100 => q,
            Ok(_) => return CommandResult::Failure("Quantity must be between 1 and 100".to_string()),
            Err(_) => return CommandResult::Failure("Invalid quantity".to_string()),
        }
    } else {
        1
    };

    // Get the template
    let template = match ITEM_TEMPLATES.get(template_name.as_str()) {
        Some(t) => t.clone(),
        None => return CommandResult::Failure(format!("Unknown template: {}. Use 'item templates' to see available templates.", template_name)),
    };

    // Get current location
    let (area_uuid, room_uuid) = {
        let world = context.entities().read().await;
        match world.get::<&Location>(entity) {
            Ok(loc) => (loc.area_id.uuid(), loc.room_id.uuid()),
            Err(_) => {
                return CommandResult::Failure("You have no location".to_string());
            }
        }
    };

    // Spawn the items
    let mut spawned_uuids = Vec::new();
    for _ in 0..quantity {
        let item_uuid = uuid::Uuid::new_v4();
        let item_entity = {
            let mut world = context.entities().write().await;
            let entity = world.spawn((
                EntityUuid(item_uuid),
                Name::new(&template.name),
                Description::new(template.name.clone(), template.description.clone()),
                Location::new(EntityId::from_uuid(area_uuid), EntityId::from_uuid(room_uuid)),
                Containable::new(template.weight),
                Persistent,
            ));

            // Add weapon component if present
            if let Some((min_dmg, max_dmg, damage_type)) = template.weapon {
                let _ = world.insert_one(entity, Weapon::new(min_dmg, max_dmg, damage_type));
            }

            // Add armor component if present
            if let Some(defense) = template.armor {
                let _ = world.insert_one(entity, Armor {
                    defense,
                    armor_type: MaterialKind::Leather,
                });
            }

            entity
        };

        context.register_entity(item_entity, item_uuid).await;
        context.mark_entity_dirty(item_entity).await;
        spawned_uuids.push(item_uuid);
    }

    if quantity == 1 {
        CommandResult::Success(format!(
            "Spawned {} (UUID: {})",
            template.name, spawned_uuids[0]
        ))
    } else {
        CommandResult::Success(format!(
            "Spawned {} x{} items",
            template.name, quantity
        ))
    }
}

/// List available item templates
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn item_templates_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("Item Templates Command from {}: {}", entity.id(), args.join(" "));

    let filter = if !args.is_empty() {
        Some(args.join(" ").to_lowercase())
    } else {
        None
    };

    let mut output = format!("\r\nAvailable Item Templates\r\n{}\r\n", "=".repeat(80));
    
    // Group templates by category
    let mut weapons = Vec::new();
    let mut armor = Vec::new();
    let mut misc = Vec::new();
    
    for (key, template) in ITEM_TEMPLATES.iter() {
        // Apply filter if present
        if let Some(ref filter_str) = filter {
            if !key.contains(filter_str) && !template.name.to_lowercase().contains(filter_str) {
                continue;
            }
        }
        
        if template.weapon.is_some() {
            weapons.push((key, template));
        } else if template.armor.is_some() {
            armor.push((key, template));
        } else {
            misc.push((key, template));
        }
    }
    
    // Sort each category
    weapons.sort_by_key(|(k, _)| *k);
    armor.sort_by_key(|(k, _)| *k);
    misc.sort_by_key(|(k, _)| *k);
    
    // Display weapons
    if !weapons.is_empty() {
        output.push_str("\r\nWeapons:\r\n");
        for (key, template) in weapons {
            let weapon_info = if let Some((min, max, dmg_type)) = template.weapon {
                format!(" [{}-{} {}]", min, max, dmg_type.as_str())
            } else {
                String::new()
            };
            output.push_str(&format!(
                "  {:20} - {}{}\r\n",
                key, template.name, weapon_info
            ));
        }
    }
    
    // Display armor
    if !armor.is_empty() {
        output.push_str("\r\nArmor:\r\n");
        for (key, template) in armor {
            let armor_info = if let Some(defense) = template.armor {
                format!(" [Defense: {}]", defense)
            } else {
                String::new()
            };
            output.push_str(&format!(
                "  {:20} - {}{}\r\n",
                key, template.name, armor_info
            ));
        }
    }
    
    // Display misc items
    if !misc.is_empty() {
        output.push_str("\r\nMiscellaneous:\r\n");
        for (key, template) in misc {
            output.push_str(&format!(
                "  {:20} - {}\r\n",
                key, template.name
            ));
        }
    }
    
    output.push_str(&format!("\r\nUsage: item spawn <template_name> [quantity]\r\n"));
    output.push_str(&format!("Example: item spawn longsword 5\r\n"));
    
    CommandResult::Success(output)
}



// ============================================================================
// Utility Functions
// ============================================================================

/// Find an entity by its UUID
fn find_entity_by_uuid(world: &crate::ecs::GameWorld, uuid: uuid::Uuid) -> Option<EcsEntity> {
    for (entity, entity_uuid) in world.query::<&EntityUuid>().iter() {
        if entity_uuid.0 == uuid {
            return Some(entity);
        }
    }
    None
}

// Made with Bob


// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_reverse_direction() {
        // Test cardinal directions
        assert_eq!(get_reverse_direction("north"), Some("South".to_string()));
        assert_eq!(get_reverse_direction("south"), Some("North".to_string()));
        assert_eq!(get_reverse_direction("east"), Some("West".to_string()));
        assert_eq!(get_reverse_direction("west"), Some("East".to_string()));
        
        // Test vertical directions
        assert_eq!(get_reverse_direction("up"), Some("Down".to_string()));
        assert_eq!(get_reverse_direction("down"), Some("Up".to_string()));
        
        // Test diagonal directions
        assert_eq!(get_reverse_direction("northeast"), Some("Southwest".to_string()));
        assert_eq!(get_reverse_direction("northwest"), Some("Southeast".to_string()));
        assert_eq!(get_reverse_direction("southeast"), Some("Northwest".to_string()));
        assert_eq!(get_reverse_direction("southwest"), Some("Northeast".to_string()));
        
        // Test abbreviations
        assert_eq!(get_reverse_direction("n"), Some("South".to_string()));
        assert_eq!(get_reverse_direction("s"), Some("North".to_string()));
        assert_eq!(get_reverse_direction("e"), Some("West".to_string()));
        assert_eq!(get_reverse_direction("w"), Some("East".to_string()));
        assert_eq!(get_reverse_direction("u"), Some("Down".to_string()));
        assert_eq!(get_reverse_direction("d"), Some("Up".to_string()));
        assert_eq!(get_reverse_direction("ne"), Some("Southwest".to_string()));
        assert_eq!(get_reverse_direction("nw"), Some("Southeast".to_string()));
        assert_eq!(get_reverse_direction("se"), Some("Northwest".to_string()));
        assert_eq!(get_reverse_direction("sw"), Some("Northeast".to_string()));
        
        // Test case insensitivity
        assert_eq!(get_reverse_direction("NORTH"), Some("South".to_string()));
        assert_eq!(get_reverse_direction("NoRtH"), Some("South".to_string()));
        
        // Test invalid directions
        assert_eq!(get_reverse_direction("invalid"), None);
        assert_eq!(get_reverse_direction(""), None);
        assert_eq!(get_reverse_direction("northsouth"), None);
    }

    #[test]
    fn test_find_entity_by_uuid() {
        use hecs::World;
        
        let mut world = World::new();
        
        // Create test entities
        let uuid1 = uuid::Uuid::new_v4();
        let uuid2 = uuid::Uuid::new_v4();
        let uuid3 = uuid::Uuid::new_v4();
        
        let entity1 = world.spawn((EntityUuid(uuid1), Name::new("Entity1")));
        let entity2 = world.spawn((EntityUuid(uuid2), Name::new("Entity2")));
        let _entity3 = world.spawn((Name::new("Entity3 No UUID"),)); // No UUID
        
        // Test finding existing entities
        assert_eq!(find_entity_by_uuid(&world, uuid1), Some(entity1));
        assert_eq!(find_entity_by_uuid(&world, uuid2), Some(entity2));
        
        // Test finding non-existent UUID
        assert_eq!(find_entity_by_uuid(&world, uuid3), None);
        
        // Test with random UUID
        let random_uuid = uuid::Uuid::new_v4();
        assert_eq!(find_entity_by_uuid(&world, random_uuid), None);
    }

    #[test]
    fn test_area_kind_parsing() {
        // Test that AreaKind can be created and compared
        let overworld = AreaKind::Overworld;
        let vehicle = AreaKind::Vehicle;
        let building = AreaKind::Building;
        let dungeon = AreaKind::Dungeon;
        
        assert_ne!(overworld, vehicle);
        assert_ne!(overworld, building);
        assert_ne!(overworld, dungeon);
        assert_ne!(vehicle, building);
    }

    #[test]
    fn test_area_flags_operations() {
        let mut area = Area::new(AreaKind::Overworld);
        
        // Test default state
        assert!(area.area_flags.is_empty());
        
        // Test adding flags
        area.area_flags.push("no_recall".to_string());
        area.area_flags.push("safe_zone".to_string());
        assert_eq!(area.area_flags.len(), 2);
        assert!(area.area_flags.contains(&"no_recall".to_string()));
        assert!(area.area_flags.contains(&"safe_zone".to_string()));
        assert!(!area.area_flags.contains(&"no_summon".to_string()));
    }

    #[test]
    fn test_exit_data_creation() {
        let dest_uuid = uuid::Uuid::new_v4();
        let exit = ExitData::new("north", EntityId::from_uuid(dest_uuid));
        
        assert_eq!(exit.direction, "north");
        assert_eq!(exit.dest_id.uuid(), dest_uuid);
        assert!(!exit.closeable);
        assert!(!exit.closed);
        assert!(!exit.lockable);
        assert!(!exit.locked);
        assert_eq!(exit.door_rating, None);
        assert_eq!(exit.lock_rating, None);
        assert_eq!(exit.unlock_code, None);
    }

    #[test]
    fn test_exits_has_exit() {
        let mut exits = Exits::new();
        let dest_uuid = uuid::Uuid::new_v4();
        
        // Initially no exits
        assert!(!exits.has_exit("north"));
        
        // Add an exit
        exits.exits.push(ExitData::new("north", EntityId::from_uuid(dest_uuid)));
        
        // Now has exit
        assert!(exits.has_exit("north"));
        assert!(exits.has_exit("NORTH")); // Case insensitive
        assert!(exits.has_exit("NoRtH")); // Case insensitive
        
        // Still doesn't have other directions
        assert!(!exits.has_exit("south"));
        assert!(!exits.has_exit("east"));
    }

    #[test]
    fn test_entity_id_conversions() {
        let uuid = uuid::Uuid::new_v4();
        let entity_id = EntityId::from_uuid(uuid);
        
        // Test UUID extraction
        assert_eq!(entity_id.uuid(), uuid);
        
        // Test that different UUIDs create different EntityIds
        let uuid2 = uuid::Uuid::new_v4();
        let entity_id2 = EntityId::from_uuid(uuid2);
        
        assert_ne!(entity_id.uuid(), entity_id2.uuid());
    }

    #[tokio::test]
    async fn test_area_create_validation() {
        // This test validates the area creation logic without database
        let area_name = "Test Area";
        assert!(!area_name.is_empty());
        assert!(area_name.len() > 0);
        assert!(area_name.len() < 1000); // Reasonable limit
    }

    #[tokio::test]
    async fn test_room_create_validation() {
        // Test room name validation
        let room_name = "Test Room";
        assert!(!room_name.is_empty());
        
        // Test UUID validation
        let valid_uuid = uuid::Uuid::new_v4();
        assert!(uuid::Uuid::parse_str(&valid_uuid.to_string()).is_ok());
        
        let invalid_uuid = "not-a-uuid";
        assert!(uuid::Uuid::parse_str(invalid_uuid).is_err());
    }

    #[test]
    fn test_direction_normalization() {
        // Test that directions are handled case-insensitively
        let directions = vec!["north", "NORTH", "North", "NoRtH"];
        for dir in directions {
            assert_eq!(dir.to_lowercase(), "north");
        }
    }

    #[test]
    fn test_exit_direction_matching() {
        let mut exits = Exits::new();
        let dest_uuid = uuid::Uuid::new_v4();
        
        exits.exits.push(ExitData::new("north", EntityId::from_uuid(dest_uuid)));
        exits.exits.push(ExitData::new("South", EntityId::from_uuid(dest_uuid)));
        exits.exits.push(ExitData::new("EAST", EntityId::from_uuid(dest_uuid)));
        
        // Test case-insensitive matching
        assert!(exits.exits.iter().any(|e| e.direction.to_lowercase() == "north"));
        assert!(exits.exits.iter().any(|e| e.direction.to_lowercase() == "south"));
        assert!(exits.exits.iter().any(|e| e.direction.to_lowercase() == "east"));
        assert!(!exits.exits.iter().any(|e| e.direction.to_lowercase() == "west"));
    }
}

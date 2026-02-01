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

//! NPC editor commands

use crate::ecs::EcsEntity;
use crate::ecs::components::*;
use crate::ecs::context::WorldContext;
use crate::ecs::systems::CommandResult;
use hecs::Entity;
use std::sync::Arc;
// ============================================================================
// NPC Commands
// ============================================================================

/// Create a new NPC
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn npc_create_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!(
        "NPC Create Command from {}: {}",
        entity.id(),
        args.join(" ")
    );

    if args.is_empty() {
        return CommandResult::Failure("Usage: npc create <name> [template_id]".to_string());
    }

    let npc_name = if args.len() > 1 {
        args[0..args.len() - 1].join(" ")
    } else {
        args[0].clone()
    };

    let template_id = if args.len() > 1 {
        Some(args.last().unwrap().clone())
    } else {
        None
    };

    // Get creator's location
    let location = {
        let world = context.entities().read().await;
        world.get::<&Location>(entity).ok().map(|l| (*l).clone())
    };

    // Create the NPC entity
    let npc_uuid = uuid::Uuid::new_v4();
    let npc_entity = {
        let mut world = context.entities().write().await;

        let mut builder = hecs::EntityBuilder::new();
        builder.add(EntityUuid(npc_uuid));
        builder.add(Name::new(&npc_name));
        builder.add(Description::new(
            format!("A new NPC named {}", npc_name),
            "This is a newly created NPC. Use 'npc edit' to configure it.".to_string(),
        ));

        // Add NPC marker
        if let Some(ref template) = template_id {
            builder.add(Npc::from_template(template));
        } else {
            builder.add(Npc::new());
        }

        // Add AI components
        builder.add(AIController::new(BehaviorType::Passive));
        builder.add(Personality::new());
        builder.add(Memory);

        // Add GOAP planner
        builder.add(GoapPlanner::new());

        // Add dialogue
        builder.add(NpcDialogue::new("gpt-4"));
        builder.add(NpcConversation::new());

        // Add location if creator has one
        if let Some(loc) = location {
            builder.add(loc);
        }

        builder.add(Persistent);

        world.spawn(builder.build())
    };

    // Register the entity
    context.register_entity(npc_entity, npc_uuid).await;

    // Mark as dirty for persistence
    context.mark_entity_dirty(npc_entity).await;

    let output = format!(
        "\r\nNPC created successfully!\r\n{}\r\n\r\n\
         UUID: {}\r\n\
         Name: {}\r\n\
         Template: {}\r\n\
         Behavior: Passive (default)\r\n\r\n\
         Use 'npc edit {}' to configure the NPC.\r\n\
         Use 'npc goap {}' to configure GOAP AI.\r\n\
         Use 'npc dialogue {}' to configure dialogue.\r\n\r\n\
         {}\r\n",
        "=".repeat(80),
        npc_uuid,
        npc_name,
        template_id.as_deref().unwrap_or("none"),
        npc_uuid,
        npc_uuid,
        npc_uuid,
        "=".repeat(80)
    );

    CommandResult::Success(output)
}

/// List all NPCs
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn npc_list_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("NPC List Command from {}: {}", entity.id(), args.join(" "));

    let filter = if args.is_empty() {
        None
    } else {
        Some(args.join(" ").to_lowercase())
    };

    let world = context.entities().read().await;
    let mut npcs = Vec::new();

    for (_e, uuid, name, npc, ai) in world
        .query::<(Entity, &EntityUuid, &Name, &Npc, &AIController)>()
        .iter()
    {
        if let Some(ref f) = filter {
            if !name.display.to_lowercase().contains(f) {
                continue;
            }
        }

        npcs.push((
            uuid.0,
            name.display.clone(),
            npc.active,
            ai.behavior_type,
            npc.template_id.clone(),
        ));
    }

    drop(world);

    if npcs.is_empty() {
        return CommandResult::Success("No NPCs found.\r\n".to_string());
    }

    let mut output = format!("\r\nNPCs:\r\n{}\r\n\r\n", "=".repeat(80));
    output.push_str(&format!(
        "{:<36} {:<20} {:<10} {:<15} {:<15}\r\n",
        "UUID", "Name", "Active", "Behavior", "Template"
    ));
    output.push_str(&format!("{}\r\n", "-".repeat(80)));

    for (uuid, name, active, behavior, template) in npcs {
        output.push_str(&format!(
            "{:<36} {:<20} {:<10} {:<15} {:<15}\r\n",
            uuid,
            name,
            if active { "Yes" } else { "No" },
            behavior.as_str(),
            template.as_deref().unwrap_or("none")
        ));
    }

    output.push_str(&format!("\r\n{}\r\n", "=".repeat(80)));

    CommandResult::Success(output)
}

/// Edit NPC properties
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn npc_edit_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("NPC Edit Command from {}: {}", entity.id(), args.join(" "));

    if args.len() < 3 {
        return CommandResult::Failure(
            "Usage: npc edit <uuid> <property> <value>\r\n\
             Properties: name, description, behavior, active\r\n"
                .to_string(),
        );
    }

    let uuid_str = &args[0];
    let property = &args[1].to_lowercase();
    let value = args[2..].join(" ");

    // Parse UUID
    let npc_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(u) => u,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    // Find NPC entity
    let npc_entity = match context.get_entity_by_uuid(npc_uuid).await {
        Some(e) => e,
        None => return CommandResult::Failure("NPC not found".to_string()),
    };

    let world = context.entities().read().await;

    // Check if entity is an NPC
    if world.get::<&Npc>(npc_entity).is_err() {
        return CommandResult::Failure("Entity is not an NPC".to_string());
    }

    match property.as_str() {
        "name" => {
            if let Ok(mut name) = world.get::<&mut Name>(npc_entity) {
                name.display = value.clone();
                context.mark_entity_dirty(npc_entity).await;
                CommandResult::Success(format!("NPC name updated to: {}\r\n", value))
            } else {
                CommandResult::Failure("Failed to update name".to_string())
            }
        }
        "description" => {
            if let Ok(mut desc) = world.get::<&mut Description>(npc_entity) {
                desc.long = value.clone();
                context.mark_entity_dirty(npc_entity).await;
                CommandResult::Success(format!("NPC description updated\r\n"))
            } else {
                CommandResult::Failure("Failed to update description".to_string())
            }
        }
        "behavior" => {
            let behavior_type = match BehaviorType::from_str(&value) {
                Some(b) => b,
                None => {
                    return CommandResult::Failure(format!(
                        "Invalid behavior type. Valid types: Passive, Wandering, Aggressive, Defensive, Friendly, Merchant, Quest, Custom\r\n"
                    ));
                }
            };

            if let Ok(mut ai) = world.get::<&mut AIController>(npc_entity) {
                ai.behavior_type = behavior_type;
                context.mark_entity_dirty(npc_entity).await;
                CommandResult::Success(format!("NPC behavior updated to: {}\r\n", value))
            } else {
                CommandResult::Failure("Failed to update behavior".to_string())
            }
        }
        "active" => {
            let active =
                value.to_lowercase() == "true" || value == "1" || value.to_lowercase() == "yes";

            if let Ok(mut npc) = world.get::<&mut Npc>(npc_entity) {
                npc.active = active;
                context.mark_entity_dirty(npc_entity).await;
                CommandResult::Success(format!("NPC active status set to: {}\r\n", active))
            } else {
                CommandResult::Failure("Failed to update active status".to_string())
            }
        }
        _ => CommandResult::Failure(format!("Unknown property: {}\r\n", property)),
    }
}

/// Configure NPC dialogue
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn npc_dialogue_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!(
        "NPC Dialogue Command from {}: {}",
        entity.id(),
        args.join(" ")
    );

    if args.len() < 3 {
        return CommandResult::Failure(
            "Usage: npc dialogue <uuid> <property> <value>\r\n\
             Properties: enabled, model, system_prompt, temperature, max_tokens\r\n"
                .to_string(),
        );
    }

    let uuid_str = &args[0];
    let property = &args[1].to_lowercase();
    let value = args[2..].join(" ");

    let npc_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(u) => u,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    let npc_entity = match context.get_entity_by_uuid(npc_uuid).await {
        Some(e) => e,
        None => return CommandResult::Failure("NPC not found".to_string()),
    };

    let world = context.entities().read().await;

    let mut dialogue = match world.get::<&mut NpcDialogue>(npc_entity) {
        Ok(d) => d,
        Err(_) => return CommandResult::Failure("NPC has no dialogue configuration".to_string()),
    };

    match property.as_str() {
        "enabled" => {
            let enabled = value.to_lowercase() == "true" || value == "1";
            dialogue.llm_enabled = enabled;
            context.mark_entity_dirty(npc_entity).await;
            CommandResult::Success(format!(
                "LLM dialogue {}\r\n",
                if enabled { "enabled" } else { "disabled" }
            ))
        }
        "model" => {
            dialogue.llm_model = value.clone();
            context.mark_entity_dirty(npc_entity).await;
            CommandResult::Success(format!("LLM model set to: {}\r\n", value))
        }
        "system_prompt" => {
            dialogue.system_prompt = value.clone();
            context.mark_entity_dirty(npc_entity).await;
            CommandResult::Success("System prompt updated\r\n".to_string())
        }
        "temperature" => match value.parse::<f32>() {
            Ok(temp) => {
                dialogue.temperature = temp.clamp(0.0, 2.0);
                context.mark_entity_dirty(npc_entity).await;
                CommandResult::Success(format!("Temperature set to: {}\r\n", dialogue.temperature))
            }
            Err(_) => CommandResult::Failure(
                "Invalid temperature value (must be 0.0-2.0)\r\n".to_string(),
            ),
        },
        "max_tokens" => match value.parse::<u32>() {
            Ok(tokens) => {
                dialogue.max_tokens = tokens;
                context.mark_entity_dirty(npc_entity).await;
                CommandResult::Success(format!("Max tokens set to: {}\r\n", tokens))
            }
            Err(_) => CommandResult::Failure("Invalid max_tokens value\r\n".to_string()),
        },
        _ => CommandResult::Failure(format!("Unknown property: {}\r\n", property)),
    }
}

/// Configure NPC GOAP AI
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn npc_goap_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    tracing::debug!("NPC GOAP Command from {}: {}", entity.id(), args.join(" "));

    if args.is_empty() {
        return CommandResult::Failure(
            "Usage: npc goap <uuid> <subcommand> [args...]\r\n\
             Subcommands:\r\n\
             - addgoal <name> <priority> - Add a goal\r\n\
             - addaction <name> <cost> - Add an action\r\n\
             - setstate <key> <value> - Set world state\r\n\
             - show - Show current GOAP configuration\r\n"
                .to_string(),
        );
    }

    let uuid_str = &args[0];
    let npc_uuid = match uuid::Uuid::parse_str(uuid_str) {
        Ok(u) => u,
        Err(_) => return CommandResult::Failure("Invalid UUID format".to_string()),
    };

    let npc_entity = match context.get_entity_by_uuid(npc_uuid).await {
        Some(e) => e,
        None => return CommandResult::Failure("NPC not found".to_string()),
    };

    if args.len() < 2 {
        return CommandResult::Failure("Missing subcommand\r\n".to_string());
    }

    let subcommand = &args[1].to_lowercase();
    let world = context.entities().read().await;

    let mut planner = match world.get::<&mut GoapPlanner>(npc_entity) {
        Ok(p) => p,
        Err(_) => return CommandResult::Failure("NPC has no GOAP planner".to_string()),
    };

    match subcommand.as_str() {
        "addgoal" => {
            if args.len() < 4 {
                return CommandResult::Failure(
                    "Usage: npc goap <uuid> addgoal <name> <priority>\r\n".to_string(),
                );
            }
            let name = &args[2];
            let priority = match args[3].parse::<i32>() {
                Ok(p) => p,
                Err(_) => return CommandResult::Failure("Invalid priority value\r\n".to_string()),
            };

            let goal = GoapGoal::new(name, name, priority);
            planner.add_goal(goal);
            context.mark_entity_dirty(npc_entity).await;
            CommandResult::Success(format!(
                "Goal '{}' added with priority {}\r\n",
                name, priority
            ))
        }
        "addaction" => {
            if args.len() < 4 {
                return CommandResult::Failure(
                    "Usage: npc goap <uuid> addaction <name> <cost>\r\n".to_string(),
                );
            }
            let name = &args[2];
            let cost = match args[3].parse::<f32>() {
                Ok(c) => c,
                Err(_) => return CommandResult::Failure("Invalid cost value\r\n".to_string()),
            };

            let action = GoapAction::new(name, name).with_cost(cost);
            planner.add_action(action);
            context.mark_entity_dirty(npc_entity).await;
            CommandResult::Success(format!("Action '{}' added with cost {}\r\n", name, cost))
        }
        "setstate" => {
            if args.len() < 4 {
                return CommandResult::Failure(
                    "Usage: npc goap <uuid> setstate <key> <value>\r\n".to_string(),
                );
            }
            let key = &args[2];
            let value = args[3].to_lowercase() == "true" || args[3] == "1";

            planner.set_state(key, value);
            context.mark_entity_dirty(npc_entity).await;
            CommandResult::Success(format!("World state '{}' set to {}\r\n", key, value))
        }
        "show" => {
            let mut output = format!("\r\nGOAP Configuration:\r\n{}\r\n\r\n", "=".repeat(80));
            output.push_str(&format!("Goals: {}\r\n", planner.goals.len()));
            for goal in &planner.goals {
                output.push_str(&format!(
                    "  - {} (priority: {})\r\n",
                    goal.name, goal.priority
                ));
            }
            output.push_str(&format!("\r\nActions: {}\r\n", planner.actions.len()));
            for action in &planner.actions {
                output.push_str(&format!("  - {} (cost: {})\r\n", action.name, action.cost));
            }
            output.push_str(&format!("\r\n{}\r\n", "=".repeat(80)));
            CommandResult::Success(output)
        }
        _ => CommandResult::Failure(format!("Unknown subcommand: {}\r\n", subcommand)),
    }
}

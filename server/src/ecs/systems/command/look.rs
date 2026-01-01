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

use crate::ecs::components::{Description, EntityUuid, Exits, Location, Name, Room};
use crate::ecs::context::WorldContext;
use crate::ecs::systems::CommandResult;
use crate::ecs::EcsEntity;
use std::sync::Arc;

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn look_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    let world = context.entities().read().await;
    tracing::debug!("Look Command from {}: {}", entity.id(), args.join(" "));

    let result = if args.is_empty() {
        // Look at the current room
        if let Ok(loc) = world.get::<&Location>(entity) {
            tracing::debug!("Entity {} is in room {}", entity.id(), loc.room_id);
            let room_id = loc.room_id;

            // Try to find the room entity and get its details
            let mut room_info = None;
            for (room_entity, (_room_comp, name, desc)) in
                world.query::<(&Room, &Name, &Description)>().iter()
            {
                if let Ok(room_uuid) = world.get::<&EntityUuid>(room_entity)
                {
                    if room_uuid.0 == room_id.uuid() {
                        // Get exits for this room
                        let exit_directions = if let Ok(exits) = world.get::<&Exits>(room_entity) {
                            exits
                                .exits
                                .iter()
                                .map(|e| {
                                    if e.locked {
                                        format!("{} (locked)", e.direction)
                                    } else if e.closed {
                                        format!("{} (closed)", e.direction)
                                    } else {
                                        e.direction.clone()
                                    }
                                })
                                .collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        };

                        room_info =
                            Some((name.display.clone(), desc.long.clone(), exit_directions));
                        break;
                    }
                }
            }

            if let Some((name, description, exit_directions)) = room_info {
                let mut output = format!("\r\n{}\r\n", name);
                output.push_str(&format!("{}\r\n", "=".repeat(name.len())));
                output.push_str(&format!("{}\r\n", description));

                // List exits from the Exits component
                if !exit_directions.is_empty() {
                    output.push_str("\r\nExits:\r\n");
                    for exit in exit_directions {
                        output.push_str(&format!("  {}\r\n", exit));
                    }
                }

                // List visible entities in the room (excluding self)
                let mut entities_here = Vec::new();
                for (other_entity, (other_loc, other_name)) in
                    world.query::<(&Location, &Name)>().iter()
                {
                    if other_entity != entity
                        && other_loc.room_id == room_id
                        && other_loc.area_id == loc.area_id
                    {
                        entities_here.push(other_name.display.clone());
                    }
                }

                if !entities_here.is_empty() {
                    output.push_str("\r\nAlso here:\r\n");
                    for ent in entities_here {
                        output.push_str(&format!("  - {}\r\n", ent));
                    }
                }

                CommandResult::Success(output)
            } else {
                tracing::warn!("No room ({}) found for entity ({})", room_id, entity.id());
                CommandResult::Success(format!(
                    "\r\nYou are in an undefined location.\r\nArea: {}\r\nRoom: {}\r\n",
                    loc.area_id, loc.room_id
                ))
            }
        } else {
            CommandResult::Failure("You have no location".to_string())
        }
    } else {
        // Look at specific target
        let target = args.join(" ");

        // Try to find the target in the current room
        if let Ok(loc) = world.get::<&Location>(entity) {
            let mut found = None;
            for (other_entity, (other_loc, other_name, other_desc)) in
                world.query::<(&Location, &Name, &Description)>().iter()
            {
                if other_entity != entity
                    && other_loc.room_id == loc.room_id
                    && other_loc.area_id == loc.area_id
                    && other_name.matches(&target)
                {
                    found = Some(CommandResult::Success(format!(
                        "{}\r\n{}",
                        other_name.display, other_desc.long
                    )));
                    break;
                }
            }

            found.unwrap_or_else(|| CommandResult::Failure(format!("You don't see '{}' here.", target)))
        } else {
            CommandResult::Failure(format!("You don't see '{}' here.", target))
        }
    };

    drop(world);
    result
}

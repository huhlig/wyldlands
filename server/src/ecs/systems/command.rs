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

//! Command system for processing player and NPC commands

mod admin;
mod comms;
mod exit;
mod inventory;
mod look;
mod query;
mod score;

use crate::ecs::components::Location;
use crate::ecs::context::WorldContext;
use crate::ecs::events::{EventBus, GameEvent};
use crate::ecs::{EcsEntity, GameWorld};
use std::collections::HashMap;
use std::sync::Arc;

pub type CommandFn = Box<
    dyn Fn(Arc<WorldContext>, EcsEntity, String, Vec<String>) -> std::pin::Pin<Box<dyn std::future::Future<Output = CommandResult> + Send + 'static>> + Send + Sync,
>;

#[derive(Debug, Clone)]
pub enum CommandResult {
    Success(String),
    Failure(String),
    Invalid(String),
}

struct CommandMetadata {
    handler: CommandFn,
    help_text: String,
    aliases: Vec<String>,
}

pub struct CommandSystem {
    commands: HashMap<String, CommandMetadata>,
    aliases: HashMap<String, String>,
    event_bus: EventBus,
}

impl CommandSystem {
    /// Create a new command system
    pub fn new(event_bus: EventBus) -> Self {
        let mut system = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
            event_bus,
        };

        system.register_default_commands();
        system
    }

    /// Register a command with aliases and help text
    /// TODO: Add validation for command names and aliases
    pub fn register_command<F, Fut>(&mut self, name: String, aliases: Vec<String>, help_text: String, handler: F)
    where
        F: Fn(Arc<WorldContext>, EcsEntity, String, Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CommandResult> + Send + 'static,
    {
        let handler = Box::new(move |ctx: Arc<WorldContext>, entity: EcsEntity, cmd: String, args: Vec<String>| {
            Box::pin(handler(ctx, entity, cmd, args)) as std::pin::Pin<Box<dyn std::future::Future<Output = CommandResult> + Send + 'static>>
        });
        let metadata = CommandMetadata {
            handler,
            help_text,
            aliases: aliases.clone(),
        };
        self.commands.insert(name.clone(), metadata);
        for alias in aliases {
            self.aliases.insert(alias, name.clone());
        }
    }

    /// Generate help text from registered commands
    fn generate_help(&self) -> String {
        let mut regular_commands = Vec::new();
        let mut admin_commands = Vec::new();
        let mut movement_commands = Vec::new();

        for (name, metadata) in &self.commands {
            if name.starts_with("world ") {
                admin_commands.push(&metadata.help_text);
            } else if ["north", "south", "east", "west", "up", "down", "northeast", "northwest", "southeast", "southwest"].contains(&name.as_str()) {
                movement_commands.push(&metadata.help_text);
            } else {
                regular_commands.push(&metadata.help_text);
            }
        }

        // Sort each category
        regular_commands.sort();
        admin_commands.sort();
        movement_commands.sort();

        let mut help = String::from("\r\nAvailable Commands:\r\n");
        for cmd in regular_commands {
            help.push_str("  ");
            help.push_str(cmd);
            help.push('\r');
            help.push('\n');
        }

        if !admin_commands.is_empty() {
            help.push_str("\r\nAdmin Commands:\r\n");
            for cmd in admin_commands {
                help.push_str("  ");
                help.push_str(cmd);
                help.push('\r');
                help.push('\n');
            }
        }

        if !movement_commands.is_empty() {
            help.push_str("\r\nMovement:\r\n");
            for cmd in movement_commands {
                help.push_str("  ");
                help.push_str(cmd);
                help.push('\r');
                help.push('\n');
            }
        }

        help
    }

    /// Execute a command
    #[tracing::instrument(skip(self, context), fields(entity_id = entity.id()))]
    pub async fn execute(
        &mut self,
        context: Arc<WorldContext>,
        entity: EcsEntity,
        command: &str,
        args: &[String],
    ) -> CommandResult {
        let cmd_name = command.to_lowercase();

        // Try to resolve alias
        let cmd_name = self.aliases.get(&cmd_name).unwrap_or(&cmd_name).clone();

        // Special handling for help command to access command registry
        if cmd_name == "help" {
            return CommandResult::Success(self.generate_help());
        }

        // First try exact match
        if let Some(metadata) = self.commands.get(&cmd_name) {
            let result = (metadata.handler)(context.clone(), entity, cmd_name.clone(), args.to_vec()).await;

            self.event_bus.publish(GameEvent::CommandExecuted {
                entity,
                command: command.to_string(),
                success: matches!(result, CommandResult::Success(_)),
            });

            return result;
        }

        // If no exact match and we have args, try combining command with first arg as subcommand
        if !args.is_empty() {
            let subcommand_name = format!("{} {}", cmd_name, args[0].to_lowercase());

            // Try to resolve subcommand alias
            let subcommand_name = self.aliases.get(&subcommand_name).unwrap_or(&subcommand_name).clone();

            if let Some(metadata) = self.commands.get(&subcommand_name) {
                // Pass remaining args (excluding the subcommand)
                let remaining_args = args[1..].to_vec();
                let result = (metadata.handler)(context.clone(), entity, subcommand_name.clone(), remaining_args).await;

                self.event_bus.publish(GameEvent::CommandExecuted {
                    entity,
                    command: format!("{} {}", command, args[0]),
                    success: matches!(result, CommandResult::Success(_)),
                });

                return result;
            }
        }

        CommandResult::Invalid(format!("Unknown command: {}", command))
    }

    /// Register default commands
    /// TODO: Add Enter Command
    fn register_default_commands(&mut self) {
        // Look command - enhanced to show room details
        self.register_command(
            "look".to_string(),
            vec!["l".to_string()],
            "look (l) [target]  - Look at your surroundings or a specific target".to_string(),
            |ctx, entity, cmd, args| look::look_command(ctx, entity, cmd, args),
        );

        // Inventory command
        self.register_command(
            "inventory".to_string(),
            vec!["i".to_string(), "inv".to_string()],
            "inventory (i, inv) - Check your inventory".to_string(),
            |ctx, entity, cmd, args| inventory::inventory_command(ctx, entity, cmd, args),
        );

        // Say command
        self.register_command(
            "say".to_string(),
            vec!["'".to_string()],
            "say (')            - Say something".to_string(),
            |ctx, entity, cmd, args| comms::say_command(ctx, entity, cmd, args),
        );

        // Yell command
        self.register_command(
            "yell".to_string(),
            vec!["\"".to_string()],
            "yell (\")          - Yell something".to_string(),
            |ctx, entity, cmd, args| comms::yell_command(ctx, entity, cmd, args),
        );

        // Emote command
        self.register_command(
            "emote".to_string(),
            vec!["em".to_string(), ":".to_string()],
            "emote (em, :)      - Perform an emote".to_string(),
            |ctx, entity, cmd, args| comms::emote_command(ctx, entity, cmd, args),
        );

        // Score/stats command
        self.register_command(
            "score".to_string(),
            vec!["stats".to_string()],
            "score (stats)      - View your stats".to_string(),
            |ctx, entity, cmd, args| score::score_command(ctx, entity, cmd, args),
        );

        // Exit/quit command
        self.register_command(
            "exit".to_string(),
            vec![
                "quit".to_string(),
                "logoff".to_string(),
                "logout".to_string(),
            ],
            "exit (quit, logoff, logout) - Save and return to character selection".to_string(),
            |ctx, entity, cmd, args| exit::exit_command(ctx, entity, cmd, args),
        );

        // World inspect command (admin)
        self.register_command(
            "world inspect".to_string(),
            vec!["winspect".to_string(), "query".to_string(), "inspect".to_string()],
            "world inspect (winspect) - Query all components of an entity by UUID".to_string(),
            |ctx, entity, cmd, args| admin::query_entity_command(ctx, entity, cmd, args),
        );

        // World list command (admin)
        self.register_command(
            "world list".to_string(),
            vec!["wlist".to_string(), "entities".to_string(), "list".to_string()],
            "world list (wlist)     - List all entities with their UUIDs and components".to_string(),
            |ctx, entity, cmd, args| admin::list_entities_command(ctx, entity, cmd, args),
        );

        // World save command (admin)
        self.register_command(
            "world save".to_string(),
            vec!["wsave".to_string()],
            "world save (wsave)     - Save all persistent entities to the database".to_string(),
            |ctx, entity, cmd, args| admin::world_save_command(ctx, entity, cmd, args),
        );

        // World reload command (admin)
        self.register_command(
            "world reload".to_string(),
            vec!["wreload".to_string()],
            "world reload (wreload) - Clear ECS and reload entities from database".to_string(),
            |ctx, entity, cmd, args| admin::world_reload_command(ctx, entity, cmd, args),
        );

        // Help command - dynamically generates help from registered commands
        self.register_command(
            "help".to_string(),
            vec!["?".to_string(), "commands".to_string()],
            "help (?, commands) - Show this help".to_string(),
            |_context, _entity, _cmd, _args| async {
                // This handler is never called - help is handled specially in execute()
                CommandResult::Success(String::new())
            },
        );

        // Movement commands
        self.register_movement_commands();
    }

    /// Register all movement commands
    /// TODO: Add Run Command
    fn register_movement_commands(&mut self) {
        // North
        self.register_command(
            "north".to_string(),
            vec!["n".to_string()],
            "north (n)          - Move north".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "north".to_string()),
        );

        // South
        self.register_command(
            "south".to_string(),
            vec!["s".to_string()],
            "south (s)          - Move south".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "south".to_string()),
        );

        // East
        self.register_command(
            "east".to_string(),
            vec!["e".to_string()],
            "east (e)           - Move east".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "east".to_string()),
        );

        // West
        self.register_command(
            "west".to_string(),
            vec!["w".to_string()],
            "west (w)           - Move west".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "west".to_string()),
        );

        // Up
        self.register_command(
            "up".to_string(),
            vec!["u".to_string()],
            "up (u)             - Move up".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "up".to_string()),
        );

        // Down
        self.register_command(
            "down".to_string(),
            vec!["d".to_string()],
            "down (d)           - Move down".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "down".to_string()),
        );

        // Northeast
        self.register_command(
            "northeast".to_string(),
            vec!["ne".to_string()],
            "northeast (ne)     - Move northeast".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "northeast".to_string()),
        );

        // Northwest
        self.register_command(
            "northwest".to_string(),
            vec!["nw".to_string()],
            "northwest (nw)     - Move northwest".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "northwest".to_string()),
        );

        // Southeast
        self.register_command(
            "southeast".to_string(),
            vec!["se".to_string()],
            "southeast (se)     - Move southeast".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "southeast".to_string()),
        );

        // Southwest
        self.register_command(
            "southwest".to_string(),
            vec!["sw".to_string()],
            "southwest (sw)     - Move southwest".to_string(),
            |context, entity, _cmd, _args| Self::attempt_move(context, entity, "southwest".to_string()),
        );
    }

    /// Attempt to move an entity in a direction
    /// TODO: Handle Walk, Run, Crawl, and Fly
    async fn attempt_move(context: Arc<WorldContext>, entity: EcsEntity, direction: String) -> CommandResult {
        use crate::ecs::components::{Exits, ExitData, EntityUuid, Room};

        // Normalize direction
        let normalized_direction = Self::normalize_direction(&direction);
        if normalized_direction.is_none() {
            return CommandResult::Invalid(format!("'{}' is not a valid direction", direction));
        }
        let normalized_direction = normalized_direction.unwrap();

        // Get current location and exits - scope the borrow
        let (current_loc, exit_data) = {
            let world = context.entities().read().await;
            let current_loc = match world.get::<&Location>(entity) {
                Ok(loc) => *loc,
                Err(_) => return CommandResult::Failure("You have no location".to_string()),
            };

            // Find the room entity by UUID (since EntityId.entity may not be resolved)
            let room_uuid = current_loc.room_id.uuid();
            let mut room_entity_opt = None;

            for (room_entity, _room_comp) in world.query::<&Room>().iter() {
                if let Ok(entity_uuid) = world.get::<&EntityUuid>(room_entity) {
                    if entity_uuid.0 == room_uuid {
                        room_entity_opt = Some(room_entity);
                        break;
                    }
                }
            }

            let room_entity = match room_entity_opt {
                Some(e) => e,
                None => {
                    tracing::warn!("Room with UUID {} not found in world", room_uuid);
                    return CommandResult::Failure(format!(
                        "You try to go {}, but you are in an invalid location.",
                        direction
                    ));
                }
            };

            // Get the room's exits
            let exits = match world.get::<&Exits>(room_entity) {
                Ok(exits) => exits,
                Err(_) => {
                    tracing::warn!("Room {:?} (UUID: {}) has no Exits component", room_entity, room_uuid);
                    return CommandResult::Failure(format!(
                        "You try to go {}, but there are no exits here.",
                        direction
                    ));
                }
            };

            // Find the exit in the requested direction and clone it
            let exit_data = match exits.find_exit(&normalized_direction) {
                Some(exit) => exit.clone(),
                None => {
                    return CommandResult::Failure(format!(
                        "You try to go {}, but there is no exit in that direction.",
                        direction
                    ));
                }
            };

            (current_loc, exit_data)
        }; // world lock is released here

        // Check if exit is blocked
        if exit_data.closeable && exit_data.closed {
            return CommandResult::Failure(format!(
                "You try to go {}, but the door is closed.",
                direction
            ));
        }

        if exit_data.lockable && exit_data.locked {
            return CommandResult::Failure(format!(
                "You try to go {}, but the door is locked.",
                direction
            ));
        }

        // Perform the movement
        let new_location = Location::new(current_loc.area_id, exit_data.dest_id);

        {
            let mut world = context.entities().write().await;
            if let Ok(mut location) = world.get::<&mut Location>(entity) {
                *location = new_location;
            } else {
                return CommandResult::Failure("Failed to move".to_string());
            }
        } // world write lock is released here

        // Return success with new room description
        // Call the look command to show the new room
        look::look_command(context, entity, "look".to_string(), vec![]).await
    }

    /// Normalize direction strings to full direction names
    fn normalize_direction(direction: &str) -> Option<String> {
        match direction.to_lowercase().as_str() {
            "north" | "n" => Some("North".to_string()),
            "south" | "s" => Some("South".to_string()),
            "east" | "e" => Some("East".to_string()),
            "west" | "w" => Some("West".to_string()),
            "up" | "u" => Some("Up".to_string()),
            "down" | "d" => Some("Down".to_string()),
            "northeast" | "ne" => Some("Northeast".to_string()),
            "northwest" | "nw" => Some("Northwest".to_string()),
            "southeast" | "se" => Some("Southeast".to_string()),
            "southwest" | "sw" => Some("Southwest".to_string()),
            _ => None,
        }
    }

    /// Update the command system, processing queued commands
    pub fn update(&mut self, context: Arc<WorldContext>) {
        let mut commands_to_execute = Vec::new();

        // Collect commands from all commandable entities
        {
            let world_result = context.entities().try_write();
            if let Ok(mut world) = world_result {
                for (_entity, commandable) in world.query_mut::<&mut crate::ecs::components::Commandable>() {
                    if let Some(cmd) = commandable.next_command() {
                        commands_to_execute.push((_entity, cmd.command, cmd.args));
                    }
                }
            }
        }

        // Execute collected commands
        for (entity, command, args) in commands_to_execute {
            self.execute(context.clone(), entity, &command, &args);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::{
        BodyAttributes, Commandable, Container, Description, MindAttributes, Name, Room,
        SoulAttributes, EntityId,
    };

    // Helper function to create a test context with a mock world
    fn create_test_context() -> Arc<WorldContext> {
        // Create an in-memory mock for testing without requiring a database
        // Note: This won't have full persistence functionality, but is sufficient for command tests
        use crate::persistence::PersistenceManager;

        // We can't create a real persistence manager without a database,
        // so tests that need context should be integration tests
        // For unit tests, we'll need to refactor to work without context
        unimplemented!("Use integration tests for context-dependent tests")
    }

    #[test]
    fn test_command_system_creation() {
        let event_bus = EventBus::new();
        let _system = CommandSystem::new(event_bus);
    }

    #[test]
    fn test_look_command() {
        // TODO: Fix test

        // let mut world = GameWorld::new();
        // let event_bus = EventBus::new();
        // let mut system = CommandSystem::new(event_bus);

        // let entity = world.spawn((Name::new("Player"), Location::new(1, 1)));

        // let result = system.execute(&mut world, entity, "look", &[]);
        // assert!(matches!(result, CommandResult::Success(_)));
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_inventory_command() {
        // TODO: Convert to integration test with proper WorldEngineContext
        // let context = create_test_context();
        // let mut world = context.entities().blocking_write();
        // let event_bus = EventBus::new();
        // let mut system = CommandSystem::new(event_bus);
        //
        // let entity = world.spawn((Name::new("Player"), Container::new(Some(10))));
        // drop(world);
        //
        // let result = system.execute(context, entity, "inventory", &[]);
        // assert!(matches!(result, CommandResult::Success(_)));
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_say_command() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_invalid_command() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    fn test_command_aliases() {
        // TODO: Fix Test
        // let mut world = GameWorld::new();
        // let event_bus = EventBus::new();
        // let mut system = CommandSystem::new(event_bus);

        // let entity = world.spawn((Name::new("Player"), Location::new(area_id, room_id)));

        // Test alias 'l' for 'look'
        // let result = system.execute(&mut world, entity, "l", &[]);
        // assert!(matches!(result, CommandResult::Success(_)));
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_score_command() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_movement_commands() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_enhanced_look_command() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_look_at_target() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_help_command_dynamic_generation() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_subcommand_handling() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_subcommand_vs_regular_command() {
        // TODO: Convert to integration test with proper WorldEngineContext
    }
}



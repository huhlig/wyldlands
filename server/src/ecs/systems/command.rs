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

//! Command system for processing player and NPC commands

mod admin;
mod combat;
mod comms;
mod exit;
mod help;
mod inventory;
mod llm_generate;
mod look;
mod npc;
mod query;
mod score;

use crate::account::AccountRole;
use crate::ecs::EcsEntity;
use crate::ecs::components::{Avatar, Combatant, Commandable, Location};
use crate::ecs::context::WorldContext;
use crate::ecs::events::{EventBus, GameEvent};
use hecs::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub type CommandFn = Box<
    dyn Fn(
            Arc<WorldContext>,
            EcsEntity,
            String,
            Vec<String>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = CommandResult> + Send + 'static>>
        + Send
        + Sync,
>;

#[derive(Debug, Clone)]
pub enum CommandResult {
    Success(String),
    Failure(String),
    Invalid(String),
}

/// Information about an available command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommand {
    /// Primary command name
    pub name: String,
    /// Alternative names for the command
    pub aliases: Vec<String>,
    /// Short documentation describing what the command does
    pub description: String,
}

struct CommandMetadata {
    handler: CommandFn,
    help_text: String,
    aliases: Vec<String>,
    required_role: Option<AccountRole>,
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

    /// Register a command with aliases, help text, and optional role requirement
    pub fn register_command<F, Fut>(
        &mut self,
        name: String,
        aliases: Vec<String>,
        help_text: String,
        handler: F,
    ) where
        F: Fn(Arc<WorldContext>, EcsEntity, String, Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CommandResult> + Send + 'static,
    {
        self.register_command_with_role(name, aliases, help_text, None, handler)
    }

    /// Register a command with role requirement
    pub fn register_command_with_role<F, Fut>(
        &mut self,
        name: String,
        aliases: Vec<String>,
        help_text: String,
        required_role: Option<AccountRole>,
        handler: F,
    ) where
        F: Fn(Arc<WorldContext>, EcsEntity, String, Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CommandResult> + Send + 'static,
    {
        let handler = Box::new(
            move |ctx: Arc<WorldContext>, entity: EcsEntity, cmd: String, args: Vec<String>| {
                Box::pin(handler(ctx, entity, cmd, args))
                    as std::pin::Pin<
                        Box<dyn std::future::Future<Output = CommandResult> + Send + 'static>,
                    >
            },
        );
        let metadata = CommandMetadata {
            handler,
            help_text,
            aliases: aliases.clone(),
            required_role,
        };
        self.commands.insert(name.clone(), metadata);
        for alias in aliases {
            self.aliases.insert(alias, name.clone());
        }
    }

    /// Check if entity has required role for a command
    async fn check_role_permission(
        &self,
        context: Arc<WorldContext>,
        entity: EcsEntity,
        required_role: AccountRole,
    ) -> Result<(), String> {
        // Get the avatar component to find account_id
        let account_id = {
            let world = context.entities().read().await;
            match world.get::<&Avatar>(entity) {
                Ok(avatar) => avatar.account_id,
                Err(_) => {
                    return Err("You must be logged in to use this command".to_string());
                }
            }
        };

        // Get account from database to check role
        let account = context
            .persistence()
            .get_account_by_id(account_id)
            .await
            .map_err(|e| format!("Failed to verify permissions: {}", e))?;

        // Check if account has required role
        if !account.role.has_permission(required_role) {
            return Err(format!(
                "You need {} role or higher to use this command",
                required_role
            ));
        }

        Ok(())
    }

    /// Generate help text from registered commands, filtered by user's role
    pub async fn generate_help(&self, context: Arc<WorldContext>, entity: EcsEntity) -> String {
        // Get user's role
        let user_role = self
            .get_user_role(context.clone(), entity)
            .await
            .unwrap_or(AccountRole::Player);

        let mut regular_commands = Vec::new();
        let mut storyteller_commands = Vec::new();
        let mut builder_commands = Vec::new();
        let mut admin_commands = Vec::new();
        let mut movement_commands = Vec::new();

        for (name, metadata) in &self.commands {
            // Skip commands that require a higher role than the user has
            if let Some(required_role) = metadata.required_role {
                if !user_role.has_permission(required_role) {
                    continue;
                }
            }

            // Categorize commands
            if name.starts_with("world ") {
                admin_commands.push(&metadata.help_text);
            } else if [
                "north",
                "south",
                "east",
                "west",
                "up",
                "down",
                "northeast",
                "northwest",
                "southeast",
                "southwest",
            ]
            .contains(&name.as_str())
            {
                movement_commands.push(&metadata.help_text);
            } else if let Some(required_role) = metadata.required_role {
                match required_role {
                    AccountRole::Admin => admin_commands.push(&metadata.help_text),
                    AccountRole::Builder => builder_commands.push(&metadata.help_text),
                    AccountRole::Storyteller => storyteller_commands.push(&metadata.help_text),
                    AccountRole::Player => regular_commands.push(&metadata.help_text),
                }
            } else {
                regular_commands.push(&metadata.help_text);
            }
        }

        // Sort each category
        regular_commands.sort();
        storyteller_commands.sort();
        builder_commands.sort();
        admin_commands.sort();
        movement_commands.sort();

        let mut help = String::from("\r\nAvailable Commands:\r\n");
        for cmd in regular_commands {
            help.push_str("  ");
            help.push_str(cmd);
            help.push('\r');
            help.push('\n');
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

        if !storyteller_commands.is_empty() {
            help.push_str("\r\nStoryteller Commands:\r\n");
            for cmd in storyteller_commands {
                help.push_str("  ");
                help.push_str(cmd);
                help.push('\r');
                help.push('\n');
            }
        }

        if !builder_commands.is_empty() {
            help.push_str("\r\nBuilder Commands:\r\n");
            for cmd in builder_commands {
                help.push_str("  ");
                help.push_str(cmd);
                help.push('\r');
                help.push('\n');
            }
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

        help
    }

    /// Get the user's role from their account
    async fn get_user_role(
        &self,
        context: Arc<WorldContext>,
        entity: EcsEntity,
    ) -> Result<AccountRole, String> {
        // Get the avatar component to find account_id
        let account_id = {
            let world = context.entities().read().await;
            match world.get::<&Avatar>(entity) {
                Ok(avatar) => avatar.account_id,
                Err(_) => {
                    return Ok(AccountRole::Player); // Default to player if not logged in
                }
            }
        };

        // Get account from database to check role
        let account = context
            .persistence()
            .get_account_by_id(account_id)
            .await
            .map_err(|e| format!("Failed to get account: {}", e))?;

        Ok(account.role)
    }

    /// Get a list of available commands for an avatar based on their current state
    ///
    /// This function examines the avatar's:
    /// - Account role (Player, Storyteller, Builder, Admin)
    /// - Combat state (in combat or not)
    ///
    /// Returns a vector of AvailableCommand structs with name, aliases, and description
    pub async fn get_available_commands(
        &self,
        context: Arc<WorldContext>,
        entity: EcsEntity,
    ) -> Vec<AvailableCommand> {
        // Get user's role
        let user_role = self
            .get_user_role(context.clone(), entity)
            .await
            .unwrap_or(AccountRole::Player);

        // Check if entity is in combat
        // TODO: Future enhancement - filter commands based on combat state
        // For example, hide movement commands during combat, or show combat-specific commands
        let _in_combat = {
            let world = context.entities().read().await;
            world
                .get::<&Combatant>(entity)
                .map(|c| c.in_combat)
                .unwrap_or(false)
        };

        let mut available_commands = Vec::new();

        for (name, metadata) in &self.commands {
            // TODO: Future enhancement - filter based on other state conditions:
            // - Location-specific commands (e.g., "swim" only near water)
            // - Item-specific commands (e.g., "read" only with readable items)
            // - Quest-specific commands
            // - Time-of-day specific commands

            // Skip commands that require a higher role than the user has
            if let Some(required_role) = metadata.required_role {
                if !user_role.has_permission(required_role) {
                    continue;
                }
            }

            // Extract description from help_text (first line before any formatting)
            let description = metadata
                .help_text
                .lines()
                .next()
                .unwrap_or(&metadata.help_text)
                .trim()
                .to_string();

            available_commands.push(AvailableCommand {
                name: name.clone(),
                aliases: metadata.aliases.clone(),
                description,
            });
        }

        // Sort by command name for consistent output
        available_commands.sort_by(|a, b| a.name.cmp(&b.name));

        available_commands
    }

    /// Execute a command
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

        // Special handling for help command - delegate to help system
        if cmd_name == "help" {
            if args.is_empty() {
                // Basic help
                return help::help_command(
                    context.clone(),
                    entity,
                    cmd_name.clone(),
                    args.to_vec(),
                )
                .await;
            } else if args[0].to_lowercase() == "commands" {
                // List all commands - use dynamic generation from registered commands filtered by role
                return CommandResult::Success(self.generate_help(context.clone(), entity).await);
            } else {
                // Help for specific keyword - use database
                return help::help_keyword_command(
                    context.clone(),
                    entity,
                    cmd_name.clone(),
                    args.to_vec(),
                )
                .await;
            }
        }

        // First try exact match
        if let Some(metadata) = self.commands.get(&cmd_name) {
            // Check role permission if required
            if let Some(required_role) = metadata.required_role {
                if let Err(err) = self
                    .check_role_permission(context.clone(), entity, required_role)
                    .await
                {
                    return CommandResult::Failure(err);
                }
            }

            let result =
                (metadata.handler)(context.clone(), entity, cmd_name.clone(), args.to_vec()).await;

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
            let subcommand_name = self
                .aliases
                .get(&subcommand_name)
                .unwrap_or(&subcommand_name)
                .clone();

            if let Some(metadata) = self.commands.get(&subcommand_name) {
                // Check role permission if required
                if let Some(required_role) = metadata.required_role {
                    if let Err(err) = self
                        .check_role_permission(context.clone(), entity, required_role)
                        .await
                    {
                        return CommandResult::Failure(err);
                    }
                }

                // Pass remaining args (excluding the subcommand)
                let remaining_args = args[1..].to_vec();
                let result = (metadata.handler)(
                    context.clone(),
                    entity,
                    subcommand_name.clone(),
                    remaining_args,
                )
                .await;

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

        // Combat commands
        self.register_command(
            "attack".to_string(),
            vec!["kill".to_string(), "k".to_string()],
            "attack (kill, k) <target> - Attack a target".to_string(),
            |ctx, entity, _cmd, args| async move {
                match combat::handle_attack(ctx, entity, &args).await {
                    Ok(msg) => CommandResult::Success(msg),
                    Err(msg) => CommandResult::Failure(msg),
                }
            },
        );

        self.register_command(
            "defend".to_string(),
            vec!["def".to_string()],
            "defend (def)       - Take a defensive stance".to_string(),
            |ctx, entity, _cmd, args| async move {
                match combat::handle_defend(ctx, entity, &args).await {
                    Ok(msg) => CommandResult::Success(msg),
                    Err(msg) => CommandResult::Failure(msg),
                }
            },
        );

        self.register_command(
            "flee".to_string(),
            vec!["run".to_string()],
            "flee (run)         - Attempt to flee from combat".to_string(),
            |ctx, entity, _cmd, args| async move {
                match combat::handle_flee(ctx, entity, &args).await {
                    Ok(msg) => CommandResult::Success(msg),
                    Err(msg) => CommandResult::Failure(msg),
                }
            },
        );

        self.register_command(
            "combat".to_string(),
            vec!["c".to_string()],
            "combat (c)         - View combat status".to_string(),
            |ctx, entity, _cmd, args| async move {
                match combat::handle_combat_status(ctx, entity, &args).await {
                    Ok(msg) => CommandResult::Success(msg),
                    Err(msg) => CommandResult::Failure(msg),
                }
            },
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
        self.register_command_with_role(
            "world inspect".to_string(),
            vec![
                "winspect".to_string(),
                "query".to_string(),
                "inspect".to_string(),
            ],
            "world inspect (winspect) - Query all components of an entity by UUID".to_string(),
            Some(AccountRole::Admin),
            |ctx, entity, cmd, args| admin::query_entity_command(ctx, entity, cmd, args),
        );

        // World list command (admin)
        self.register_command_with_role(
            "world list".to_string(),
            vec![
                "wlist".to_string(),
                "entities".to_string(),
                "list".to_string(),
            ],
            "world list (wlist)     - List all entities with their UUIDs and components"
                .to_string(),
            Some(AccountRole::Admin),
            |ctx, entity, cmd, args| admin::list_entities_command(ctx, entity, cmd, args),
        );

        // World save command (admin)
        self.register_command_with_role(
            "world save".to_string(),
            vec!["wsave".to_string()],
            "world save (wsave)     - Save all persistent entities to the database".to_string(),
            Some(AccountRole::Admin),
            |ctx, entity, cmd, args| admin::world_save_command(ctx, entity, cmd, args),
        );

        // World reload command (admin)
        self.register_command_with_role(
            "world reload".to_string(),
            vec!["wreload".to_string()],
            "world reload (wreload) - Clear ECS and reload entities from database".to_string(),
            Some(AccountRole::Admin),
            |ctx, entity, cmd, args| admin::world_reload_command(ctx, entity, cmd, args),
        );

        // Area commands (builder)
        self.register_command_with_role(
            "area create".to_string(),
            vec!["acreate".to_string()],
            "area create (acreate) <name> - Create a new area".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::area_create_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "area list".to_string(),
            vec!["alist".to_string()],
            "area list (alist) [filter] - List all areas".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::area_list_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "area edit".to_string(),
            vec!["aedit".to_string()],
            "area edit (aedit) <uuid> <property> <value> - Edit area properties".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::area_edit_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "area delete".to_string(),
            vec!["adelete".to_string()],
            "area delete (adelete) <uuid> - Delete an area (if empty)".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::area_delete_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "area info".to_string(),
            vec!["ainfo".to_string()],
            "area info (ainfo) <uuid> - Show detailed area information".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::area_info_command(ctx, entity, cmd, args),
        );

        // Room commands (builder)
        self.register_command_with_role(
            "room create".to_string(),
            vec!["rcreate".to_string()],
            "room create (rcreate) <area-uuid> <name> - Create a new room".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::room_create_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "room list".to_string(),
            vec!["rlist".to_string()],
            "room list (rlist) [area-uuid] - List rooms in area or all rooms".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::room_list_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "room goto".to_string(),
            vec!["rgoto".to_string(), "goto".to_string()],
            "room goto (rgoto, goto) <uuid> - Teleport to a room".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::room_goto_command(ctx, entity, cmd, args),
        );

        // Exit commands (builder)
        self.register_command_with_role(
            "exit add".to_string(),
            vec!["exitadd".to_string()],
            "exit add (exitadd) <direction> <dest-uuid> - Add exit from current room".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::exit_add_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "exit remove".to_string(),
            vec!["exitremove".to_string()],
            "exit remove (exitremove) <direction> - Remove exit from current room".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::exit_remove_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "exit list".to_string(),
            vec!["exitlist".to_string(), "exits".to_string()],
            "exit list (exitlist, exits) - List exits from current room".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::exit_list_command(ctx, entity, cmd, args),
        );

        // Dig command (builder)
        self.register_command_with_role(
            "dig".to_string(),
            vec![],
            "dig <direction> <name> [oneway] [area <uuid>] - Create and connect new room"
                .to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| admin::dig_command(ctx, entity, cmd, args),
        );

        // Phase 2: Advanced room/exit commands (builder)
        self.register_command_with_role(
            "room edit".to_string(),
            vec!["redit".to_string()],
            "room edit (redit) <uuid> <field> <value> - Edit room properties".to_string(),
            Some(AccountRole::Builder),
            admin::room_edit_command,
        );

        self.register_command_with_role(
            "room deleteall".to_string(),
            vec!["rdeleteall".to_string()],
            "room deleteall (rdeleteall) <area-uuid> - Delete all rooms in an area".to_string(),
            Some(AccountRole::Builder),
            admin::room_delete_bulk_command,
        );

        self.register_command_with_role(
            "exit edit".to_string(),
            vec!["exitedit".to_string()],
            "exit edit (exitedit) <direction> <property> <value> - Edit exit properties"
                .to_string(),
            Some(AccountRole::Builder),
            admin::exit_edit_command,
        );

        self.register_command_with_role(
            "area search".to_string(),
            vec!["asearch".to_string()],
            "area search (asearch) <query> - Search for areas by name".to_string(),
            Some(AccountRole::Builder),
            admin::area_search_command,
        );

        self.register_command_with_role(
            "room search".to_string(),
            vec!["rsearch".to_string()],
            "room search (rsearch) <query> [area-uuid] - Search for rooms by name".to_string(),
            Some(AccountRole::Builder),
            admin::room_search_command,
        );

        // Phase 3: Item/Object editor commands (builder)
        self.register_command_with_role(
            "item create".to_string(),
            vec!["icreate".to_string()],
            "item create (icreate) <name> - Create a new item in current room".to_string(),
            Some(AccountRole::Builder),
            admin::item_create_command,
        );

        self.register_command_with_role(
            "item edit".to_string(),
            vec!["iedit".to_string()],
            "item edit (iedit) <uuid> <field> <value> - Edit item properties".to_string(),
            Some(AccountRole::Builder),
            admin::item_edit_command,
        );

        self.register_command_with_role(
            "item clone".to_string(),
            vec!["iclone".to_string(), "icopy".to_string()],
            "item clone (iclone, icopy) <uuid> [new-name] - Clone an existing item".to_string(),
            Some(AccountRole::Builder),
            admin::item_clone_command,
        );

        self.register_command_with_role(
            "item list".to_string(),
            vec!["ilist".to_string(), "items".to_string()],
            "item list (ilist, items) [query] - List items in current room".to_string(),
            Some(AccountRole::Builder),
            admin::item_list_command,
        );

        // NPC commands (storyteller)
        self.register_command_with_role(
            "npc create".to_string(),
            vec!["ncreate".to_string()],
            "npc create (ncreate) <name> [template] - Create a new NPC".to_string(),
            Some(AccountRole::Storyteller),
            |ctx, entity, cmd, args| npc::npc_create_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "npc list".to_string(),
            vec!["nlist".to_string()],
            "npc list (nlist) [filter] - List all NPCs".to_string(),
            Some(AccountRole::Storyteller),
            |ctx, entity, cmd, args| npc::npc_list_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "npc edit".to_string(),
            vec!["nedit".to_string()],
            "npc edit (nedit) <uuid> <property> <value> - Edit NPC properties".to_string(),
            Some(AccountRole::Storyteller),
            |ctx, entity, cmd, args| npc::npc_edit_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "npc dialogue".to_string(),
            vec!["ndialogue".to_string()],
            "npc dialogue (ndialogue) <uuid> <property> <value> - Configure NPC dialogue"
                .to_string(),
            Some(AccountRole::Storyteller),
            |ctx, entity, cmd, args| npc::npc_dialogue_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "npc goap".to_string(),
            vec!["ngoap".to_string()],
            "npc goap (ngoap) <uuid> <subcommand> [args] - Configure NPC GOAP AI".to_string(),
            Some(AccountRole::Storyteller),
            |ctx, entity, cmd, args| npc::npc_goap_command(ctx, entity, cmd, args),
        );

        // LLM Generation commands (builder)
        self.register_command_with_role(
            "room generate".to_string(),
            vec!["rgen".to_string()],
            "room generate (rgen) <prompt> - Generate room description using LLM".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| llm_generate::room_generate_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "item generate".to_string(),
            vec!["igen".to_string()],
            "item generate (igen) <prompt> - Generate item details using LLM".to_string(),
            Some(AccountRole::Builder),
            |ctx, entity, cmd, args| llm_generate::item_generate_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "npc generate".to_string(),
            vec!["ngen".to_string()],
            "npc generate (ngen) <prompt> - Generate NPC profile using LLM".to_string(),
            Some(AccountRole::Storyteller),
            |ctx, entity, cmd, args| llm_generate::npc_generate_command(ctx, entity, cmd, args),
        );

        self.register_command_with_role(
            "item info".to_string(),
            vec!["iinfo".to_string()],
            "item info (iinfo) <uuid> - Show detailed item information".to_string(),
            Some(AccountRole::Builder),
            admin::item_info_command,
        );

        // Item template commands (builder)
        self.register_command_with_role(
            "item spawn".to_string(),
            vec!["ispawn".to_string()],
            "item spawn (ispawn) <template> [quantity] - Spawn item(s) from template".to_string(),
            Some(AccountRole::Builder),
            admin::item_spawn_command,
        );
        self.register_command_with_role(
            "item templates".to_string(),
            vec!["itemplates".to_string()],
            "item templates (itemplates) [filter] - List available item templates".to_string(),
            Some(AccountRole::Builder),
            admin::item_templates_command,
        );

        // Help command - uses database-driven help system
        self.register_command(
            "help".to_string(),
            vec!["?".to_string()],
            "help (?) [keyword|commands] - Show help information".to_string(),
            |_context, _entity, _cmd, _args| async {
                // This handler is never called - help is handled specially in execute()
                CommandResult::Success(String::new())
            },
        );

        // Movement commands
        self.register_movement_commands();
    }

    /// Register all movement commands
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
            |context, entity, _cmd, _args| {
                Self::attempt_move(context, entity, "northeast".to_string())
            },
        );

        // Northwest
        self.register_command(
            "northwest".to_string(),
            vec!["nw".to_string()],
            "northwest (nw)     - Move northwest".to_string(),
            |context, entity, _cmd, _args| {
                Self::attempt_move(context, entity, "northwest".to_string())
            },
        );

        // Southeast
        self.register_command(
            "southeast".to_string(),
            vec!["se".to_string()],
            "southeast (se)     - Move southeast".to_string(),
            |context, entity, _cmd, _args| {
                Self::attempt_move(context, entity, "southeast".to_string())
            },
        );

        // Southwest
        self.register_command(
            "southwest".to_string(),
            vec!["sw".to_string()],
            "southwest (sw)     - Move southwest".to_string(),
            |context, entity, _cmd, _args| {
                Self::attempt_move(context, entity, "southwest".to_string())
            },
        );
    }

    /// Attempt to move an entity in a direction
    /// TODO: Handle Walk, Run, Crawl, and Fly
    async fn attempt_move(
        context: Arc<WorldContext>,
        entity: EcsEntity,
        direction: String,
    ) -> CommandResult {
        use crate::ecs::components::{EntityUuid, Exits, Room};

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

            for (room_entity, _room_comp) in world.query::<(Entity, &Room)>().iter() {
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
                    tracing::warn!(
                        "Room {:?} (UUID: {}) has no Exits component",
                        room_entity,
                        room_uuid
                    );
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
            let world = context.entities().read().await;
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
    pub async fn update(&mut self, context: Arc<WorldContext>) {
        let mut commands_to_execute = Vec::new();

        // Collect commands from all commandable entities
        {
            let world_result = context.entities().try_write();
            if let Ok(mut world) = world_result {
                for (entity, commandable) in world.query_mut::<(Entity, &mut Commandable)>() {
                    if let Some(cmd) = commandable.next_command() {
                        commands_to_execute.push((entity, cmd.command, cmd.args));
                    }
                }
            }
        }

        // Execute collected commands
        for (entity, command, args) in commands_to_execute {
            self.execute(context.clone(), entity, &command, &args).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_system_creation() {
        let event_bus = EventBus::new();
        let _system = CommandSystem::new(event_bus);
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
    fn test_subcommand_vs_regular_command() {}

    #[test]
    fn test_available_command_struct() {
        // Test that AvailableCommand can be created and serialized
        let cmd = AvailableCommand {
            name: "look".to_string(),
            aliases: vec!["l".to_string()],
            description: "Look around the room".to_string(),
        };

        assert_eq!(cmd.name, "look");
        assert_eq!(cmd.aliases.len(), 1);
        assert!(cmd.description.contains("Look"));

        // Test serialization
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("look"));
        assert!(json.contains("description"));
    }

    #[test]
    #[ignore = "Requires WorldEngineContext - convert to integration test"]
    fn test_get_available_commands() {
        // TODO: Convert to integration test with proper WorldEngineContext
        // This test should verify:
        // 1. Commands are filtered by user role
        // 2. Commands include name, aliases, and description
        // 3. Commands are sorted alphabetically
        // 4. Combat state affects available commands (future enhancement)
        // TODO: Convert to integration test with proper WorldEngineContext
    }
}

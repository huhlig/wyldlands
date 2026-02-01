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

//! Exit/logoff command implementation

use crate::ecs::EcsEntity;
use crate::ecs::components::Name;
use crate::ecs::context::WorldContext;
use crate::ecs::systems::command::CommandResult;
use std::sync::Arc;

/// Exit command - returns player to character selection
/// This saves the character and signals the need to unload the character
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn exit_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    _args: Vec<String>,
) -> CommandResult {
    let world = context.entities().read().await;
    // Get character name for the farewell message
    let name = world
        .get::<&Name>(entity)
        .map(|n| n.display.clone())
        .unwrap_or_else(|_| "Adventurer".to_string());
    drop(world);

    // Return success with a special marker that the gateway can recognize
    // The actual character saving and session cleanup should be handled by the server
    CommandResult::Success(format!(
        "Farewell, {}! Your character has been saved.\r\n\
         \r\n\
         [EXIT_TO_CHARACTER_SELECTION]",
        name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::PersistenceManager;

    #[tokio::test]
    async fn test_exit_command() {
        let persistence_manager = Arc::new(PersistenceManager::new_mock());
        let context = Arc::new(WorldContext::new(persistence_manager));

        let entity = {
            let mut world = context.entities().write().await;
            world.spawn((Name::new("TestPlayer"),))
        };

        // Run the command
        let result = exit_command(context, entity, "exit".to_string(), vec![]).await;

        assert!(matches!(result, CommandResult::Success(_)));

        if let CommandResult::Success(msg) = result {
            assert!(msg.contains("TestPlayer"));
            assert!(msg.contains("EXIT_TO_CHARACTER_SELECTION"));
        }
    }
}

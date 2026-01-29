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

use crate::ecs::components::Container;
use crate::ecs::context::WorldContext;
use crate::ecs::systems::CommandResult;
use crate::ecs::EcsEntity;
use std::sync::Arc;

/// Command to get list of inventory items
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn inventory_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    _args: Vec<String>,
) -> CommandResult {
    let world = context.entities().read().await;
    let result = if let Ok(container) = world.get::<&Container>(entity) {
        // Container no longer tracks contents directly
        // TODO: Query world for entities with this entity as parent
        CommandResult::Success(format!(
            "Inventory capacity: {}, max weight: {:.1}",
            container
                .capacity
                .map(|c| c.to_string())
                .unwrap_or("unlimited".to_string()),
            container.max_weight.unwrap_or(f32::INFINITY)
        ))
    } else {
        CommandResult::Failure("You have no inventory".to_string())
    };
    drop(world);
    result
}

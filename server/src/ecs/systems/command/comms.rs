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

use crate::ecs::EcsEntity;
use crate::ecs::components::Name;
use crate::ecs::context::WorldContext;
use crate::ecs::systems::CommandResult;
use std::sync::Arc;

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn say_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Invalid("Say what?".to_string());
    }

    // TODO: Say to other people as well

    let message = args.join(" ");
    let world = context.entities().read().await;
    let result = if let Ok(_name) = world.get::<&Name>(entity) {
        CommandResult::Success(format!("You say: '{}'", message))
    } else {
        CommandResult::Failure("You cannot speak".to_string())
    };
    drop(world);
    result
}

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn yell_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Invalid("Yell what?".to_string());
    }

    // TODO: Yell to other people in same and nearby rooms

    let message = args.join(" ");
    let world = context.entities().read().await;
    let result = if let Ok(_name) = world.get::<&Name>(entity) {
        CommandResult::Success(format!("You say: '{}'", message))
    } else {
        CommandResult::Failure("You cannot speak".to_string())
    };
    drop(world);
    result
}

#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn emote_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    args: Vec<String>,
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Invalid("Emote what?".to_string());
    }

    let action = args.join(" ");
    let world = context.entities().read().await;
    let result = if let Ok(name) = world.get::<&Name>(entity) {
        CommandResult::Success(format!("{} {}", name.display, action))
    } else {
        CommandResult::Failure("You cannot emote".to_string())
    };
    drop(world);
    result
}

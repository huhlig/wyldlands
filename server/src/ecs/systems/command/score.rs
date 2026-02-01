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
use crate::ecs::components::{AttributeScores, Name, Skills};
use crate::ecs::context::WorldContext;
use crate::ecs::systems::CommandResult;
use std::sync::Arc;

/// TODO: Rewrite Score/Sheet Command to resemble the character builder
#[tracing::instrument(skip(context), fields(entity_id = entity.id()))]
pub async fn score_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _cmd: String,
    _args: Vec<String>,
) -> CommandResult {
    let world = context.entities().read().await;
    let mut output = String::new();

    if let Ok(name) = world.get::<&Name>(entity) {
        output.push_str(&format!("Name: {}\r\n", name.display));
    }

    if let Ok(body) = world.get::<&AttributeScores>(entity) {
        output.push_str(&format!(
            "Health: {:.1}/{:.1} ({:.0}%)\r\n",
            body.health_current,
            body.health_maximum,
            (body.health_current / body.health_maximum) * 100.0
        ));
        output.push_str(&format!(
            "Energy: {:.1}/{:.1}\r\n",
            body.energy_current, body.energy_maximum
        ));
        output.push_str(&format!(
            "Offence: {}, Finesse: {}, Defence: {}\r\n",
            body.score_offence, body.score_finesse, body.score_defence
        ));
    }

    if let Ok(skills) = world.get::<&Skills>(entity) {
        output.push_str(&format!("Skills: {} learned\r\n", skills.len()));
    }

    let result = if output.is_empty() {
        CommandResult::Failure("No stats available".to_string())
    } else {
        CommandResult::Success(output)
    };
    drop(world);
    result
}

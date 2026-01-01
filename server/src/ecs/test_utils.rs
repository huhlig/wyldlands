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

//! Test utilities for ECS testing

use crate::ecs::{GameWorld, EcsEntity};
use crate::ecs::components::{Name, Location, EntityId};

/// Create a test world
pub fn create_test_world() -> GameWorld {
    GameWorld::new()
}

/// Spawn a test entity with basic components
pub fn spawn_test_entity(world: &mut GameWorld) -> EcsEntity {
    world.spawn((
        Name::new("Test Entity"),
        Location::new(
            EntityId::from_uuid(uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap()),
            EntityId::from_uuid(uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap()),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_world() {
        let _world = create_test_world();
    }
    
    #[test]
    fn test_spawn_entity() {
        let mut world = create_test_world();
        let entity = spawn_test_entity(&mut world);
        
        assert!(world.get::<&Name>(entity).is_ok());
        assert!(world.get::<&Location>(entity).is_ok());
    }
}



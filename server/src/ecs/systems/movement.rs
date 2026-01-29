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

//! Movement system for entity movement

use hecs::Entity;
use crate::ecs::components::{Commandable, EntityId, Exits, Location};
use crate::ecs::events::{EventBus, GameEvent};
use crate::ecs::{EcsEntity, GameWorld};

pub struct MovementSystem {
    event_bus: EventBus,
}

impl MovementSystem {
    /// Create a new movement system
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }

    /// Update the movement system, processing queued movement commands
    pub fn update(&mut self, world: &mut GameWorld, _delta_time: f32) {
        let mut movements = Vec::new();

        // Collect movement commands
        for (entity, commandable) in world.query_mut::<(Entity, &mut Commandable)>() {
            if let Some(cmd) = commandable.command_queue.first() {
                if cmd.command == "move" || Self::is_direction(&cmd.command) {
                    let direction = if cmd.command == "move" {
                        cmd.args.first().map(|s| s.as_str())
                    } else {
                        Some(cmd.command.as_str())
                    };

                    if let Some(dir) = direction {
                        movements.push((entity, dir.to_string()));
                    }
                }
            }
        }

        // Execute movements
        for (entity, direction) in movements {
            self.move_entity(world, entity, &direction);

            // Remove the processed command
            if let Ok(mut cmd) = world.get::<&mut Commandable>(entity) {
                cmd.next_command();
            }
        }
    }

    /// Move an entity in a direction
    fn move_entity(&mut self, world: &mut GameWorld, entity: EcsEntity, direction: &str) {
        // Normalize direction
        let normalized_direction = Self::normalize_direction(direction);
        if normalized_direction.is_none() {
            tracing::debug!("Invalid direction: {}", direction);
            return;
        }
        let normalized_direction = normalized_direction.unwrap();

        // Get entity's current location
        let current_location = match world.get::<&Location>(entity) {
            Ok(loc) => *loc,
            Err(_) => {
                tracing::warn!("Entity {:?} has no location component", entity);
                return;
            }
        };

        // Get the current room's exits
        let room_entity = current_location.room_id.entity();
        let exits = match world.get::<&Exits>(room_entity) {
            Ok(exits) => exits,
            Err(_) => {
                tracing::warn!(
                    "Room {:?} (UUID: {}) has no Exits component",
                    room_entity,
                    current_location.room_id.uuid()
                );
                return;
            }
        };

        // Find the exit in the requested direction
        let exit = match exits.find_exit(&normalized_direction) {
            Some(exit) => exit,
            None => {
                tracing::debug!(
                    "No exit found in direction '{}' from room {}",
                    normalized_direction,
                    current_location.room_id.uuid()
                );
                return;
            }
        };

        // Check if exit is blocked
        if exit.closeable && exit.closed {
            tracing::debug!("Exit {} is closed", normalized_direction);
            return;
        }

        if exit.lockable && exit.locked {
            tracing::debug!("Exit {} is locked", normalized_direction);
            return;
        }

        // Update entity location to destination room
        let new_location = Location::new(current_location.area_id, exit.dest_id);

        if let Ok(mut location) = world.get::<&mut Location>(entity) {
            *location = new_location;

            // Publish movement event
            self.event_bus.publish(GameEvent::EntityMoved {
                entity,
                from: (current_location.area_id.uuid(), current_location.room_id.uuid()),
                to: (new_location.area_id.uuid(), new_location.room_id.uuid()),
            });

            tracing::debug!(
                "Entity {:?} moved {} from room {} to room {}",
                entity,
                normalized_direction,
                current_location.room_id.uuid(),
                new_location.room_id.uuid()
            );
        }
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

    /// Teleport an entity to a specific location
    pub fn teleport(
        &mut self,
        world: &mut GameWorld,
        entity: EcsEntity,
        target: Location,
    ) -> Result<(), String> {
        if let Ok(mut loc) = world.get::<&mut Location>(entity) {
            let old_loc = *loc;
            *loc = target;

            self.event_bus.publish(GameEvent::EntityMoved {
                entity,
                from: (old_loc.area_id.uuid(), old_loc.room_id.uuid()),
                to: (target.area_id.uuid(), target.room_id.uuid()),
            });

            Ok(())
        } else {
            Err("Entity has no location component".to_string())
        }
    }

    /// Teleport an entity using EntityId (convenience method)
    ///
    /// This demonstrates using EntityId for operations that need both
    /// the ECS entity handle and UUID for logging/tracking purposes.
    pub fn teleport_by_id(
        &mut self,
        world: &mut GameWorld,
        entity_id: EntityId,
        target: Location,
    ) -> Result<(), String> {
        if let Ok(mut loc) = world.get::<&mut Location>(entity_id.entity()) {
            let old_loc = *loc;
            *loc = target;

            tracing::debug!(
                "Teleporting entity {} (UUID: {}) from {:?} to {:?}",
                entity_id.entity().id(),
                entity_id.uuid(),
                old_loc,
                target
            );

            self.event_bus.publish(GameEvent::EntityMoved {
                entity: entity_id.entity(),
                from: (old_loc.area_id.uuid(), old_loc.room_id.uuid()),
                to: (target.area_id.uuid(), target.room_id.uuid()),
            });

            Ok(())
        } else {
            Err(format!(
                "Entity {} (UUID: {}) has no location component",
                entity_id.entity().id(),
                entity_id.uuid()
            ))
        }
    }

    /// Get an entity's location by EntityId
    pub fn get_location_by_id(
        &self,
        world: &GameWorld,
        entity_id: EntityId,
    ) -> Result<Location, String> {
        world
            .get::<&Location>(entity_id.entity())
            .map(|loc| *loc)
            .map_err(|_| {
                format!(
                    "Entity {} (UUID: {}) has no location component",
                    entity_id.entity().id(),
                    entity_id.uuid()
                )
            })
    }

    /// Check if a string is a valid direction
    fn is_direction(s: &str) -> bool {
        matches!(
            s.to_lowercase().as_str(),
            "north"
                | "n"
                | "south"
                | "s"
                | "east"
                | "e"
                | "west"
                | "w"
                | "up"
                | "u"
                | "down"
                | "d"
                | "northeast"
                | "ne"
                | "northwest"
                | "nw"
                | "southeast"
                | "se"
                | "southwest"
                | "sw"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Name;

    #[test]
    fn test_movement_system_creation() {
        let event_bus = EventBus::new();
        let _system = MovementSystem::new(event_bus);
    }

    #[test]
    fn test_basic_movement() {
        use crate::ecs::components::{ExitData, Room};

        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = MovementSystem::new(event_bus.clone());

        let area_id = uuid::Uuid::new_v4();
        let room1_id = uuid::Uuid::new_v4();
        let room2_id = uuid::Uuid::new_v4();

        // Create room 1 with an exit to room 2
        let room1_entity = world.spawn((
            Name::new("Room 1"),
            Room::new(EntityId::from_uuid(area_id)),
            Exits::new().add_exit(ExitData::new("North", EntityId::from_uuid(room2_id))),
            crate::ecs::components::EntityUuid(room1_id),
        ));

        // Create room 2
        let _room2_entity = world.spawn((
            Name::new("Room 2"),
            Room::new(EntityId::from_uuid(area_id)),
            Exits::new(),
            crate::ecs::components::EntityUuid(room2_id),
        ));

        // Register the rooms in a temporary registry
        let mut registry = crate::ecs::registry::EntityRegistry::new();
        registry.register(room1_entity, room1_id);

        // Get the EntityId for room1
        let room1_entity_id = registry.get_entity_id(room1_entity).unwrap();

        // Create an entity in room 1
        let entity = world.spawn((
            Name::new("Test"),
            Location::new(EntityId::from_uuid(area_id), room1_entity_id),
            Commandable::new(),
        ));

        // Queue movement command
        {
            let mut cmd = world.get::<&mut Commandable>(entity).unwrap();
            cmd.queue_command("north".into(), vec![]);
        }

        system.update(&mut world, 0.1);

        // Verify the entity moved to room 2
        let loc = world.get::<&Location>(entity).unwrap();
        assert_eq!(loc.area_id.uuid(), area_id);
        assert_eq!(loc.room_id.uuid(), room2_id);
    }

    #[test]
    fn test_teleport() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = MovementSystem::new(event_bus);

        let old_area = uuid::Uuid::new_v4();
        let old_room = uuid::Uuid::new_v4();
        let new_area = uuid::Uuid::new_v4();
        let new_room = uuid::Uuid::new_v4();

        let entity = world.spawn((Name::new("Test"), Location::new(EntityId::from_uuid(old_area), EntityId::from_uuid(old_room))));

        let target = Location::new(EntityId::from_uuid(new_area), EntityId::from_uuid(new_room));
        assert!(system.teleport(&mut world, entity, target).is_ok());

        let loc = world.get::<&Location>(entity).unwrap();
        assert_eq!(loc.area_id.uuid(), new_area);
        assert_eq!(loc.room_id.uuid(), new_room);
    }

    #[test]
    fn test_direction_validation() {
        assert!(MovementSystem::is_direction("north"));
        assert!(MovementSystem::is_direction("n"));
        assert!(MovementSystem::is_direction("northeast"));
        assert!(!MovementSystem::is_direction("invalid"));
    }

    #[test]
    fn test_teleport_by_entity_id() {
        let mut world = GameWorld::new();
        let mut registry = crate::ecs::registry::EntityRegistry::new();
        let event_bus = EventBus::new();
        let mut system = MovementSystem::new(event_bus);

        let old_area = uuid::Uuid::new_v4();
        let old_room = uuid::Uuid::new_v4();
        let new_area = uuid::Uuid::new_v4();
        let new_room = uuid::Uuid::new_v4();
        let entity_uuid = uuid::Uuid::new_v4();

        let entity = world.spawn((
            Name::new("Test"),
            Location::new(EntityId::from_uuid(old_area), EntityId::from_uuid(old_room)),
            crate::ecs::components::EntityUuid(entity_uuid),
        ));

        registry.register(entity, entity_uuid).unwrap();
        let entity_id = registry.get_entity_id(entity).unwrap();

        let target = Location::new(EntityId::from_uuid(new_area), EntityId::from_uuid(new_room));
        assert!(system.teleport_by_id(&mut world, entity_id, target).is_ok());

        let loc = world.get::<&Location>(entity).unwrap();
        assert_eq!(loc.area_id.uuid(), new_area);
        assert_eq!(loc.room_id.uuid(), new_room);
    }

    #[test]
    fn test_get_location_by_id() {
        let mut world = GameWorld::new();
        let mut registry = crate::ecs::registry::EntityRegistry::new();
        let event_bus = EventBus::new();
        let system = MovementSystem::new(event_bus);

        let area = uuid::Uuid::new_v4();
        let room = uuid::Uuid::new_v4();
        let entity_uuid = uuid::Uuid::new_v4();

        let entity = world.spawn((
            Name::new("Test"),
            Location::new(EntityId::from_uuid(area), EntityId::from_uuid(room)),
            crate::ecs::components::EntityUuid(entity_uuid),
        ));

        registry.register(entity, entity_uuid).unwrap();
        let entity_id = registry.get_entity_id(entity).unwrap();

        let location = system.get_location_by_id(&world, entity_id).unwrap();
        assert_eq!(location.area_id.uuid(), area);
        assert_eq!(location.room_id.uuid(), room);
    }
}



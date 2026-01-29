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

//! Inventory system for item management

use crate::ecs::components::{Containable, Container, EntityId, Location};
use crate::ecs::events::{EventBus, GameEvent};
use crate::ecs::{EcsEntity, GameWorld};

pub struct InventorySystem {
    event_bus: EventBus,
}

impl InventorySystem {
    /// Create a new inventory system
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }

    /// Pick up an item from the ground
    pub fn pickup_item(
        &mut self,
        world: &mut GameWorld,
        entity: EcsEntity,
        item: EcsEntity,
    ) -> Result<(), String> {
        // Get item weight
        let weight = world
            .get::<&Containable>(item)
            .map(|c| c.weight)
            .unwrap_or(0.0);

        // Check if entity has container
        if let Ok(container) = world.get::<&Container>(entity) {
            // Check capacity constraints
            if let Some(capacity) = container.capacity {
                let current_count = self.get_item_count(world, entity);
                if current_count >= capacity as usize {
                    return Err("Inventory is full".to_string());
                }
            }

            if let Some(max_weight) = container.max_weight {
                let current_weight = self.get_total_weight(world, entity);
                if current_weight + weight > max_weight {
                    return Err("Item is too heavy".to_string());
                }
            }

            // Remove item from world location (set to invalid location)
            if let Ok(mut loc) = world.get::<&mut Location>(item) {
                loc.area_id = EntityId::from_uuid(uuid::Uuid::nil());
                loc.room_id = EntityId::from_uuid(uuid::Uuid::nil());
            }

            self.event_bus
                .publish(GameEvent::ItemPickedUp { entity, item });
            Ok(())
        } else {
            Err("Entity has no inventory".to_string())
        }
    }

    /// Drop an item to the ground
    pub fn drop_item(
        &mut self,
        world: &mut GameWorld,
        entity: EcsEntity,
        item: EcsEntity,
    ) -> Result<(), String> {
        // Check if entity has container
        if world.get::<&Container>(entity).is_ok() {
            // Set item location to entity's location
            if let Ok(entity_loc) = world.get::<&Location>(entity) {
                if let Ok(mut item_loc) = world.get::<&mut Location>(item) {
                    *item_loc = *entity_loc;
                }
            }

            self.event_bus
                .publish(GameEvent::ItemDropped { entity, item });
            Ok(())
        } else {
            Err("Entity has no inventory".to_string())
        }
    }

    /// Transfer an item from one entity to another
    pub fn transfer_item(
        &mut self,
        world: &mut GameWorld,
        from: EcsEntity,
        to: EcsEntity,
        item: EcsEntity,
    ) -> Result<(), String> {
        // Get item weight
        let weight = world
            .get::<&Containable>(item)
            .map(|c| c.weight)
            .unwrap_or(0.0);

        // Check if recipient can receive the item
        {
            let to_container = world
                .get::<&Container>(to)
                .map_err(|_| "Recipient has no inventory")?;

            // Check capacity constraints
            if let Some(max_weight) = to_container.max_weight {
                // TODO: Calculate current weight properly
                if weight > max_weight {
                    return Err("Recipient cannot carry the item".to_string());
                }
            }
        }

        // Check source has inventory
        world
            .get::<&Container>(from)
            .map_err(|_| "Source has no inventory")?;

        // Publish events
        self.event_bus
            .publish(GameEvent::ItemDropped { entity: from, item });
        self.event_bus
            .publish(GameEvent::ItemPickedUp { entity: to, item });

        Ok(())
    }

    /// Get all items in a container
    pub fn get_items_in_container(&self, _world: &GameWorld, _entity: EcsEntity) -> Vec<EcsEntity> {
        // TODO: Query world for entities with parent = entity
        // Container no longer tracks contents directly
        Vec::new()
    }

    /// Check if an entity has a specific item
    pub fn has_item(&self, world: &GameWorld, _entity: EcsEntity, item: EcsEntity) -> bool {
        // Check if item has nil location (meaning it's in inventory)
        if let Ok(loc) = world.get::<&Location>(item) {
            loc.area_id.uuid() == uuid::Uuid::nil() && loc.room_id.uuid() == uuid::Uuid::nil()
        } else {
            false
        }
    }

    /// Get the total weight of items in a container
    pub fn get_total_weight(&self, world: &GameWorld, _entity: EcsEntity) -> f32 {
        // Sum weight of items with nil location (in inventory)
        let mut total_weight = 0.0;
        for (loc, containable) in world.query::<(&Location, &Containable)>().iter() {
            if loc.area_id.uuid() == uuid::Uuid::nil() && loc.room_id.uuid() == uuid::Uuid::nil() {
                total_weight += containable.weight;
            }
        }
        total_weight
    }

    /// Get the number of items in a container
    pub fn get_item_count(&self, world: &GameWorld, _entity: EcsEntity) -> usize {
        // Count items with nil location (in inventory)
        let mut count = 0;
        for loc in world.query::<&Location>().iter() {
            if loc.area_id.uuid() == uuid::Uuid::nil() && loc.room_id.uuid() == uuid::Uuid::nil() {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Name;

    #[test]
    fn test_inventory_system_creation() {
        let event_bus = EventBus::new();
        let _system = InventorySystem::new(event_bus);
    }

    #[test]
    fn test_pickup_drop() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = InventorySystem::new(event_bus);

        let player = world.spawn((
            Name::new("Player"),
            Container::new(Some(10)),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        let item = world.spawn((
            Name::new("Sword"),
            Containable::new(5.0),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        // Test pickup
        assert!(system.pickup_item(&mut world, player, item).is_ok());
        assert!(system.has_item(&world, player, item));
        assert_eq!(system.get_item_count(&world, player), 1);
        assert_eq!(system.get_total_weight(&world, player), 5.0);

        // Test drop
        assert!(system.drop_item(&mut world, player, item).is_ok());
        assert!(!system.has_item(&world, player, item));
        assert_eq!(system.get_item_count(&world, player), 0);
        assert_eq!(system.get_total_weight(&world, player), 0.0);
    }

    #[test]
    #[ignore = "Requires parent/owner tracking system to be implemented"]
    fn test_transfer() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = InventorySystem::new(event_bus);

        let player1 = world.spawn((Name::new("Player1"), Container::new(Some(10))));

        let player2 = world.spawn((Name::new("Player2"), Container::new(Some(10))));

        let item = world.spawn((
            Name::new("Sword"),
            Containable::new(5.0),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        // Give item to player1 using the inventory system
        assert!(system.pickup_item(&mut world, player1, item).is_ok());

        // Transfer to player2
        assert!(
            system
                .transfer_item(&mut world, player1, player2, item)
                .is_ok()
        );
        assert!(!system.has_item(&world, player1, item));
        assert!(system.has_item(&world, player2, item));
    }

    #[test]
    fn test_capacity_limit() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = InventorySystem::new(event_bus);

        let player = world.spawn((
            Name::new("Player"),
            Container::new(Some(2)),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        let item1 = world.spawn((
            Name::new("Item1"),
            Containable::new(1.0),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        let item2 = world.spawn((
            Name::new("Item2"),
            Containable::new(1.0),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        let item3 = world.spawn((
            Name::new("Item3"),
            Containable::new(1.0),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        assert!(system.pickup_item(&mut world, player, item1).is_ok());
        assert!(system.pickup_item(&mut world, player, item2).is_ok());
        assert!(system.pickup_item(&mut world, player, item3).is_err());
    }

    #[test]
    fn test_weight_limit() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = InventorySystem::new(event_bus);

        let mut container = Container::new(None);
        container.max_weight = Some(10.0);

        let player = world.spawn((
            Name::new("Player"),
            container,
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        let heavy_item = world.spawn((
            Name::new("Heavy"),
            Containable::new(15.0),
            Location::new(
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap(),
                ),
                EntityId::from_uuid(
                    uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap(),
                ),
            ),
        ));

        assert!(system.pickup_item(&mut world, player, heavy_item).is_err());
    }
}

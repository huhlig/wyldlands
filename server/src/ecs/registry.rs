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

//! Entity Registry for mapping between ECS runtime handles and persistent UUIDs
//!
//! This module provides bidirectional mapping between:
//! - `EcsEntity`: hecs runtime entity handles (non-persistent, memory-only)
//! - `Uuid`: Persistent database identifiers
//!
//! The registry enables fast O(1) lookups in both directions, solving the
//! identifier confusion problem in the codebase.

use crate::ecs::EcsEntity;
use crate::ecs::components::EntityId;
use std::collections::HashMap;
use uuid::Uuid;

/// Registry for mapping between ECS entities and persistent UUIDs
#[derive(Debug, Default)]
pub struct EntityRegistry {
    /// Map from UUID to ECS entity handle
    uuid_to_entity: HashMap<Uuid, EcsEntity>,

    /// Map from ECS entity handle to UUID
    entity_to_uuid: HashMap<EcsEntity, Uuid>,
}

impl EntityRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            uuid_to_entity: HashMap::new(),
            entity_to_uuid: HashMap::new(),
        }
    }

    /// Register a mapping between an ECS entity and its UUID
    ///
    /// # Arguments
    /// * `entity` - The ECS runtime entity handle
    /// * `uuid` - The persistent UUID for this entity
    ///
    /// # Returns
    /// * `Ok(())` if registration succeeded
    /// * `Err(String)` if either the entity or UUID is already registered
    pub fn register(&mut self, entity: EcsEntity, uuid: Uuid) -> Result<(), String> {
        // Check for existing registrations
        if self.entity_to_uuid.contains_key(&entity) {
            return Err(format!("Entity {:?} is already registered", entity));
        }
        if self.uuid_to_entity.contains_key(&uuid) {
            return Err(format!("UUID {} is already registered", uuid));
        }

        // Register bidirectional mapping
        self.uuid_to_entity.insert(uuid, entity);
        self.entity_to_uuid.insert(entity, uuid);

        Ok(())
    }

    /// Unregister an entity by its ECS handle
    ///
    /// # Arguments
    /// * `entity` - The ECS entity to unregister
    ///
    /// # Returns
    /// * `Some(Uuid)` - The UUID that was associated with this entity
    /// * `None` - If the entity was not registered
    pub fn unregister_entity(&mut self, entity: EcsEntity) -> Option<Uuid> {
        if let Some(uuid) = self.entity_to_uuid.remove(&entity) {
            self.uuid_to_entity.remove(&uuid);
            Some(uuid)
        } else {
            None
        }
    }

    /// Unregister an entity by its UUID
    ///
    /// # Arguments
    /// * `uuid` - The UUID to unregister
    ///
    /// # Returns
    /// * `Some(EcsEntity)` - The entity that was associated with this UUID
    /// * `None` - If the UUID was not registered
    pub fn unregister_uuid(&mut self, uuid: Uuid) -> Option<EcsEntity> {
        if let Some(entity) = self.uuid_to_entity.remove(&uuid) {
            self.entity_to_uuid.remove(&entity);
            Some(entity)
        } else {
            None
        }
    }

    /// Look up an ECS entity by its UUID
    ///
    /// # Arguments
    /// * `uuid` - The UUID to look up
    ///
    /// # Returns
    /// * `Some(EcsEntity)` - The entity associated with this UUID
    /// * `None` - If no entity is registered with this UUID
    pub fn get_entity(&self, uuid: Uuid) -> Option<EcsEntity> {
        self.uuid_to_entity.get(&uuid).copied()
    }

    /// Look up a UUID by its ECS entity
    ///
    /// # Arguments
    /// * `entity` - The entity to look up
    ///
    /// # Returns
    /// * `Some(Uuid)` - The UUID associated with this entity
    /// * `None` - If no UUID is registered for this entity
    pub fn get_uuid(&self, entity: EcsEntity) -> Option<Uuid> {
        self.entity_to_uuid.get(&entity).copied()
    }

    /// Check if an entity is registered
    pub fn contains_entity(&self, entity: EcsEntity) -> bool {
        self.entity_to_uuid.contains_key(&entity)
    }

    /// Check if a UUID is registered
    pub fn contains_uuid(&self, uuid: Uuid) -> bool {
        self.uuid_to_entity.contains_key(&uuid)
    }

    /// Get the number of registered entities
    pub fn len(&self) -> usize {
        self.entity_to_uuid.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.entity_to_uuid.is_empty()
    }

    /// Clear all registrations
    pub fn clear(&mut self) {
        self.uuid_to_entity.clear();
        self.entity_to_uuid.clear();
    }

    /// Get all registered UUIDs
    pub fn uuids(&self) -> impl Iterator<Item = &Uuid> {
        self.uuid_to_entity.keys()
    }

    /// Get all registered entities
    pub fn entities(&self) -> impl Iterator<Item = &EcsEntity> {
        self.entity_to_uuid.keys()
    }

    /// Get an EntityId by combining entity and UUID lookup
    ///
    /// # Arguments
    /// * `entity` - The ECS entity to look up
    ///
    /// # Returns
    /// * `Some(EntityId)` - Combined entity/UUID identifier
    /// * `None` - If the entity is not registered
    pub fn get_entity_id(&self, entity: EcsEntity) -> Option<EntityId> {
        self.get_uuid(entity)
            .map(|uuid| EntityId::new(entity, uuid))
    }

    /// Get an EntityId by UUID
    ///
    /// # Arguments
    /// * `uuid` - The UUID to look up
    ///
    /// # Returns
    /// * `Some(EntityId)` - Combined entity/UUID identifier
    /// * `None` - If the UUID is not registered
    pub fn get_entity_id_by_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        self.get_entity(uuid)
            .map(|entity| EntityId::new(entity, uuid))
    }

    /// Get all EntityIds as an iterator
    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entity_to_uuid
            .iter()
            .map(|(entity, uuid)| EntityId::new(*entity, *uuid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::GameWorld;

    #[test]
    fn test_register_and_lookup() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity = world.spawn(());
        let uuid = Uuid::new_v4();

        // Register mapping
        assert!(registry.register(entity, uuid).is_ok());

        // Lookup both directions
        assert_eq!(registry.get_entity(uuid), Some(entity));
        assert_eq!(registry.get_uuid(entity), Some(uuid));
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity = world.spawn(());
        let uuid = Uuid::new_v4();

        // First registration succeeds
        assert!(registry.register(entity, uuid).is_ok());

        // Duplicate entity registration fails
        let uuid2 = Uuid::new_v4();
        assert!(registry.register(entity, uuid2).is_err());

        // Duplicate UUID registration fails
        let entity2 = world.spawn(());
        assert!(registry.register(entity2, uuid).is_err());
    }

    #[test]
    fn test_unregister() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity = world.spawn(());
        let uuid = Uuid::new_v4();

        registry.register(entity, uuid).unwrap();

        // Unregister by entity
        assert_eq!(registry.unregister_entity(entity), Some(uuid));
        assert_eq!(registry.get_entity(uuid), None);
        assert_eq!(registry.get_uuid(entity), None);

        // Re-register
        registry.register(entity, uuid).unwrap();

        // Unregister by UUID
        assert_eq!(registry.unregister_uuid(uuid), Some(entity));
        assert_eq!(registry.get_entity(uuid), None);
        assert_eq!(registry.get_uuid(entity), None);
    }

    #[test]
    fn test_contains() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity = world.spawn(());
        let uuid = Uuid::new_v4();

        assert!(!registry.contains_entity(entity));
        assert!(!registry.contains_uuid(uuid));

        registry.register(entity, uuid).unwrap();

        assert!(registry.contains_entity(entity));
        assert!(registry.contains_uuid(uuid));
    }

    #[test]
    fn test_clear() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity1 = world.spawn(());
        let entity2 = world.spawn(());
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        registry.register(entity1, uuid1).unwrap();
        registry.register(entity2, uuid2).unwrap();

        assert_eq!(registry.len(), 2);

        registry.clear();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_entity_id() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity = world.spawn(());
        let uuid = Uuid::new_v4();

        registry.register(entity, uuid).unwrap();

        let entity_id = registry.get_entity_id(entity).unwrap();
        assert_eq!(entity_id.entity(), entity);
        assert_eq!(entity_id.uuid(), uuid);
    }

    #[test]
    fn test_get_entity_id_by_uuid() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity = world.spawn(());
        let uuid = Uuid::new_v4();

        registry.register(entity, uuid).unwrap();

        let entity_id = registry.get_entity_id_by_uuid(uuid).unwrap();
        assert_eq!(entity_id.entity(), entity);
        assert_eq!(entity_id.uuid(), uuid);
    }

    #[test]
    fn test_entity_ids_iterator() {
        let mut registry = EntityRegistry::new();
        let mut world = GameWorld::new();

        let entity1 = world.spawn(());
        let entity2 = world.spawn(());
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        registry.register(entity1, uuid1).unwrap();
        registry.register(entity2, uuid2).unwrap();

        let ids: Vec<EntityId> = registry.entity_ids().collect();
        assert_eq!(ids.len(), 2);

        // Verify both entity IDs are present
        assert!(
            ids.iter()
                .any(|id| id.entity() == entity1 && id.uuid() == uuid1)
        );
        assert!(
            ids.iter()
                .any(|id| id.entity() == entity2 && id.uuid() == uuid2)
        );
    }
}

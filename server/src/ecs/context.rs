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
use crate::ecs::components::{CharacterBuilder, EntityId};
use crate::ecs::events::EventBus;
use crate::ecs::registry::EntityRegistry;
use crate::ecs::systems::CommandSystem;
use crate::models::ModelManager;
use crate::persistence::PersistenceManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::instrument;
use uuid::Uuid;

/// World context that provides safe access to ECS world, registry, and persistence
///
/// This context abstracts away lock management and provides safe passthrough methods
/// for common operations. Instead of manually acquiring locks, users can call methods
/// on the context that handle locking internally.
///
/// # Architecture
///
/// The context wraps three core components:
/// - **ECS World**: hecs::World for entity-component storage
/// - **Registry**: Bidirectional mapping between ECS entities and persistent UUIDs
/// - **Persistence Manager**: Database operations for loading/saving entities
///
/// # Safe Operation Methods
///
/// ## Entity Operations (automatic lock management)
/// - `spawn()` / `spawn_blocking()` - Create new entities
/// - `despawn()` - Remove entities
/// - `insert()` / `insert_one()` - Add components to entities
/// - `remove_one()` / `remove_one_blocking()` - Remove components
/// - `contains()` - Check entity existence
/// - `len()` / `is_empty()` - World statistics
///
/// ## Registry Operations
/// - `register_entity()` - Register ECS entity with UUID
/// - `unregister_entity()` / `unregister_uuid()` - Remove registrations
/// - `get_entity_by_uuid()` / `get_uuid_by_entity()` - Lookup mappings
/// - `get_entity_id()` / `get_entity_id_by_uuid()` - Get combined EntityId
///
/// ## Persistence Operations
/// - `mark_dirty()` / `mark_entity_dirty()` - Mark entities for saving
/// - `save()` - Save all dirty entities
/// - `save_entity()` / `save_entity_by_uuid()` - Save specific entities
/// - `load()` - Load entire world from database
/// - `load_character()` - Load specific character
/// - `create_character()` - Create new character in database
/// - `delete_entity()` - Remove entity from database
/// - `clear_dirty()` / `dirty_count()` / `is_dirty()` - Dirty tracking utilities
///
/// ## Manual Lock Access (for complex operations)
/// - `entities()` - Get Arc<RwLock<World>> for manual management
/// - `registry()` - Get Arc<RwLock<EntityRegistry>> for manual management
/// - `persistence_manager()` - Get Arc<PersistenceManager> reference
///
/// # Examples
///
/// ```ignore
/// // Spawn an entity with safe automatic locking
/// let entity = context.spawn((Name::new("Player"), Health::new(100))).await;
///
/// // Register it with a UUID for persistence
/// let uuid = Uuid::new_v4();
/// context.register_entity(entity, uuid).await;
///
/// // Mark it dirty so it gets saved
/// context.mark_entity_dirty(entity).await;
///
/// // Save all dirty entities
/// context.save().await?;
///
/// // For complex operations requiring multiple lookups, use manual locks:
/// let world = context.entities().read().await;
/// for (entity, (name, health)) in world.query::<(&Name, &Health)>().iter() {
///     // Process entities...
/// }
/// ```
pub struct WorldContext {
    entities: Arc<RwLock<hecs::World>>,
    registry: Arc<RwLock<EntityRegistry>>,
    persistence_manager: Arc<PersistenceManager>,
    llm_manager: Arc<ModelManager>,
    command_system: Arc<RwLock<CommandSystem>>,
    event_bus: EventBus,
}

impl WorldContext {
    /// Create a new world engine context
    pub fn new(persistence_manager: Arc<PersistenceManager>) -> Self {
        let event_bus = EventBus::new();
        let command_system = CommandSystem::new(event_bus.clone());

        Self {
            entities: Arc::new(RwLock::new(hecs::World::new())),
            registry: Arc::new(RwLock::new(EntityRegistry::new())),
            persistence_manager,
            llm_manager: Arc::new(ModelManager::new()),
            command_system: Arc::new(RwLock::new(command_system)),
            event_bus,
        }
    }

    /// Create a new world engine context with a custom LLM manager
    pub fn with_llm_manager(
        persistence_manager: Arc<PersistenceManager>,
        llm_manager: Arc<ModelManager>,
    ) -> Self {
        let event_bus = EventBus::new();
        let command_system = CommandSystem::new(event_bus.clone());

        Self {
            entities: Arc::new(RwLock::new(hecs::World::new())),
            registry: Arc::new(RwLock::new(EntityRegistry::new())),
            persistence_manager,
            llm_manager,
            command_system: Arc::new(RwLock::new(command_system)),
            event_bus,
        }
    }

    // ============================================================================
    // Direct Lock Access (for complex operations requiring manual lock management)
    // ============================================================================

    /// Get the ECS world - use only when you need manual lock management
    pub fn entities(&self) -> &Arc<RwLock<hecs::World>> {
        &self.entities
    }

    /// Get the entity registry - use only when you need manual lock management
    pub fn registry(&self) -> &Arc<RwLock<EntityRegistry>> {
        &self.registry
    }

    /// Get the persistence manager
    pub fn persistence(&self) -> &Arc<PersistenceManager> {
        &self.persistence_manager
    }

    /// Get the LLM manager
    pub fn llm_manager(&self) -> &Arc<ModelManager> {
        &self.llm_manager
    }

    /// Get the command system - use only when you need manual lock management
    pub fn command_system(&self) -> &Arc<RwLock<CommandSystem>> {
        &self.command_system
    }

    /// Get the event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    // ============================================================================
    // Safe Entity Operations (automatic lock management)
    // ============================================================================

    /// Spawn a new entity with the given components
    ///
    /// Returns the spawned entity handle.
    #[instrument(skip(self, bundle))]
    pub async fn spawn(&self, bundle: impl hecs::DynamicBundle) -> EcsEntity {
        let mut world = self.entities.write().await;
        world.spawn(bundle)
    }

    /// Spawn a new entity with the given components (blocking version)
    #[instrument(skip(self, bundle))]
    pub fn spawn_blocking(&self, bundle: impl hecs::DynamicBundle) -> EcsEntity {
        let mut world = self.entities.blocking_write();
        world.spawn(bundle)
    }

    /// Despawn an entity, removing it and all its components
    ///
    /// Returns Ok(()) if the entity was despawned, or Err if it didn't exist.
    #[instrument(skip(self))]
    pub async fn despawn(&self, entity: EcsEntity) -> Result<(), hecs::NoSuchEntity> {
        let mut world = self.entities.write().await;
        world.despawn(entity)
    }

    /// Check if an entity exists in the world
    #[instrument(skip(self))]
    pub async fn contains(&self, entity: EcsEntity) -> bool {
        let world = self.entities.read().await;
        world.contains(entity)
    }

    /// Get the number of entities in the world
    pub async fn len(&self) -> usize {
        let world = self.entities.read().await;
        world.len() as usize
    }

    /// Check if the world is empty
    pub async fn is_empty(&self) -> bool {
        let world = self.entities.read().await;
        world.is_empty()
    }

    /// Insert a component into an entity
    ///
    /// Returns Ok(()) if successful, or Err if the entity doesn't exist.
    pub async fn insert(
        &self,
        entity: EcsEntity,
        component: impl hecs::DynamicBundle,
    ) -> Result<(), hecs::NoSuchEntity> {
        let mut world = self.entities.write().await;
        world.insert(entity, component)
    }

    /// Insert a component into an entity, replacing any existing component of the same type
    pub async fn insert_one(
        &self,
        entity: EcsEntity,
        component: impl hecs::Component,
    ) -> Result<(), hecs::NoSuchEntity> {
        let mut world = self.entities.write().await;
        world.insert_one(entity, component)
    }

    /// Remove a component from an entity
    ///
    /// Returns Ok(component) if the component was present, or Err if not.
    pub async fn remove_one<T: hecs::Component>(
        &self,
        entity: EcsEntity,
    ) -> Result<T, hecs::ComponentError> {
        let mut world = self.entities.write().await;
        world.remove_one::<T>(entity)
    }

    /// Remove a component from an entity (blocking version)
    pub fn remove_one_blocking<T: hecs::Component>(
        &self,
        entity: EcsEntity,
    ) -> Result<T, hecs::ComponentError> {
        let mut world = self.entities.blocking_write();
        world.remove_one::<T>(entity)
    }

    // ============================================================================
    // Safe Registry Operations
    // ============================================================================

    /// Register a mapping between an ECS entity and its UUID
    pub async fn register_entity(&self, entity: EcsEntity, uuid: Uuid) {
        let mut registry = self.registry.write().await;
        registry
            .register(entity, uuid)
            .expect("Failed to register entity in registry");
    }

    /// Unregister an entity from the registry
    pub async fn unregister_entity(&self, entity: EcsEntity) -> Option<Uuid> {
        let mut registry = self.registry.write().await;
        registry.unregister_entity(entity)
    }

    /// Unregister an entity from the registry by UUID
    pub async fn unregister_uuid(&self, uuid: Uuid) -> Option<EcsEntity> {
        let mut registry = self.registry.write().await;
        registry.unregister_uuid(uuid)
    }

    /// Look up an ECS entity by its UUID
    pub async fn get_entity_by_uuid(&self, uuid: Uuid) -> Option<EcsEntity> {
        let registry = self.registry.read().await;
        registry.get_entity(uuid)
    }

    /// Look up a UUID by its ECS entity handle
    pub async fn get_uuid_by_entity(&self, entity: EcsEntity) -> Option<Uuid> {
        let registry = self.registry.read().await;
        registry.get_uuid(entity)
    }

    /// Get EntityId by ECS entity handle
    pub async fn get_entity_id(&self, entity: EcsEntity) -> Option<EntityId> {
        let registry = self.registry.read().await;
        registry.get_entity_id(entity)
    }

    /// Get EntityId by UUID
    pub async fn get_entity_id_by_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        let registry = self.registry.read().await;
        registry.get_entity_id_by_uuid(uuid)
    }

    // ============================================================================
    // Safe Persistence Operations
    // ============================================================================

    /// Mark an entity as dirty (needs to be saved)
    pub async fn mark_dirty(&self, uuid: Uuid) {
        self.persistence_manager.mark_dirty(uuid).await;
    }

    /// Mark an entity as dirty by EcsEntity (looks up UUID first)
    pub async fn mark_entity_dirty(&self, entity: EcsEntity) {
        if let Some(uuid) = self.get_uuid_by_entity(entity).await {
            self.persistence_manager.mark_dirty(uuid).await;
        }
    }

    /// Mark an entity as dirty using EntityId
    pub async fn mark_dirty_by_id(&self, entity_id: EntityId) {
        self.persistence_manager.mark_dirty_by_id(entity_id).await;
    }

    /// Clear the dirty entities set
    pub async fn clear_dirty(&self) {
        self.persistence_manager.clear_dirty().await;
    }

    /// Get the count of dirty entities
    pub async fn dirty_count(&self) -> usize {
        self.persistence_manager.dirty_count().await
    }

    /// Check if a specific entity is marked as dirty
    pub async fn is_dirty(&self, uuid: Uuid) -> bool {
        self.persistence_manager.is_dirty(uuid).await
    }

    /// Check if an ECS entity is marked as dirty (looks up UUID first)
    pub async fn is_entity_dirty(&self, entity: EcsEntity) -> bool {
        if let Some(uuid) = self.get_uuid_by_entity(entity).await {
            self.persistence_manager.is_dirty(uuid).await
        } else {
            false
        }
    }

    /// Save all dirty entities to the database
    #[instrument(skip(self))]
    pub async fn save(&self) -> Result<usize, String> {
        let world = self.entities.read().await;
        self.persistence_manager.auto_save(&world).await
    }

    /// Load all persistent entities from the database into the world
    #[instrument(skip(self))]
    pub async fn load(&self) -> Result<usize, String> {
        let mut world = self.entities.write().await;
        let mut registry = self.registry.write().await;
        self.persistence_manager
            .load_world(&mut world, &mut registry)
            .await
    }

    /// Create a new default character in the database
    ///
    /// This creates the entity record, links it to an account, and sets up
    /// initial components like name and description.
    pub async fn create_character(&self, account_id: Uuid, name: &str) -> Result<Uuid, String> {
        self.create_character_with_builder(
            account_id,
            &CharacterBuilder::new(name.to_string(), 0, 0),
        )
        .await
    }

    /// Create a new character in the database with a Default Sheet
    ///
    /// This creates the entity record, links it to an account, and sets up
    /// initial components like name and description.
    pub async fn create_character_with_builder(
        &self,
        account_id: Uuid,
        builder: &CharacterBuilder,
    ) -> Result<Uuid, String> {
        self.persistence_manager
            .create_character_with_builder(account_id, builder)
            .await
    }

    /// Load a specific character entity from the database into the world
    ///
    /// Returns the EcsEntity handle if successful.
    pub async fn load_character(&self, entity_id: Uuid) -> Result<EcsEntity, String> {
        let mut world = self.entities.write().await;
        let registry = self.registry.read().await;
        self.persistence_manager
            .load_entity(&mut world, &registry, entity_id)
            .await
    }

    /// Save a specific entity to the database by UUID
    pub async fn save_entity_by_uuid(&self, entity_id: Uuid) -> Result<(), String> {
        let world = self.entities.read().await;
        let registry = self.registry.read().await;

        if let Some(entity) = registry.get_entity(entity_id) {
            self.persistence_manager.save_entity(&world, entity).await
        } else {
            Err(format!("Entity {} not found in registry", entity_id))
        }
    }

    /// Save a specific entity to the database by ECS entity handle
    pub async fn save_entity(&self, entity: EcsEntity) -> Result<(), String> {
        let world = self.entities.read().await;
        self.persistence_manager.save_entity(&world, entity).await
    }

    /// Delete an entity from the database
    pub async fn delete_entity(&self, entity_id: Uuid) -> Result<(), String> {
        // First remove from registry
        self.unregister_uuid(entity_id).await;
        // Then delete from database
        self.persistence_manager.delete_entity(entity_id).await
    }
}

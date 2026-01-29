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

//! Persistence manager for ECS entity storage and retrieval
//!
//! This manager handles:
//! - Loading full ECS entities from component tables
//! - Saving ECS entities to individual component tables
//! - Auto-save of dirty entities
//! - Component-based relational storage

use crate::ecs::components::*;
use crate::ecs::registry::EntityRegistry;
use crate::ecs::{EcsEntity, GameWorld};
use hecs::Entity;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Persistence manager for ECS entities
pub struct PersistenceManager {
    /// Database connection pool
    pool: PgPool,

    /// Set of entity UUIDs that need saving
    dirty_entities: Arc<RwLock<HashSet<Uuid>>>,

    /// Auto-save interval in seconds
    auto_save_interval: u64,
}

impl PersistenceManager {
    /// Create a new persistence manager
    pub fn new(pool: PgPool, auto_save_interval: u64) -> Self {
        Self {
            pool,
            dirty_entities: Arc::new(RwLock::new(HashSet::new())),
            auto_save_interval,
        }
    }

    /// Get a reference to the database pool
    pub fn database(&self) -> &PgPool {
        &self.pool
    }

    /// Get account by ID from database
    pub async fn get_account_by_id(&self, account_id: Uuid) -> Result<wyldlands_common::account::Account, sqlx::Error> {
        sqlx::query_as::<_, wyldlands_common::account::Account>(
            "SELECT id, login, display, timezone, discord, email, rating, active, role
             FROM wyldlands.accounts
             WHERE id = $1"
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Create a new character entity in the database
    /// Returns the UUID of the newly created character
    pub async fn create_character(
        &self,
        account_id: Uuid,
        name: String,
        description_short: String,
        description_long: String,
    ) -> Result<Uuid, String> {
        tracing::info!("Creating new character for account: {}", account_id);

        let entity_uuid = Uuid::new_v4();

        // Start a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        // Create base entity record
        sqlx::query(
            "INSERT INTO wyldlands.entities (uuid, created_at, updated_at)
             VALUES ($1, NOW(), NOW())",
        )
        .bind(entity_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create entity record: {}", e))?;

        // Link to account in entity_avatars
        sqlx::query(
            "INSERT INTO wyldlands.entity_avatars (entity_id, account_id, created_at)
             VALUES ($1, $2, NOW())",
        )
        .bind(entity_uuid)
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to link avatar to account: {}", e))?;

        // Create name component
        let keywords = vec![name.to_lowercase()];
        sqlx::query(
            "INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
             VALUES ($1, $2, $3)",
        )
        .bind(entity_uuid)
        .bind(&name)
        .bind(&keywords)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create name component: {}", e))?;

        // Create description component
        sqlx::query(
            "INSERT INTO wyldlands.entity_description (entity_id, short, long)
             VALUES ($1, $2, $3)",
        )
        .bind(entity_uuid)
        .bind(&description_short)
        .bind(&description_long)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create description component: {}", e))?;

        // Create default body attributes
        sqlx::query(
            "INSERT INTO wyldlands.entity_body_attributes 
             (entity_id, score_offence, score_finesse, score_defence,
              health_current, health_maximum, health_regen,
              energy_current, energy_maximum, energy_regen)
             VALUES ($1, 10, 10, 10, 100.0, 100.0, 1.0, 100.0, 100.0, 1.0)",
        )
        .bind(entity_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create body attributes: {}", e))?;

        // Create default mind attributes
        sqlx::query(
            "INSERT INTO wyldlands.entity_mind_attributes 
             (entity_id, score_offence, score_finesse, score_defence,
              health_current, health_maximum, health_regen,
              energy_current, energy_maximum, energy_regen)
             VALUES ($1, 10, 10, 10, 100.0, 100.0, 1.0, 100.0, 100.0, 1.0)",
        )
        .bind(entity_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create mind attributes: {}", e))?;

        // Create default soul attributes
        sqlx::query(
            "INSERT INTO wyldlands.entity_soul_attributes 
             (entity_id, score_offence, score_finesse, score_defence,
              health_current, health_maximum, health_regen,
              energy_current, energy_maximum, energy_regen)
             VALUES ($1, 10, 10, 10, 100.0, 100.0, 1.0, 100.0, 100.0, 1.0)",
        )
        .bind(entity_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create soul attributes: {}", e))?;

        // Create commandable component
        sqlx::query(
            "INSERT INTO wyldlands.entity_commandable (entity_id, max_queue_size)
             VALUES ($1, 10)",
        )
        .bind(entity_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create commandable component: {}", e))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        tracing::info!(
            "Created new character {} for account {}",
            entity_uuid,
            account_id
        );
        Ok(entity_uuid)
    }

    /// Load all components for an entity that already exists in the world
    /// This is used during world loading after all entities are registered
    async fn load_entity_components(
        &self,
        world: &mut GameWorld,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
    ) -> Result<(), String> {

        self.load_avatar_component(entity_uuid, entity_id, world)
            .await?;
        // Load components based on what exists in the database
        self.load_name_component(entity_uuid, entity_id, world)
            .await?;
        self.load_description_component(entity_uuid, entity_id, world)
            .await?;
        self.load_body_attributes_component(entity_uuid, entity_id, world)
            .await?;
        self.load_mind_attributes_component(entity_uuid, entity_id, world)
            .await?;
        self.load_soul_attributes_component(entity_uuid, entity_id, world)
            .await?;
        self.load_skills_component(entity_uuid, entity_id, world)
            .await?;
        self.load_location_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_combatant_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_equipment_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_ai_controller_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_personality_component(entity_uuid, entity_id, world)
            .await?;
        self.load_area_component(entity_uuid, entity_id, world)
            .await?;
        self.load_room_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_room_exits_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_container_component(entity_uuid, entity_id, world)
            .await?;
        self.load_containable_component(entity_uuid, entity_id, world)
            .await?;
        self.load_enterable_component(registry, entity_uuid, entity_id, world)
            .await?;
        self.load_equipable_component(entity_uuid, entity_id, world)
            .await?;
        self.load_weapon_component(entity_uuid, entity_id, world)
            .await?;
        self.load_material_component(entity_uuid, entity_id, world)
            .await?;
        self.load_armor_defense_component(entity_uuid, entity_id, world)
            .await?;
        self.load_commandable_component(entity_uuid, entity_id, world)
            .await?;
        self.load_interactable_component(entity_uuid, entity_id, world)
            .await?;

        // Mark as persistent
        world
            .insert_one(entity_id, Persistent)
            .map_err(|e| format!("Failed to add Persistent marker: {}", e))?;

        Ok(())
    }

    /// Load any entity from database by entity UUID
    /// Loads all components that exist for the entity (characters, rooms, objects, NPCs, etc.)
    pub async fn load_entity(
        &self,
        world: &mut GameWorld,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
    ) -> Result<EcsEntity, String> {
        tracing::info!("Loading entity: {}", entity_uuid);

        // First, check if entity exists
        let exists: Option<(Uuid,)> =
            sqlx::query_as("SELECT uuid FROM wyldlands.entities WHERE uuid = $1")
                .bind(entity_uuid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| format!("Database error loading entity: {}", e))?;

        if exists.is_none() {
            return Err(format!("No entity found with UUID: {}", entity_uuid));
        }

        // Create the entity in the world
        let entity_id = world.spawn((EntityUuid(entity_uuid),));

        // Load all components
        self.load_entity_components(world, registry, entity_uuid, entity_id).await?;

        tracing::info!("Loaded entity {} as ECS entity {:?}", entity_uuid, entity_id);

        Ok(entity_id)
    }

    /// Convenience method for loading character entities (alias for load_entity)
    pub async fn load_character(
        &self,
        world: &mut GameWorld,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
    ) -> Result<EcsEntity, String> {
        self.load_entity(world, registry, entity_uuid).await
    }

    /// Load all persistent entities from the database into the world
    /// Excludes inactive player character avatars (available = false)
    /// Loads: Areas, Rooms, NPCs, Objects, Active Players, etc.
    /// Note: All entities in the database are considered persistent by default
    pub async fn load_world(&self, world: &mut GameWorld, registry: &mut EntityRegistry) -> Result<usize, String> {
        tracing::info!("Loading world from database...");

        // Get all entity UUIDs, excluding inactive avatars
        let entity_uuids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT e.uuid
             FROM wyldlands.entities e
             LEFT JOIN wyldlands.entity_avatars ea ON e.uuid = ea.entity_id
             WHERE ea.entity_id IS NULL OR ea.available = true"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to query world entities: {}", e))?;

        let total_entities = entity_uuids.len();
        tracing::info!("Found {} entities to load", total_entities);

        // Phase 1: Create all entities and register them so cross-references can be resolved
        tracing::info!("Phase 1: Creating entities and registering UUIDs...");
        for (entity_uuid,) in &entity_uuids {
            let entity_id = world.spawn((EntityUuid(*entity_uuid),));
            registry.register(entity_id, *entity_uuid)
                .map_err(|e| format!("Failed to register entity {}: {}", entity_uuid, e))?;
        }
        tracing::info!("Registered {} entities", total_entities);

        // Phase 2: Load all components with fully resolved EntityIds
        tracing::info!("Phase 2: Loading components...");
        let mut loaded_count = 0;
        let mut failed_count = 0;

        for (entity_uuid,) in entity_uuids {
            let entity_id = registry.get_entity(entity_uuid)
                .ok_or_else(|| format!("Entity {} not found in registry", entity_uuid))?;

            match self.load_entity_components(world, registry, entity_uuid, entity_id).await {
                Ok(_) => {
                    loaded_count += 1;
                    if loaded_count % 100 == 0 {
                        tracing::info!("Loaded {}/{} entities...", loaded_count, total_entities);
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    tracing::error!("Failed to load entity {}: {}", entity_uuid, e);
                }
            }
        }

        tracing::info!(
            "World loading complete: {} loaded, {} failed, {} total",
            loaded_count,
            failed_count,
            total_entities
        );

        if failed_count > 0 {
            tracing::warn!("{} entities failed to load", failed_count);
        }

        Ok(loaded_count)
    }

    /// Load Name component
    async fn load_name_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(String, Vec<String>)> = sqlx::query_as(
            "SELECT display, keywords FROM wyldlands.entity_name WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load name component: {}", e))?;

        if let Some((display, keywords)) = row {
            let name = Name { display, keywords };
            world
                .insert_one(entity_id, name)
                .map_err(|e| format!("Failed to add Name component: {}", e))?;
        }

        Ok(())
    }

    /// Load Avatar component
    async fn load_avatar_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(Uuid, bool)> = sqlx::query_as(
            "SELECT account_id, available FROM wyldlands.entity_avatars WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load avatar component: {}", e))?;

        if let Some((account_id, available)) = row {
            let avatar = Avatar {
                account_id,
                available,
            };
            world
                .insert_one(entity_id, avatar)
                .map_err(|e| format!("Failed to add Avatar component: {}", e))?;
        }

        Ok(())
    }

    /// Load Description component
    async fn load_description_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT short, long FROM wyldlands.entity_description WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load description component: {}", e))?;

        if let Some((short, long)) = row {
            let description = Description { short, long };
            world
                .insert_one(entity_id, description)
                .map_err(|e| format!("Failed to add Description component: {}", e))?;
        }

        Ok(())
    }

    /// Load BodyAttributes component
    async fn load_body_attributes_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(i32, i32, i32, f32, f32, f32, f32, f32, f32)> = sqlx::query_as(
            "SELECT score_offence, score_finesse, score_defence, 
                    health_current, health_maximum, health_regen,
                    energy_current, energy_maximum, energy_regen
             FROM wyldlands.entity_body_attributes WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load body attributes component: {}", e))?;

        if let Some((
            score_offence,
            score_finesse,
            score_defence,
            health_current,
            health_maximum,
            health_regen,
            energy_current,
            energy_maximum,
            energy_regen,
        )) = row
        {
            let attributes = BodyAttributes {
                score_offence,
                score_finesse,
                score_defence,
                health_current,
                health_maximum,
                health_regen,
                energy_current,
                energy_maximum,
                energy_regen,
            };
            world
                .insert_one(entity_id, attributes)
                .map_err(|e| format!("Failed to add BodyAttributes component: {}", e))?;
        }

        Ok(())
    }

    /// Load MindAttributes component
    async fn load_mind_attributes_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(i32, i32, i32, f32, f32, f32, f32, f32, f32)> = sqlx::query_as(
            "SELECT score_offence, score_finesse, score_defence, 
                    health_current, health_maximum, health_regen,
                    energy_current, energy_maximum, energy_regen
             FROM wyldlands.entity_mind_attributes WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load mind attributes component: {}", e))?;

        if let Some((
            score_offence,
            score_finesse,
            score_defence,
            health_current,
            health_maximum,
            health_regen,
            energy_current,
            energy_maximum,
            energy_regen,
        )) = row
        {
            let attributes = MindAttributes {
                score_offence,
                score_finesse,
                score_defence,
                health_current,
                health_maximum,
                health_regen,
                energy_current,
                energy_maximum,
                energy_regen,
            };
            world
                .insert_one(entity_id, attributes)
                .map_err(|e| format!("Failed to add MindAttributes component: {}", e))?;
        }

        Ok(())
    }

    /// Load SoulAttributes component
    async fn load_soul_attributes_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(i32, i32, i32, f32, f32, f32, f32, f32, f32)> = sqlx::query_as(
            "SELECT score_offence, score_finesse, score_defence, 
                    health_current, health_maximum, health_regen,
                    energy_current, energy_maximum, energy_regen
             FROM wyldlands.entity_soul_attributes WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load soul attributes component: {}", e))?;

        if let Some((
            score_offence,
            score_finesse,
            score_defence,
            health_current,
            health_maximum,
            health_regen,
            energy_current,
            energy_maximum,
            energy_regen,
        )) = row
        {
            let attributes = SoulAttributes {
                score_offence,
                score_finesse,
                score_defence,
                health_current,
                health_maximum,
                health_regen,
                energy_current,
                energy_maximum,
                energy_regen,
            };
            world
                .insert_one(entity_id, attributes)
                .map_err(|e| format!("Failed to add SoulAttributes component: {}", e))?;
        }

        Ok(())
    }

    /// Load Skills component
    async fn load_skills_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let rows: Vec<(String, i32, i32, i32)> = sqlx::query_as(
            "SELECT skill_name, level, experience, knowledge FROM wyldlands.entity_skills WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to load skills component: {}", e))?;

        if !rows.is_empty() {
            let mut skills_map = HashMap::new();
            for (skill_name, _level, experience, knowledge) in rows {
                // Convert string skill name to SkillId
                if let Some(skill_id) = SkillId::from_string(&skill_name) {
                    skills_map.insert(
                        skill_id,
                        Skill {
                            experience,
                            knowledge,
                        },
                    );
                } else {
                    tracing::warn!("Unknown skill name in database: {}", skill_name);
                }
            }

            let skills = Skills { skills: skills_map };
            world
                .insert_one(entity_id, skills)
                .map_err(|e| format!("Failed to add Skills component: {}", e))?;
        }

        Ok(())
    }

    /// Load Location component
    async fn load_location_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(Uuid,Uuid)> = sqlx::query_as(
            "SELECT r.area_id, l.room_id FROM wyldlands.entity_location AS l LEFT JOIN wyldlands.entity_rooms as r ON l.room_id = r.entity_id WHERE l.entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load location component: {}", e))?;

        if let Some((area_uuid, room_uuid)) = row {
            let area_id = registry.get_entity_id_by_uuid(area_uuid)
                .ok_or_else(|| format!("Area UUID {} not found in registry", area_uuid))?;
            let room_id = registry.get_entity_id_by_uuid(room_uuid)
                .ok_or_else(|| format!("Room UUID {} not found in registry", room_uuid))?;
            let location = Location { area_id, room_id };
            world
                .insert_one(entity_id, location)
                .map_err(|e| format!("Failed to add Location component: {}", e))?;
        }

        Ok(())
    }

    /// Load Combatant component
    async fn load_combatant_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(bool, Option<Uuid>, i32, f32, f32)> = sqlx::query_as(
            "SELECT in_combat, target_id, initiative, action_cooldown, time_since_action
             FROM wyldlands.entity_combatant WHERE entity_id = $1",
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load combatant component: {}", e))?;

        if let Some((in_combat, target_uuid, initiative, action_cooldown, time_since_action)) = row {
            let target_id = target_uuid.and_then(|uuid| registry.get_entity_id_by_uuid(uuid));
            let combatant = Combatant {
                in_combat,
                target_id,
                initiative,
                action_cooldown,
                time_since_action,
                is_defending: false,
                defense_bonus: 0,
            };
            world
                .insert_one(entity_id, combatant)
                .map_err(|e| format!("Failed to add Combatant component: {}", e))?;
        }

        Ok(())
    }

    /// Load Equipment component
    async fn load_equipment_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let rows: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT slot, item_id FROM wyldlands.entity_equipment WHERE entity_id = $1 AND item_id IS NOT NULL"
        )
        .bind(entity_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to load equipment component: {}", e))?;

        if !rows.is_empty() {
            let mut slots = HashMap::new();
            for (slot_str, item_uuid) in rows {
                if let Some(slot) = EquipSlot::from_str(&slot_str) {
                    if let Some(item_id) = registry.get_entity_id_by_uuid(item_uuid) {
                        slots.insert(slot, item_id);
                    }
                }
            }

            let equipment = Equipment { slots };
            world
                .insert_one(entity_id, equipment)
                .map_err(|e| format!("Failed to add Equipment component: {}", e))?;
        }

        Ok(())
    }

    /// Load AIController component
    async fn load_ai_controller_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(String, Option<String>, String, Option<Uuid>, f32, f32)> = sqlx::query_as(
            "SELECT behavior_type, current_goal, state_type, state_target_id, update_interval, time_since_update
             FROM wyldlands.entity_ai_controller WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load AI controller component: {}", e))?;

        if let Some((
            behavior_type_str,
            current_goal,
            state_type_str,
            state_target_uuid,
            update_interval,
            time_since_update,
        )) = row
        {
            if let (Some(behavior_type), Some(state_type)) = (
                BehaviorType::from_str(&behavior_type_str),
                StateType::from_str(&state_type_str),
            ) {
                let state_target_id = state_target_uuid.and_then(|uuid| registry.get_entity_id_by_uuid(uuid));
                let ai_controller = AIController {
                    behavior_type,
                    current_goal,
                    state_type,
                    state_target_id,
                    update_interval,
                    time_since_update,
                };
                world
                    .insert_one(entity_id, ai_controller)
                    .map_err(|e| format!("Failed to add AIController component: {}", e))?;
            }
        }

        Ok(())
    }

    /// Load Personality component
    async fn load_personality_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT background, speaking_style FROM wyldlands.entity_personality WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load personality component: {}", e))?;

        if let Some((background, speaking_style)) = row {
            let personality = Personality {
                background,
                speaking_style,
            };
            world
                .insert_one(entity_id, personality)
                .map_err(|e| format!("Failed to add Personality component: {}", e))?;
        }

        Ok(())
    }

    /// Load Area component
    async fn load_area_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(String, Vec<String>)> = sqlx::query_as(
            "SELECT area_kind::text, COALESCE(area_flags, '{}') as area_flags
             FROM wyldlands.entity_areas WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load area component: {}", e))?;

        if let Some((area_kind_str, area_flags)) = row {
            let area_kind = match area_kind_str.as_str() {
                "Overworld" => AreaKind::Overworld,
                "Vehicle" => AreaKind::Vehicle,
                "Building" => AreaKind::Building,
                "Dungeon" => AreaKind::Dungeon,
                _ => return Err(format!("Unknown area kind: {}", area_kind_str)),
            };
            let area = Area { area_kind, area_flags };
            world
                .insert_one(entity_id, area)
                .map_err(|e| format!("Failed to add Area component: {}", e))?;
        }

        Ok(())
    }

    /// Load Room component
    async fn load_room_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(Uuid, Vec<String>)> = sqlx::query_as(
            "SELECT area_id, COALESCE(room_flags::text[], '{}') as room_flags
             FROM wyldlands.entity_rooms WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load room component: {}", e))?;

        if let Some((area_uuid, flags_strs)) = row {
            let area_id = registry.get_entity_id_by_uuid(area_uuid)
                .ok_or_else(|| format!("Area UUID {} not found in registry", area_uuid))?;
            let mut room_flags = Vec::new();
            for flag_str in flags_strs {
                match flag_str.as_str() {
                    "Breathable" => room_flags.push(RoomFlag::Breathable),
                    _ => tracing::warn!("Unknown room flag: {}", flag_str),
                }
            }
            let room = Room { area_id, room_flags };
            world
                .insert_one(entity_id, room)
                .map_err(|e| format!("Failed to add Room component: {}", e))?;
        }

        Ok(())
    }

    /// Load room exits (multiple exits per room into a single Exits component)
    async fn load_room_exits_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let rows: Vec<(Uuid, String, bool, bool, Option<i32>, bool, bool, Option<String>, Option<i32>, bool)> = sqlx::query_as(
            "SELECT dest_id, direction, closeable, closed, door_rating, lockable, locked, unlock_code, lock_rating, transparent
             FROM wyldlands.entity_room_exits WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to load room exits: {}", e))?;

        if !rows.is_empty() {
            let mut exits = Exits::new();

            for (dest_uuid, direction, closeable, closed, door_rating, lockable, locked, unlock_code, lock_rating, transparent) in rows {
                let dest_id = registry.get_entity_id_by_uuid(dest_uuid)
                    .ok_or_else(|| format!("Exit destination UUID {} not found in registry", dest_uuid))?;
                let exit_data = ExitData {
                    dest_id,
                    direction,
                    closeable,
                    closed,
                    door_rating,
                    lockable,
                    locked,
                    unlock_code,
                    lock_rating,
                    transparent,
                };
                exits.exits.push(exit_data);
            }

            world
                .insert_one(entity_id, exits)
                .map_err(|e| format!("Failed to add Exits component: {}", e))?;
        }

        Ok(())
    }

    /// Load Container component
    async fn load_container_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(Option<i32>, Option<f32>, bool, bool, Option<i32>, bool, bool, Option<String>, Option<i32>, bool)> = sqlx::query_as(
            "SELECT capacity, max_weight, closeable, closed, container_rating, lockable, locked, unlock_code, lock_rating, transparent
             FROM wyldlands.entity_container WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load container component: {}", e))?;

        if let Some((capacity, max_weight, closeable, closed, container_rating, lockable, locked, unlock_code, lock_rating, transparent)) = row {
            let container = Container {
                capacity,
                max_weight,
                closeable,
                closed,
                container_rating,
                lockable,
                locked,
                unlock_code,
                lock_rating,
                transparent,
            };
            world
                .insert_one(entity_id, container)
                .map_err(|e| format!("Failed to add Container component: {}", e))?;
        }

        Ok(())
    }

    /// Load Containable component
    async fn load_containable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(f32, String, bool, i32)> = sqlx::query_as(
            "SELECT weight, size::text, stackable, stack_size
             FROM wyldlands.entity_containable WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load containable component: {}", e))?;

        if let Some((weight, size_str, stackable, stack_size)) = row {
            let size = match size_str.as_str() {
                "Fine" => Size::Tiny,
                "Diminutive" => Size::Tiny,
                "Tiny" => Size::Tiny,
                "Small" => Size::Small,
                "Medium" => Size::Medium,
                "Large" => Size::Large,
                "Huge" => Size::Huge,
                "Gargantuan" => Size::Huge,
                "Colossal" => Size::Huge,
                _ => Size::Medium,
            };
            let containable = Containable {
                weight,
                size,
                stackable,
                stack_size,
            };
            world
                .insert_one(entity_id, containable)
                .map_err(|e| format!("Failed to add Containable component: {}", e))?;
        }

        Ok(())
    }

    /// Load Enterable component
    async fn load_enterable_component(
        &self,
        registry: &EntityRegistry,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(Uuid, bool, bool, Option<i32>, bool, bool, Option<String>, Option<i32>, bool)> = sqlx::query_as(
            "SELECT dest_id, closeable, closed, door_rating, lockable, locked, unlock_code, lock_rating, transparent
             FROM wyldlands.entity_enterable WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load enterable component: {}", e))?;

        if let Some((dest_uuid, closeable, closed, door_rating, lockable, locked, unlock_code, lock_rating, transparent)) = row {
            let dest_id = registry.get_entity_id_by_uuid(dest_uuid)
                .ok_or_else(|| format!("Enterable destination UUID {} not found in registry", dest_uuid))?;
            let enterable = Enterable {
                dest_id,
                closeable,
                closed,
                door_rating,
                lockable,
                locked,
                unlock_code,
                lock_rating,
                transparent,
            };
            world
                .insert_one(entity_id, enterable)
                .map_err(|e| format!("Failed to add Enterable component: {}", e))?;
        }

        Ok(())
    }

    /// Load Equipable component
    async fn load_equipable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(Vec<String>,)> = sqlx::query_as(
            "SELECT slots::text[] FROM wyldlands.entity_equipable WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load equipable component: {}", e))?;

        if let Some((slot_strs,)) = row {
            let mut slots = Vec::new();
            for slot_str in slot_strs {
                if let Some(slot) = EquipSlot::from_str(&slot_str) {
                    slots.push(slot);
                }
            }
            let equipable = Equipable { slots };
            world
                .insert_one(entity_id, equipable)
                .map_err(|e| format!("Failed to add Equipable component: {}", e))?;
        }

        Ok(())
    }

    /// Load Weapon component
    async fn load_weapon_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(i32, i32, i32, String, f32, f32)> = sqlx::query_as(
            "SELECT damage_min, damage_max, damage_cap, damage_type::text, attack_speed, range
             FROM wyldlands.entity_weapon WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load weapon component: {}", e))?;

        if let Some((damage_min, damage_max, damage_cap, damage_type_str, attack_speed, range)) = row {
            if let Some(damage_type) = DamageType::from_str(&damage_type_str) {
                let weapon = Weapon {
                    damage_min,
                    damage_max,
                    damage_cap,
                    damage_type,
                    attack_speed,
                    range,
                };
                world
                    .insert_one(entity_id, weapon)
                    .map_err(|e| format!("Failed to add Weapon component: {}", e))?;
            }
        }

        Ok(())
    }

    /// Load Material component
    async fn load_material_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT material FROM wyldlands.entity_material WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load material component: {}", e))?;

        if let Some((material_str,)) = row {
            if let Some(material_kind) = MaterialKind::from_str(&material_str) {
                let material = Material { material_kind };
                world
                    .insert_one(entity_id, material)
                    .map_err(|e| format!("Failed to add Material component: {}", e))?;
            }
        }

        Ok(())
    }

    /// Load ArmorDefense component (multiple defense entries per entity)
    async fn load_armor_defense_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let rows: Vec<(String, i32)> = sqlx::query_as(
            "SELECT damage_kind::text, defense FROM wyldlands.entity_armor_defense WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to load armor defense component: {}", e))?;

        if !rows.is_empty() {
            let mut armor_defense = Armor::new();
            for (damage_kind_str, defense) in rows {
                if let Some(damage_kind) = DamageType::from_str(&damage_kind_str) {
                    armor_defense.set_defense(damage_kind, defense);
                }
            }
            world
                .insert_one(entity_id, armor_defense)
                .map_err(|e| format!("Failed to add ArmorDefense component: {}", e))?;
        }

        Ok(())
    }

    /// Load Commandable component
    async fn load_commandable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT max_queue_size FROM wyldlands.entity_commandable WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load commandable component: {}", e))?;

        if let Some((max_queue_size,)) = row {
            let commandable = Commandable {
                command_queue: Vec::new(),
                max_queue_size: max_queue_size as usize,
            };
            world
                .insert_one(entity_id, commandable)
                .map_err(|e| format!("Failed to add Commandable component: {}", e))?;
        }

        Ok(())
    }

    /// Load Interactable component (marker only)
    async fn load_interactable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &mut GameWorld,
    ) -> Result<(), String> {
        let exists: Option<(Uuid,)> = sqlx::query_as(
            "SELECT entity_id FROM wyldlands.entity_interactable WHERE entity_id = $1"
        )
        .bind(entity_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to load interactable component: {}", e))?;

        if exists.is_some() {
            let interactable = Interactable {
                interactions: Vec::new(),
            };
            world
                .insert_one(entity_id, interactable)
                .map_err(|e| format!("Failed to add Interactable component: {}", e))?;
        }

        Ok(())
    }

    /// Save any entity to database
    /// Saves all components attached to the entity (characters, rooms, objects, NPCs, etc.)
    pub async fn save_entity(
        &self,
        world: &GameWorld,
        entity_id: EcsEntity,
    ) -> Result<(), String> {
        // Get entity UUID
        let entity_uuid = world
            .get::<&EntityUuid>(entity_id)
            .map_err(|_| "Entity has no UUID")?;
        let uuid = entity_uuid.0;

        tracing::debug!("Saving entity {} to database", uuid);

        // Start a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        // Upsert base entity record
        sqlx::query(
            "INSERT INTO wyldlands.entities (uuid, created_at, updated_at)
             VALUES ($1, NOW(), NOW())
             ON CONFLICT (uuid)
             DO UPDATE SET updated_at = NOW()",
        )
        .bind(uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save entity record: {}", e))?;

        self.save_avatar_component(uuid, entity_id, world, &mut tx)
            .await?;
        // Save each component
        self.save_name_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_description_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_body_attributes_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_mind_attributes_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_soul_attributes_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_skills_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_location_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_combatant_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_equipment_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_ai_controller_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_personality_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_area_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_room_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_room_exits_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_container_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_containable_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_enterable_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_equipable_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_weapon_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_material_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_armor_defense_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_commandable_component(uuid, entity_id, world, &mut tx)
            .await?;
        self.save_interactable_component(uuid, entity_id, world, &mut tx)
            .await?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        // Remove from dirty set
        self.dirty_entities.write().await.remove(&uuid);

        tracing::info!("Saved entity {} successfully", uuid);
        Ok(())
    }

    /// Convenience method for saving character entities (alias for save_entity)
    pub async fn save_character(
        &self,
        world: &GameWorld,
        entity_id: EcsEntity,
    ) -> Result<(), String> {
        self.save_entity(world, entity_id).await
    }

    /// Save Name component
    async fn save_name_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(name) = world.get::<&Name>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET display = EXCLUDED.display, keywords = EXCLUDED.keywords",
            )
            .bind(entity_uuid)
            .bind(&name.display)
            .bind(&name.keywords)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save name component: {}", e))?;
        }
        Ok(())
    }

    /// Save Avatar component
    async fn save_avatar_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(avatar) = world.get::<&Avatar>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_avatars (entity_id, account_id, available, created_at)
                 VALUES ($1, $2, $3, NOW())
                 ON CONFLICT (account_id, entity_id)
                 DO UPDATE SET available = EXCLUDED.available"
            )
            .bind(entity_uuid)
            .bind(avatar.account_id)
            .bind(avatar.available)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save avatar component: {}", e))?;
        }
        Ok(())
    }

    /// Save Description component
    async fn save_description_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(desc) = world.get::<&Description>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_description (entity_id, short, long)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET short = EXCLUDED.short, long = EXCLUDED.long",
            )
            .bind(entity_uuid)
            .bind(&desc.short)
            .bind(&desc.long)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save description component: {}", e))?;
        }
        Ok(())
    }

    /// Save BodyAttributes component
    async fn save_body_attributes_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(attrs) = world.get::<&BodyAttributes>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_body_attributes 
                 (entity_id, score_offence, score_finesse, score_defence,
                  health_current, health_maximum, health_regen,
                  energy_current, energy_maximum, energy_regen)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET 
                    score_offence = EXCLUDED.score_offence,
                    score_finesse = EXCLUDED.score_finesse,
                    score_defence = EXCLUDED.score_defence,
                    health_current = EXCLUDED.health_current,
                    health_maximum = EXCLUDED.health_maximum,
                    health_regen = EXCLUDED.health_regen,
                    energy_current = EXCLUDED.energy_current,
                    energy_maximum = EXCLUDED.energy_maximum,
                    energy_regen = EXCLUDED.energy_regen",
            )
            .bind(entity_uuid)
            .bind(attrs.score_offence)
            .bind(attrs.score_finesse)
            .bind(attrs.score_defence)
            .bind(attrs.health_current)
            .bind(attrs.health_maximum)
            .bind(attrs.health_regen)
            .bind(attrs.energy_current)
            .bind(attrs.energy_maximum)
            .bind(attrs.energy_regen)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save body attributes component: {}", e))?;
        }
        Ok(())
    }

    /// Save MindAttributes component
    async fn save_mind_attributes_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(attrs) = world.get::<&MindAttributes>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_mind_attributes 
                 (entity_id, score_offence, score_finesse, score_defence,
                  health_current, health_maximum, health_regen,
                  energy_current, energy_maximum, energy_regen)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET 
                    score_offence = EXCLUDED.score_offence,
                    score_finesse = EXCLUDED.score_finesse,
                    score_defence = EXCLUDED.score_defence,
                    health_current = EXCLUDED.health_current,
                    health_maximum = EXCLUDED.health_maximum,
                    health_regen = EXCLUDED.health_regen,
                    energy_current = EXCLUDED.energy_current,
                    energy_maximum = EXCLUDED.energy_maximum,
                    energy_regen = EXCLUDED.energy_regen",
            )
            .bind(entity_uuid)
            .bind(attrs.score_offence)
            .bind(attrs.score_finesse)
            .bind(attrs.score_defence)
            .bind(attrs.health_current)
            .bind(attrs.health_maximum)
            .bind(attrs.health_regen)
            .bind(attrs.energy_current)
            .bind(attrs.energy_maximum)
            .bind(attrs.energy_regen)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save mind attributes component: {}", e))?;
        }
        Ok(())
    }

    /// Save SoulAttributes component
    async fn save_soul_attributes_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(attrs) = world.get::<&SoulAttributes>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_soul_attributes 
                 (entity_id, score_offence, score_finesse, score_defence,
                  health_current, health_maximum, health_regen,
                  energy_current, energy_maximum, energy_regen)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET 
                    score_offence = EXCLUDED.score_offence,
                    score_finesse = EXCLUDED.score_finesse,
                    score_defence = EXCLUDED.score_defence,
                    health_current = EXCLUDED.health_current,
                    health_maximum = EXCLUDED.health_maximum,
                    health_regen = EXCLUDED.health_regen,
                    energy_current = EXCLUDED.energy_current,
                    energy_maximum = EXCLUDED.energy_maximum,
                    energy_regen = EXCLUDED.energy_regen",
            )
            .bind(entity_uuid)
            .bind(attrs.score_offence)
            .bind(attrs.score_finesse)
            .bind(attrs.score_defence)
            .bind(attrs.health_current)
            .bind(attrs.health_maximum)
            .bind(attrs.health_regen)
            .bind(attrs.energy_current)
            .bind(attrs.energy_maximum)
            .bind(attrs.energy_regen)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save soul attributes component: {}", e))?;
        }
        Ok(())
    }

    /// Save Skills component
    async fn save_skills_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(skills) = world.get::<&Skills>(entity_id) {
            // Delete existing skills
            sqlx::query("DELETE FROM wyldlands.entity_skills WHERE entity_id = $1")
                .bind(entity_uuid)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to delete old skills: {}", e))?;

            // Insert all skills
            for (skill_id, skill) in &skills.skills {
                let skill_name = skill_id.to_string();
                let level = skills.level(*skill_id);
                sqlx::query(
                    "INSERT INTO wyldlands.entity_skills (entity_id, skill_name, level, experience, knowledge)
                     VALUES ($1, $2, $3, $4, $5)"
                )
                .bind(entity_uuid)
                .bind(&skill_name)
                .bind(level)
                .bind(skill.experience)
                .bind(skill.knowledge)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to save skill {}: {}", skill_name, e))?;
            }
        }
        Ok(())
    }

    /// Save Location component
    async fn save_location_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(loc) = world.get::<&Location>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_location (entity_id, room_id)
                 VALUES ($1, $2)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET room_id = EXCLUDED.room_id",
            )
            .bind(entity_uuid)
            .bind(loc.room_id.uuid())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save location component: {}", e))?;
        }
        Ok(())
    }

    /// Save Combatant component
    async fn save_combatant_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(combatant) = world.get::<&Combatant>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_combatant 
                 (entity_id, in_combat, target_id, initiative, action_cooldown, time_since_action)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET 
                    in_combat = EXCLUDED.in_combat,
                    target_id = EXCLUDED.target_id,
                    initiative = EXCLUDED.initiative,
                    action_cooldown = EXCLUDED.action_cooldown,
                    time_since_action = EXCLUDED.time_since_action",
            )
            .bind(entity_uuid)
            .bind(combatant.in_combat)
            .bind(combatant.target_id.map(|id| id.uuid()))
            .bind(combatant.initiative)
            .bind(combatant.action_cooldown)
            .bind(combatant.time_since_action)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save combatant component: {}", e))?;
        }
        Ok(())
    }

    /// Save Equipment component
    async fn save_equipment_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(equipment) = world.get::<&Equipment>(entity_id) {
            // Delete existing equipment
            sqlx::query("DELETE FROM wyldlands.entity_equipment WHERE entity_id = $1")
                .bind(entity_uuid)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to delete old equipment: {}", e))?;

            // Insert all equipped items
            for (slot, item_id) in &equipment.slots {
                sqlx::query(
                    "INSERT INTO wyldlands.entity_equipment (entity_id, slot, item_id, equipped_at)
                     VALUES ($1, $2, $3, NOW())",
                )
                .bind(entity_uuid)
                .bind(slot.as_str())
                .bind(item_id.uuid())
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to save equipment slot {:?}: {}", slot, e))?;
            }
        }
        Ok(())
    }

    /// Save AIController component
    async fn save_ai_controller_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(ai) = world.get::<&AIController>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_ai_controller 
                 (entity_id, behavior_type, current_goal, state_type, state_target_id, update_interval, time_since_update)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET 
                    behavior_type = EXCLUDED.behavior_type,
                    current_goal = EXCLUDED.current_goal,
                    state_type = EXCLUDED.state_type,
                    state_target_id = EXCLUDED.state_target_id,
                    update_interval = EXCLUDED.update_interval,
                    time_since_update = EXCLUDED.time_since_update"
            )
            .bind(entity_uuid)
            .bind(ai.behavior_type.as_str())
            .bind(&ai.current_goal)
            .bind(ai.state_type.as_str())
            .bind(ai.state_target_id.map(|id| id.uuid()))
            .bind(ai.update_interval)
            .bind(ai.time_since_update)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save AI controller component: {}", e))?;
        }
        Ok(())
    }

    /// Save Personality component
    async fn save_personality_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(personality) = world.get::<&Personality>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_personality (entity_id, background, speaking_style)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET background = EXCLUDED.background, speaking_style = EXCLUDED.speaking_style"
            )
            .bind(entity_uuid)
            .bind(&personality.background)
            .bind(&personality.speaking_style)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save personality component: {}", e))?;
        }
        Ok(())
    }

    /// Save Area component
    async fn save_area_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(area) = world.get::<&Area>(entity_id) {
            let area_kind_str = match area.area_kind {
                AreaKind::Overworld => "Overworld",
                AreaKind::Vehicle => "Vehicle",
                AreaKind::Building => "Building",
                AreaKind::Dungeon => "Dungeon",
            };

            sqlx::query(
                "INSERT INTO wyldlands.entity_areas (entity_id, area_kind, area_flags)
                 VALUES ($1, $2::area_kind, $3)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET area_kind = EXCLUDED.area_kind, area_flags = EXCLUDED.area_flags"
            )
            .bind(entity_uuid)
            .bind(area_kind_str)
            .bind(&area.area_flags)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save area component: {}", e))?;
        }
        Ok(())
    }

    /// Save Room component
    async fn save_room_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(room) = world.get::<&Room>(entity_id) {
            let room_flags_strs: Vec<String> = room.room_flags
                .iter()
                .map(|flag| match flag {
                    RoomFlag::Breathable => "Breathable".to_string(),
                })
                .collect();

            sqlx::query(
                "INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
                 VALUES ($1, $2, $3::room_flag[])
                 ON CONFLICT (entity_id)
                 DO UPDATE SET area_id = EXCLUDED.area_id, room_flags = EXCLUDED.room_flags"
            )
            .bind(entity_uuid)
            .bind(room.area_id.uuid())
            .bind(&room_flags_strs)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save room component: {}", e))?;
        }
        Ok(())
    }

    /// Save room exits (single Exits component to multiple database rows)
    async fn save_room_exits_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        // Delete existing exits
        sqlx::query("DELETE FROM wyldlands.entity_room_exits WHERE entity_id = $1")
            .bind(entity_uuid)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to delete old room exits: {}", e))?;

        // Save all exits from the Exits component
        if let Ok(exits) = world.get::<&Exits>(entity_id) {
            for exit in &exits.exits {
                sqlx::query(
                    "INSERT INTO wyldlands.entity_room_exits
                     (entity_id, dest_id, direction, closeable, closed, door_rating, lockable, locked, unlock_code, lock_rating, transparent)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
                )
                .bind(entity_uuid)
                .bind(exit.dest_id.uuid())
                .bind(&exit.direction)
                .bind(exit.closeable)
                .bind(exit.closed)
                .bind(exit.door_rating)
                .bind(exit.lockable)
                .bind(exit.locked)
                .bind(&exit.unlock_code)
                .bind(exit.lock_rating)
                .bind(exit.transparent)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to save room exit {}: {}", exit.direction, e))?;
            }
        }

        Ok(())
    }

    /// Save Container component
    async fn save_container_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(container) = world.get::<&Container>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_container
                 (entity_id, capacity, max_weight, closeable, closed, container_rating, lockable, locked, unlock_code, lock_rating, transparent)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET
                    capacity = EXCLUDED.capacity,
                    max_weight = EXCLUDED.max_weight,
                    closeable = EXCLUDED.closeable,
                    closed = EXCLUDED.closed,
                    container_rating = EXCLUDED.container_rating,
                    lockable = EXCLUDED.lockable,
                    locked = EXCLUDED.locked,
                    unlock_code = EXCLUDED.unlock_code,
                    lock_rating = EXCLUDED.lock_rating,
                    transparent = EXCLUDED.transparent"
            )
            .bind(entity_uuid)
            .bind(container.capacity)
            .bind(container.max_weight)
            .bind(container.closeable)
            .bind(container.closed)
            .bind(container.container_rating)
            .bind(container.lockable)
            .bind(container.locked)
            .bind(&container.unlock_code)
            .bind(container.lock_rating)
            .bind(container.transparent)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save container component: {}", e))?;
        }
        Ok(())
    }

    /// Save Containable component
    async fn save_containable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(containable) = world.get::<&Containable>(entity_id) {
            let size_str = match containable.size {
                Size::Tiny => "Tiny",
                Size::Small => "Small",
                Size::Medium => "Medium",
                Size::Large => "Large",
                Size::Huge => "Huge",
            };

            sqlx::query(
                "INSERT INTO wyldlands.entity_containable (entity_id, weight, size, stackable, stack_size)
                 VALUES ($1, $2, $3::size_class, $4, $5)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET weight = EXCLUDED.weight, size = EXCLUDED.size, stackable = EXCLUDED.stackable, stack_size = EXCLUDED.stack_size"
            )
            .bind(entity_uuid)
            .bind(containable.weight)
            .bind(size_str)
            .bind(containable.stackable)
            .bind(containable.stack_size)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save containable component: {}", e))?;
        }
        Ok(())
    }

    /// Save Enterable component
    async fn save_enterable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(enterable) = world.get::<&Enterable>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_enterable
                 (entity_id, dest_id, closeable, closed, door_rating, lockable, locked, unlock_code, lock_rating, transparent)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET
                    dest_id = EXCLUDED.dest_id,
                    closeable = EXCLUDED.closeable,
                    closed = EXCLUDED.closed,
                    door_rating = EXCLUDED.door_rating,
                    lockable = EXCLUDED.lockable,
                    locked = EXCLUDED.locked,
                    unlock_code = EXCLUDED.unlock_code,
                    lock_rating = EXCLUDED.lock_rating,
                    transparent = EXCLUDED.transparent"
            )
            .bind(entity_uuid)
            .bind(enterable.dest_id.uuid())
            .bind(enterable.closeable)
            .bind(enterable.closed)
            .bind(enterable.door_rating)
            .bind(enterable.lockable)
            .bind(enterable.locked)
            .bind(&enterable.unlock_code)
            .bind(enterable.lock_rating)
            .bind(enterable.transparent)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save enterable component: {}", e))?;
        }
        Ok(())
    }

    /// Save Equipable component
    async fn save_equipable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(equipable) = world.get::<&Equipable>(entity_id) {
            let slot_strs: Vec<String> = equipable.slots.iter().map(|s| s.as_str().to_string()).collect();

            sqlx::query(
                "INSERT INTO wyldlands.entity_equipable (entity_id, slots)
                 VALUES ($1, $2::slot_kind[])
                 ON CONFLICT (entity_id)
                 DO UPDATE SET slots = EXCLUDED.slots"
            )
            .bind(entity_uuid)
            .bind(&slot_strs)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save equipable component: {}", e))?;
        }
        Ok(())
    }

    /// Save Weapon component
    async fn save_weapon_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(weapon) = world.get::<&Weapon>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_weapon
                 (entity_id, damage_min, damage_max, damage_cap, damage_type, attack_speed, range)
                 VALUES ($1, $2, $3, $4, $5::damage_type, $6, $7)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET
                    damage_min = EXCLUDED.damage_min,
                    damage_max = EXCLUDED.damage_max,
                    damage_cap = EXCLUDED.damage_cap,
                    damage_type = EXCLUDED.damage_type,
                    attack_speed = EXCLUDED.attack_speed,
                    range = EXCLUDED.range"
            )
            .bind(entity_uuid)
            .bind(weapon.damage_min)
            .bind(weapon.damage_max)
            .bind(weapon.damage_cap)
            .bind(weapon.damage_type.as_str())
            .bind(weapon.attack_speed)
            .bind(weapon.range)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save weapon component: {}", e))?;
        }
        Ok(())
    }

    /// Save Material component
    async fn save_material_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(material) = world.get::<&Material>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_material (entity_id, material)
                 VALUES ($1, $2)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET material = EXCLUDED.material"
            )
            .bind(entity_uuid)
            .bind(material.material_kind.as_str())
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save material component: {}", e))?;
        }
        Ok(())
    }

    /// Save ArmorDefense component (multiple defense entries per entity)
    async fn save_armor_defense_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(armor_defense) = world.get::<&Armor>(entity_id) {
            // Delete existing defenses
            sqlx::query("DELETE FROM wyldlands.entity_armor_defense WHERE entity_id = $1")
                .bind(entity_uuid)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to delete old armor defenses: {}", e))?;

            // Insert all defense entries
            for (damage_kind, defense) in &armor_defense.defenses {
                sqlx::query(
                    "INSERT INTO wyldlands.entity_armor_defense (entity_id, damage_kind, defense)
                     VALUES ($1, $2::damage_type, $3)"
                )
                .bind(entity_uuid)
                .bind(damage_kind.as_str())
                .bind(defense)
                .execute(&mut **tx)
                .await
                .map_err(|e| format!("Failed to save armor defense for {:?}: {}", damage_kind, e))?;
            }
        }
        Ok(())
    }

    /// Save Commandable component
    async fn save_commandable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if let Ok(commandable) = world.get::<&Commandable>(entity_id) {
            sqlx::query(
                "INSERT INTO wyldlands.entity_commandable (entity_id, max_queue_size)
                 VALUES ($1, $2)
                 ON CONFLICT (entity_id)
                 DO UPDATE SET max_queue_size = EXCLUDED.max_queue_size"
            )
            .bind(entity_uuid)
            .bind(commandable.max_queue_size as i32)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save commandable component: {}", e))?;
        }
        Ok(())
    }

    /// Save Interactable component (marker only)
    async fn save_interactable_component(
        &self,
        entity_uuid: Uuid,
        entity_id: EcsEntity,
        world: &GameWorld,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), String> {
        if world.get::<&Interactable>(entity_id).is_ok() {
            sqlx::query(
                "INSERT INTO wyldlands.entity_interactable (entity_id)
                 VALUES ($1)
                 ON CONFLICT (entity_id) DO NOTHING"
            )
            .bind(entity_uuid)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Failed to save interactable component: {}", e))?;
        }
        Ok(())
    }

    /// Set avatar availability (enable/disable character)
    pub async fn set_avatar_available(
        &self,
        entity_uuid: Uuid,
        available: bool,
    ) -> Result<(), String> {
        sqlx::query("UPDATE wyldlands.entity_avatars SET available = $1 WHERE entity_id = $2")
            .bind(available)
            .bind(entity_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update avatar availability: {}", e))?;

        Ok(())
    }

    /// Get list of avatars for an account
    pub async fn get_account_avatars(&self, account_id: Uuid) -> Result<Vec<(Uuid, bool)>, String> {
        let rows: Vec<(Uuid, bool)> = sqlx::query_as(
            "SELECT entity_id, available FROM wyldlands.entity_avatars WHERE account_id = $1 ORDER BY last_played DESC NULLS LAST"
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get account avatars: {}", e))?;

        Ok(rows)
    }
    /// Mark an entity as dirty (needs saving)
    pub async fn mark_dirty(&self, entity_uuid: Uuid) {
        self.dirty_entities.write().await.insert(entity_uuid);
    }

    /// Get all dirty entity UUIDs
    pub async fn get_dirty_entities(&self) -> Vec<Uuid> {
        self.dirty_entities.read().await.iter().copied().collect()
    }

    /// Clear all dirty entities (useful after a successful save)
    pub async fn clear_dirty(&self) {
        self.dirty_entities.write().await.clear();
    }

    /// Get the count of dirty entities
    pub async fn dirty_count(&self) -> usize {
        self.dirty_entities.read().await.len()
    }

    /// Check if a specific entity is dirty
    pub async fn is_dirty(&self, entity_uuid: Uuid) -> bool {
        self.dirty_entities.read().await.contains(&entity_uuid)
    }

    /// Auto-save all dirty entities
    pub async fn auto_save(&self, world: &GameWorld) -> Result<usize, String> {
        let dirty_uuids = self.get_dirty_entities().await;

        if dirty_uuids.is_empty() {
            return Ok(0);
        }

        tracing::info!("Auto-saving {} dirty entities", dirty_uuids.len());

        let mut saved_count = 0;

        for entity_uuid in dirty_uuids {
            // Find entity by UUID in the world
            let mut found_entity = None;
            for (entity_id, entity_uuid_comp) in world.query::<(Entity, &EntityUuid)>().iter() {
                if entity_uuid_comp.0 == entity_uuid {
                    found_entity = Some(entity_id);
                    break;
                }
            }

            if let Some(entity_id) = found_entity {
                // Save the entity
                match self.save_character(world, entity_id).await {
                    Ok(_) => {
                        saved_count += 1;
                    }
                    Err(e) => {
                        tracing::error!("Failed to save entity {}: {}", entity_uuid, e);
                    }
                }
            } else {
                tracing::warn!("Dirty entity {} not found in world", entity_uuid);
                self.dirty_entities.write().await.remove(&entity_uuid);
            }
        }

        tracing::info!("Auto-save completed: {} entities saved", saved_count);
        Ok(saved_count)
    }

    /// Delete an entity from database
    pub async fn delete_entity(&self, entity_uuid: Uuid) -> Result<(), String> {
        tracing::info!("Deleting entity {} from database", entity_uuid);

        // CASCADE will handle component tables
        sqlx::query("DELETE FROM wyldlands.entities WHERE uuid = $1")
            .bind(entity_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete entity: {}", e))?;

        self.dirty_entities.write().await.remove(&entity_uuid);

        Ok(())
    }

    /// Get auto-save interval
    pub fn auto_save_interval(&self) -> u64 {
        self.auto_save_interval
    }

    /// Update last_played timestamp for an avatar
    pub async fn update_last_played(&self, entity_uuid: Uuid) -> Result<(), String> {
        sqlx::query("UPDATE wyldlands.entity_avatars SET last_played = NOW() WHERE entity_id = $1")
            .bind(entity_uuid)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to update last_played: {}", e))?;

        Ok(())
    }

    /// Start auto-save task
    pub fn start_auto_save_task(self: Arc<Self>, world: Arc<RwLock<GameWorld>>) {
        let interval = self.auto_save_interval;

        tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_secs(interval));

            loop {
                interval_timer.tick().await;

                let world_guard = world.read().await;
                if let Err(e) = self.auto_save(&world_guard).await {
                    tracing::error!("Auto-save failed: {}", e);
                }
            }
        });

        tracing::info!("Auto-save task started (interval: {}s)", interval);
    }

    // ========== EntityId Convenience Methods ==========

    /// Load an entity and return an EntityId combining both the ECS entity and UUID
    ///
    /// This is a convenience method that loads an entity from the database and
    /// automatically creates an EntityId with both the runtime ECS handle and
    /// persistent UUID for easy use throughout systems.
    pub async fn load_entity_with_id(
        &self,
        world: &mut GameWorld,
        registry: &mut EntityRegistry,
        entity_uuid: Uuid,
    ) -> Result<EntityId, String> {
        let ecs_entity = self.load_entity(world, registry, entity_uuid).await?;
        registry.register(ecs_entity, entity_uuid)
            .map_err(|e| format!("Failed to register entity in registry: {}", e))?;
        Ok(EntityId::new(ecs_entity, entity_uuid))
    }

    /// Save an entity using an EntityId
    ///
    /// Convenience method that accepts an EntityId and saves the entity to the database.
    pub async fn save_entity_by_id(
        &self,
        world: &GameWorld,
        entity_id: EntityId,
    ) -> Result<(), String> {
        self.save_entity(world, entity_id.entity()).await
    }

    /// Mark an entity as dirty using EntityId
    pub async fn mark_dirty_by_id(&self, entity_id: EntityId) {
        self.mark_dirty(entity_id.uuid()).await
    }
}

impl PersistenceManager {
    /// Create a mock persistence manager for testing (no database connection)
    #[cfg(test)]
    pub fn new_mock() -> Self {
        // Create a mock PgPool - we use an empty connect_options which won't actually connect
        // but satisfies the type system for testing
        use sqlx::postgres::PgPoolOptions;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://mock:mock@localhost/mock")
            .expect("Failed to create mock pool");

        Self {
            pool,
            dirty_entities: Arc::new(RwLock::new(HashSet::new())),
            auto_save_interval: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_manager_creation() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}



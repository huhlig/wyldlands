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

//! Persistence system for saving and loading entities

use crate::ecs::{GameWorld, EcsEntity};
use crate::ecs::components::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEntity {
    pub uuid: EntityUuid,
    pub components: HashMap<String, serde_json::Value>,
}

pub struct PersistenceSystem {
    dirty_entities: Vec<EcsEntity>,
}

impl PersistenceSystem {
    /// Create a new persistence system
    pub fn new() -> Self {
        Self {
            dirty_entities: Vec::new(),
        }
    }
    
    /// Mark an entity as dirty (needs to be saved)
    pub fn mark_dirty(&mut self, entity: EcsEntity) {
        if !self.dirty_entities.contains(&entity) {
            self.dirty_entities.push(entity);
        }
    }
    
    /// Serialize an entity to a portable format
    pub fn serialize_entity(&self, world: &GameWorld, entity: EcsEntity) -> Result<SerializedEntity, String> {
        let mut components = HashMap::new();
        
        // Get UUID (required for persistence)
        let uuid = world.get::<&EntityUuid>(entity)
            .map_err(|_| "Entity has no UUID")?;
        
        // Serialize all components (dereference to get actual value)
        if let Ok(name) = world.get::<&Name>(entity) {
            components.insert("name".to_string(), serde_json::to_value(&*name).unwrap());
        }
        
        if let Ok(desc) = world.get::<&Description>(entity) {
            components.insert("description".to_string(), serde_json::to_value(&*desc).unwrap());
        }
        
        if let Ok(entity_type) = world.get::<&EntityType>(entity) {
            components.insert("entity_type".to_string(), serde_json::to_value(&*entity_type).unwrap());
        }
        
        if let Ok(loc) = world.get::<&Location>(entity) {
            components.insert("location".to_string(), serde_json::to_value(&*loc).unwrap());
        }
        
        if let Ok(container) = world.get::<&Container>(entity) {
            components.insert("container".to_string(), serde_json::to_value(&*container).unwrap());
        }
        
        if let Ok(containable) = world.get::<&Containable>(entity) {
            components.insert("containable".to_string(), serde_json::to_value(&*containable).unwrap());
        }

        if let Ok(attrs) = world.get::<&BodyAttributes>(entity) {
            components.insert("body_attributes".to_string(), serde_json::to_value(&*attrs).unwrap());
        }

        if let Ok(attrs) = world.get::<&MindAttributes>(entity) {
            components.insert("mind_attributes".to_string(), serde_json::to_value(&*attrs).unwrap());
        }

        if let Ok(attrs) = world.get::<&SoulAttributes>(entity) {
            components.insert("soul_attributes".to_string(), serde_json::to_value(&*attrs).unwrap());
        }

        if let Ok(skills) = world.get::<&Skills>(entity) {
            components.insert("skills".to_string(), serde_json::to_value(&*skills).unwrap());
        }
        
        if let Ok(ai) = world.get::<&AIController>(entity) {
            components.insert("ai_controller".to_string(), serde_json::to_value(&*ai).unwrap());
        }
        
        if let Ok(personality) = world.get::<&Personality>(entity) {
            components.insert("personality".to_string(), serde_json::to_value(&*personality).unwrap());
        }
        
        if let Ok(memory) = world.get::<&Memory>(entity) {
            components.insert("memory".to_string(), serde_json::to_value(&*memory).unwrap());
        }
        
        if let Ok(combatant) = world.get::<&Combatant>(entity) {
            components.insert("combatant".to_string(), serde_json::to_value(&*combatant).unwrap());
        }
        
        if let Ok(equipment) = world.get::<&Equipment>(entity) {
            components.insert("equipment".to_string(), serde_json::to_value(&*equipment).unwrap());
        }
        
        Ok(SerializedEntity {
            uuid: *uuid,
            components,
        })
    }
    
    /// Deserialize an entity from a portable format
    pub fn deserialize_entity(&self, world: &mut GameWorld, serialized: SerializedEntity) -> Result<EcsEntity, String> {
        // Create entity with UUID
        let entity = world.spawn((serialized.uuid,));
        
        // Deserialize all components
        for (component_name, value) in serialized.components {
            match component_name.as_str() {
                "name" => {
                    if let Ok(name) = serde_json::from_value::<Name>(value) {
                        world.insert_one(entity, name).ok();
                    }
                }
                "description" => {
                    if let Ok(desc) = serde_json::from_value::<Description>(value) {
                        world.insert_one(entity, desc).ok();
                    }
                }
                "entity_type" => {
                    if let Ok(et) = serde_json::from_value::<EntityType>(value) {
                        world.insert_one(entity, et).ok();
                    }
                }
                "location" => {
                    if let Ok(loc) = serde_json::from_value::<Location>(value) {
                        world.insert_one(entity, loc).ok();
                    }
                }
                "container" => {
                    if let Ok(container) = serde_json::from_value::<Container>(value) {
                        world.insert_one(entity, container).ok();
                    }
                }
                "containable" => {
                    if let Ok(containable) = serde_json::from_value::<Containable>(value) {
                        world.insert_one(entity, containable).ok();
                    }
                }
                "body_attributes" => {
                    if let Ok(attrs) = serde_json::from_value::<BodyAttributes>(value) {
                        world.insert_one(entity, attrs).ok();
                    }
                }
                "mind_attributes" => {
                    if let Ok(attrs) = serde_json::from_value::<MindAttributes>(value) {
                        world.insert_one(entity, attrs).ok();
                    }
                }
                "soul_attributes" => {
                    if let Ok(attrs) = serde_json::from_value::<SoulAttributes>(value) {
                        world.insert_one(entity, attrs).ok();
                    }
                }
                "skills" => {
                    if let Ok(skills) = serde_json::from_value::<Skills>(value) {
                        world.insert_one(entity, skills).ok();
                    }
                }
                "ai_controller" => {
                    if let Ok(ai) = serde_json::from_value::<AIController>(value) {
                        world.insert_one(entity, ai).ok();
                    }
                }
                "personality" => {
                    if let Ok(personality) = serde_json::from_value::<Personality>(value) {
                        world.insert_one(entity, personality).ok();
                    }
                }
                "memory" => {
                    if let Ok(memory) = serde_json::from_value::<Memory>(value) {
                        world.insert_one(entity, memory).ok();
                    }
                }
                "combatant" => {
                    if let Ok(combatant) = serde_json::from_value::<Combatant>(value) {
                        world.insert_one(entity, combatant).ok();
                    }
                }
                "equipment" => {
                    if let Ok(equipment) = serde_json::from_value::<Equipment>(value) {
                        world.insert_one(entity, equipment).ok();
                    }
                }
                _ => {}
            }
        }
        
        Ok(entity)
    }
    
    /// Save an entity to JSON string
    pub fn save_to_json(&self, world: &GameWorld, entity: EcsEntity) -> Result<String, String> {
        let serialized = self.serialize_entity(world, entity)?;
        serde_json::to_string_pretty(&serialized)
            .map_err(|e| format!("JSON serialization error: {}", e))
    }
    
    /// Load an entity from JSON string
    pub fn load_from_json(&self, world: &mut GameWorld, json: &str) -> Result<EcsEntity, String> {
        let serialized: SerializedEntity = serde_json::from_str(json)
            .map_err(|e| format!("JSON deserialization error: {}", e))?;
        self.deserialize_entity(world, serialized)
    }
    
    /// Get all dirty entities
    pub fn get_dirty_entities(&self) -> &[EcsEntity] {
        &self.dirty_entities
    }
    
    /// Clear dirty entities list
    pub fn clear_dirty(&mut self) {
        self.dirty_entities.clear();
    }
    
    /// Find entity by UUID
    pub fn find_by_uuid(&self, world: &GameWorld, uuid: EntityUuid) -> Option<EcsEntity> {
        for (entity, entity_uuid) in world.query::<&EntityUuid>().iter() {
            if *entity_uuid == uuid {
                return Some(entity);
            }
        }
        None
    }
}

impl Default for PersistenceSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_persistence_system_creation() {
        let _system = PersistenceSystem::new();
    }
    
    #[test]
    fn test_mark_dirty() {
        let mut world = GameWorld::new();
        let mut system = PersistenceSystem::new();
        let entity = world.spawn(());

        system.mark_dirty(entity);
        assert_eq!(system.get_dirty_entities().len(), 1);

        // Marking again shouldn't duplicate
        system.mark_dirty(entity);
        assert_eq!(system.get_dirty_entities().len(), 1);
    }
    
    #[test]
    fn test_serialize_deserialize() {
        let mut world = GameWorld::new();
        let system = PersistenceSystem::new();

        let uuid = EntityUuid::new();
        let entity = world.spawn((
            uuid,
            Name::new("Test Entity"),
            Description::new("Short desc", "Long description"),
            Location::new(
                EntityId::from_uuid(uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap()),
                EntityId::from_uuid(uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap()),
            ),
            BodyAttributes::new(),
        ));

        // Serialize
        let serialized = system.serialize_entity(&world, entity).unwrap();
        assert_eq!(serialized.uuid, uuid);
        assert!(serialized.components.contains_key("name"));
        assert!(serialized.components.contains_key("location"));
        assert!(serialized.components.contains_key("body_attributes"));

        // Deserialize into new world
        let mut new_world = GameWorld::new();
        let new_entity = system.deserialize_entity(&mut new_world, serialized).unwrap();

        // Verify components
        let name = new_world.get::<&Name>(new_entity).unwrap();
        assert_eq!(name.display, "Test Entity");

        let loc = new_world.get::<&Location>(new_entity).unwrap();
        assert_eq!(loc.area_id.uuid(), uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap());
        assert_eq!(loc.room_id.uuid(), uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap());

        let attrs = new_world.get::<&BodyAttributes>(new_entity).unwrap();
        assert_eq!(attrs.health_maximum, 100.0);
    }
    
    #[test]
    fn test_json_serialization() {
        let mut world = GameWorld::new();
        let system = PersistenceSystem::new();

        let entity = world.spawn((
            EntityUuid::new(),
            Name::new("JSON Test"),
            BodyAttributes::new(),
        ));

        // Save to JSON
        let json = system.save_to_json(&world, entity).unwrap();
        assert!(json.contains("JSON Test"));

        // Load from JSON
        let mut new_world = GameWorld::new();
        let new_entity = system.load_from_json(&mut new_world, &json).unwrap();

        let name = new_world.get::<&Name>(new_entity).unwrap();
        assert_eq!(name.display, "JSON Test");
    }
    
    #[test]
    fn test_find_by_uuid() {
        let mut world = GameWorld::new();
        let system = PersistenceSystem::new();
        
        let uuid = EntityUuid::new();
        let entity = world.spawn((
            uuid,
            Name::new("Findable"),
        ));
        
        let found = system.find_by_uuid(&world, uuid);
        assert_eq!(found, Some(entity));
        
        let not_found = system.find_by_uuid(&world, EntityUuid::new());
        assert_eq!(not_found, None);
    }
    
    #[test]
    fn test_complex_entity_serialization() {
        let mut world = GameWorld::new();
        let system = PersistenceSystem::new();

        let entity = world.spawn((
            EntityUuid::new(),
            Name::new("Complex Entity"),
            Location::new(
                EntityId::from_uuid(uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000000").unwrap()),
                EntityId::from_uuid(uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap()),
            ),
            BodyAttributes::new(),
            MindAttributes::new(),
            SoulAttributes::new(),
            Skills::new(),
            Combatant::new(),
            Equipment::new(),
        ));

        let json = system.save_to_json(&world, entity).unwrap();

        let mut new_world = GameWorld::new();
        let new_entity = system.load_from_json(&mut new_world, &json).unwrap();

        // Verify all components exist
        assert!(new_world.get::<&Name>(new_entity).is_ok());
        assert!(new_world.get::<&Location>(new_entity).is_ok());
        assert!(new_world.get::<&BodyAttributes>(new_entity).is_ok());
        assert!(new_world.get::<&MindAttributes>(new_entity).is_ok());
        assert!(new_world.get::<&SoulAttributes>(new_entity).is_ok());
        assert!(new_world.get::<&Skills>(new_entity).is_ok());
        assert!(new_world.get::<&Combatant>(new_entity).is_ok());
        assert!(new_world.get::<&Equipment>(new_entity).is_ok());
    }
}



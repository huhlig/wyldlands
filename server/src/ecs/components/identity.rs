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

//! Identity components for entity identification and description

use serde::{Deserialize, Serialize};

/// Combined entity identifier containing both ECS runtime handle and persistent UUID
///
/// This struct provides convenient access to both the transient hecs::Entity ID
/// (which changes between server restarts) and the persistent database UUID
/// (which remains constant across restarts).
///
/// Note: Only the UUID is serialized since the ECS entity handle is transient.
/// The entity handle must be restored from the EntityRegistry after deserialization.
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct EntityId {
    /// Runtime ECS entity handle (transient, changes on restart)
    #[serde(skip)]
    #[serde(default = "EntityId::default_entity")]
    pub entity: hecs::Entity,
    /// Persistent database UUID (constant across restarts)
    pub uuid: uuid::Uuid,
}

impl EntityId {
    /// Default entity value for deserialization
    fn default_entity() -> hecs::Entity {
        // Use a sentinel value that can be detected and replaced
        unsafe { hecs::Entity::from_bits(u64::MAX).unwrap_unchecked() }
    }

    /// Create an EntityId with just a UUID (entity will need to be filled in later)
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self {
            entity: Self::default_entity(),
            uuid,
        }
    }

    /// Check if this EntityId needs its entity handle resolved
    pub fn needs_resolution(&self) -> bool {
        self.entity.to_bits().get() == u64::MAX
    }

    /// Update the entity handle (used after loading from persistence)
    pub fn set_entity(&mut self, entity: hecs::Entity) {
        self.entity = entity;
    }

    /// Create a new EntityId from an ECS entity and UUID
    pub fn new(entity: hecs::Entity, uuid: uuid::Uuid) -> Self {
        Self { entity, uuid }
    }

    /// Get the runtime ECS entity handle
    pub fn entity(&self) -> hecs::Entity {
        self.entity
    }

    /// Get the persistent UUID
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
}

impl From<EntityId> for hecs::Entity {
    fn from(id: EntityId) -> Self {
        id.entity
    }
}

impl From<EntityId> for uuid::Uuid {
    fn from(id: EntityId) -> Self {
        id.uuid
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

impl PartialEq for EntityId {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl PartialOrd for EntityId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntityId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.uuid.cmp(&other.uuid)
    }
}

impl Eq for EntityId {}

/// Unique identifier for entities that need persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityUuid(pub uuid::Uuid);

impl EntityUuid {
    /// Create a new random UUID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EntityUuid {
    fn default() -> Self {
        Self::new()
    }
}

/// Display name and keywords for entity identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name {
    /// Primary display name
    pub display: String,
    /// Keywords for matching (e.g., "sword", "rusty", "blade")
    pub keywords: Vec<String>,
}

impl Name {
    /// Create a new name with the display name as the only keyword
    pub fn new(display: impl Into<String>) -> Self {
        let display = display.into();
        let keywords = vec![display.to_lowercase()];
        Self { display, keywords }
    }

    /// Add custom keywords for matching
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords.into_iter().map(|k| k.to_lowercase()).collect();
        self
    }

    /// Check if this name matches a given keyword
    pub fn matches(&self, keyword: &str) -> bool {
        let keyword = keyword.to_lowercase();
        self.keywords.iter().any(|k| k.starts_with(&keyword))
    }
}

/// Entity descriptions at various detail levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    /// Brief description (one line)
    pub short: String,
    /// Detailed description (multiple paragraphs)
    pub long: String,
}

impl Description {
    /// Create a new description
    pub fn new(short: impl Into<String>, long: impl Into<String>) -> Self {
        Self {
            short: short.into(),
            long: long.into(),
        }
    }

    /// Get the short description
    pub fn get_short(&self) -> &str {
        &self.short
    }

    /// Get the long description
    pub fn get_long(&self) -> &str {
        &self.long
    }
}

/// Avatar component - links player character to account
/// Maps to: entity_avatars table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avatar {
    /// Account ID that owns this avatar
    pub account_id: uuid::Uuid,
    /// Whether this avatar is available for play
    pub available: bool,
}

impl Avatar {
    /// Create a new avatar linked to an account
    pub fn new(account_id: uuid::Uuid) -> Self {
        Self {
            account_id,
            available: true,
        }
    }

    /// Check if avatar is available for play
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Enable or disable the avatar
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }
}

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    NPC,
    Item,
    Room,
    Exit,
    Container,
    Vehicle,
    Projectile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_creation() {
        let mut world = hecs::World::new();
        let entity = world.spawn(());
        let uuid = uuid::Uuid::new_v4();
        let entity_id = EntityId::new(entity, uuid);

        assert_eq!(entity_id.entity(), entity);
        assert_eq!(entity_id.uuid(), uuid);
    }

    #[test]
    fn test_entity_id_conversions() {
        let mut world = hecs::World::new();
        let entity = world.spawn(());
        let uuid = uuid::Uuid::new_v4();
        let entity_id = EntityId::new(entity, uuid);

        let extracted_entity: hecs::Entity = entity_id.into();
        let extracted_uuid: uuid::Uuid = entity_id.into();

        assert_eq!(extracted_entity, entity);
        assert_eq!(extracted_uuid, uuid);
    }

    #[test]
    fn test_name_matching() {
        let name = Name::new("Rusty Sword").with_keywords(vec![
            "rusty".into(),
            "sword".into(),
            "blade".into(),
        ]);

        assert!(name.matches("rus"));
        assert!(name.matches("sword"));
        assert!(name.matches("bla"));
        assert!(!name.matches("axe"));
    }

    #[test]
    fn test_entity_uuid_uniqueness() {
        let id1 = EntityUuid::new();
        let id2 = EntityUuid::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_description() {
        let desc = Description::new(
            "A rusty sword",
            "This is a long, detailed description of a rusty sword.",
        );

        assert_eq!(desc.get_short(), "A rusty sword");
        assert!(desc.get_long().contains("detailed"));
    }
}

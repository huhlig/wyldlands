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

//! Spatial components for positioning and containment

use super::EntityId;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use uuid::Uuid;

/// Location of an entity in the world
/// Maps to: entity_location table
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub area_id: EntityId,
    pub room_id: EntityId,
}

impl Location {
    pub fn new(area_id: EntityId, room_id: EntityId) -> Self {
        Self { area_id, room_id }
    }
}

/// Area types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AreaKind {
    Overworld,
    Vehicle,
    Building,
    Dungeon,
}

/// Area component
/// Maps to: entity_areas table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub area_kind: AreaKind,
    pub area_flags: Vec<String>,
}

impl Area {
    pub fn new(area_kind: AreaKind) -> Self {
        Self {
            area_kind,
            area_flags: Vec::new(),
        }
    }
}

/// Room flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomFlag {
    Breathable,
}

/// Room component
/// Maps to: entity_rooms table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub area_id: EntityId,
    pub room_flags: Vec<RoomFlag>,
}

impl Room {
    pub fn new(area_id: EntityId) -> Self {
        Self {
            area_id,
            room_flags: vec![RoomFlag::Breathable],
        }
    }
}

/// Container for holding other entities
/// Maps to: entity_container table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub capacity: Option<i32>,
    pub max_weight: Option<f32>,
    pub closeable: bool,
    pub closed: bool,
    pub container_rating: Option<i32>,
    pub lockable: bool,
    pub locked: bool,
    pub unlock_code: Option<String>,
    pub lock_rating: Option<i32>,
    pub transparent: bool,
}

impl Container {
    /// Create a new container with optional capacity
    pub fn new(capacity: Option<i32>) -> Self {
        Self {
            capacity,
            max_weight: None,
            closeable: false,
            closed: false,
            container_rating: None,
            lockable: false,
            locked: false,
            unlock_code: None,
            lock_rating: None,
            transparent: false,
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Size categories for containable items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
}

/// Properties of containable entities
/// Maps to: entity_containable table
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Containable {
    pub weight: f32,
    pub size: Size,
    pub stackable: bool,
    pub stack_size: i32,
}

impl Containable {
    /// Create a new containable with the given weight
    pub fn new(weight: f32) -> Self {
        Self {
            weight,
            size: Size::Medium,
            stackable: false,
            stack_size: 1,
        }
    }
}

/// Marks entities that can be entered (rooms, vehicles)
/// Maps to: entity_enterable table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enterable {
    pub dest_id: EntityId,
    pub closeable: bool,
    pub closed: bool,
    pub door_rating: Option<i32>,
    pub lockable: bool,
    pub locked: bool,
    pub unlock_code: Option<String>,
    pub lock_rating: Option<i32>,
    pub transparent: bool,
}

impl Enterable {
    /// Create a new enterable entity
    pub fn new(dest_id: EntityId) -> Self {
        Self {
            dest_id,
            closeable: false,
            closed: false,
            door_rating: None,
            lockable: false,
            locked: false,
            unlock_code: None,
            lock_rating: None,
            transparent: false,
        }
    }
}

/// Individual exit data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitData {
    pub dest_id: EntityId,
    pub direction: String,
    pub closeable: bool,
    pub closed: bool,
    pub door_rating: Option<i32>,
    pub lockable: bool,
    pub locked: bool,
    pub unlock_code: Option<String>,
    pub lock_rating: Option<i32>,
    pub transparent: bool,
}

impl ExitData {
    /// Create a new exit with a direction and destination
    pub fn new(direction: impl Into<String>, dest_id: EntityId) -> Self {
        Self {
            dest_id,
            direction: direction.into(),
            closeable: false,
            closed: false,
            door_rating: None,
            lockable: false,
            locked: false,
            unlock_code: None,
            lock_rating: None,
            transparent: false,
        }
    }

    /// Create a new exit with a door
    pub fn with_door(mut self, door_rating: i32) -> Self {
        self.closeable = true;
        self.door_rating = Some(door_rating);
        self
    }

    /// Create a new exit with a lock
    pub fn with_lock(
        mut self,
        door_rating: i32,
        lock_rating: i32,
        unlock_code: impl Into<String>,
    ) -> Self {
        self.closeable = true;
        self.door_rating = Some(door_rating);
        self.lockable = true;
        self.lock_rating = Some(lock_rating);
        self.unlock_code = Some(unlock_code.into());
        self
    }

    /// Mark the exit as closed
    pub fn closed(mut self) -> Self {
        self.closed = true;
        self
    }

    /// Mark the exit as locked
    pub fn locked(mut self) -> Self {
        self.locked = true;
        self
    }

    /// Mark the exit as transparent
    pub fn transparent(mut self) -> Self {
        self.transparent = true;
        self
    }
}

/// Collection of exits from a room
/// Maps to: entity_room_exits table (multiple rows)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exits {
    pub exits: Vec<ExitData>,
}

impl Exits {
    /// Create a new empty exits collection
    pub fn new() -> Self {
        Self { exits: Vec::new() }
    }

    /// Add an exit to the collection
    pub fn add_exit(mut self, exit: ExitData) -> Self {
        self.exits.push(exit);
        self
    }

    /// Find an exit by direction (case-insensitive)
    pub fn find_exit(&self, direction: &str) -> Option<&ExitData> {
        let direction_lower = direction.to_lowercase();
        self.exits
            .iter()
            .find(|e| e.direction.to_lowercase() == direction_lower)
    }

    /// Find an exit by direction (mutable)
    pub fn find_exit_mut(&mut self, direction: &str) -> Option<&mut ExitData> {
        let direction_lower = direction.to_lowercase();
        self.exits
            .iter_mut()
            .find(|e| e.direction.to_lowercase() == direction_lower)
    }

    /// Get all exit directions
    pub fn directions(&self) -> Vec<&str> {
        self.exits.iter().map(|e| e.direction.as_str()).collect()
    }

    /// Check if there's an exit in a given direction
    pub fn has_exit(&self, direction: &str) -> bool {
        self.find_exit(direction).is_some()
    }
}

impl Default for Exits {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location() {
        let area_id = EntityId::from_uuid(Uuid::new_v4());
        let room_id = EntityId::from_uuid(Uuid::new_v4());
        let loc = Location::new(area_id, room_id);
        assert_eq!(loc.area_id, area_id);
        assert_eq!(loc.room_id, room_id);
    }

    #[test]
    fn test_container() {
        let container = Container::new(Some(10));
        assert_eq!(container.capacity, Some(10));
        assert!(!container.closed);
    }

    #[test]
    fn test_containable() {
        let item = Containable::new(5.0);
        assert_eq!(item.weight, 5.0);
        assert_eq!(item.size, Size::Medium);
    }

    #[test]
    fn test_exit_data_basic() {
        let dest_id = EntityId::from_uuid(Uuid::new_v4());
        let exit = ExitData::new("north", dest_id);
        assert_eq!(exit.direction, "north");
        assert_eq!(exit.dest_id, dest_id);
        assert!(!exit.closeable);
        assert!(!exit.closed);
        assert!(!exit.lockable);
        assert!(!exit.locked);
    }

    #[test]
    fn test_exit_data_with_door() {
        let dest_id = EntityId::from_uuid(Uuid::new_v4());
        let exit = ExitData::new("south", dest_id).with_door(10).closed();
        assert!(exit.closeable);
        assert!(exit.closed);
        assert_eq!(exit.door_rating, Some(10));
    }

    #[test]
    fn test_exit_data_with_lock() {
        let dest_id = EntityId::from_uuid(Uuid::new_v4());
        let exit = ExitData::new("east", dest_id)
            .with_lock(10, 5, "key123")
            .locked();
        assert!(exit.lockable);
        assert!(exit.locked);
        assert_eq!(exit.door_rating, Some(10));
        assert_eq!(exit.lock_rating, Some(5));
        assert_eq!(exit.unlock_code, Some("key123".to_string()));
    }

    #[test]
    fn test_exit_data_transparent() {
        let dest_id = EntityId::from_uuid(Uuid::new_v4());
        let exit = ExitData::new("west", dest_id).with_door(10).transparent();
        assert!(exit.transparent);
    }

    #[test]
    fn test_exits_collection() {
        let room1_id = EntityId::from_uuid(Uuid::new_v4());
        let room2_id = EntityId::from_uuid(Uuid::new_v4());

        let exits = Exits::new()
            .add_exit(ExitData::new("north", room1_id))
            .add_exit(ExitData::new("south", room2_id));

        assert_eq!(exits.exits.len(), 2);
        assert!(exits.has_exit("north"));
        assert!(exits.has_exit("SOUTH")); // Case insensitive
        assert!(!exits.has_exit("east"));

        let north_exit = exits.find_exit("north").unwrap();
        assert_eq!(north_exit.dest_id, room1_id);
    }

    #[test]
    fn test_exits_directions() {
        let exits = Exits::new()
            .add_exit(ExitData::new("north", EntityId::from_uuid(Uuid::new_v4())))
            .add_exit(ExitData::new("east", EntityId::from_uuid(Uuid::new_v4())))
            .add_exit(ExitData::new("up", EntityId::from_uuid(Uuid::new_v4())));

        let directions = exits.directions();
        assert_eq!(directions.len(), 3);
        assert!(directions.contains(&"north"));
        assert!(directions.contains(&"east"));
        assert!(directions.contains(&"up"));
    }
}



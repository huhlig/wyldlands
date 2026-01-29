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

//! Entity Component System (ECS) module
//!
//! This module provides the core ECS infrastructure for the game world,
//! including components, systems, and event handling.

pub use hecs::{Entity, Query, QueryBorrow, QueryOne, World};

/// Type alias for hecs runtime entity handles (non-persistent, memory-only)
/// This is NOT the same as the persistent UUID stored in EntityUuid component
pub type EcsEntity = Entity;

/// DEPRECATED: Use EcsEntity instead
/// Temporary backward compatibility alias during migration
#[deprecated(
    since = "0.1.0",
    note = "Use EcsEntity instead to avoid confusion with PersistentEntityId"
)]
pub type EntityId = EcsEntity;

/// Type alias for the game world
pub type GameWorld = World;

// Re-exports
pub mod character_builder;
pub mod components;
pub mod events;
pub mod registry;
pub mod systems;

pub use registry::EntityRegistry;
pub use character_builder::ServerCharacterBuilder;

pub mod context;
#[cfg(test)]
pub mod test_utils;


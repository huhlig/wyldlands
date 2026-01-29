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

//! Pre-built GOAP actions for NPCs

use crate::ecs::components::*;
use crate::ecs::context::WorldContext;
use std::sync::Arc;

/// Action handler trait for executing GOAP actions
#[async_trait::async_trait]
pub trait ActionHandler: Send + Sync {
    /// Execute the action for the given entity
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String>;
    
    /// Get the action definition
    fn definition(&self) -> GoapAction;
}

/// Wander action - NPC moves to a random adjacent location
pub struct WanderAction;

#[async_trait::async_trait]
impl ActionHandler for WanderAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing wander action", entity);
        
        // Get current location
        let location = {
            let world = context.entities().read().await;
            *world.get::<&Location>(entity)
                .map_err(|_| "NPC has no location")?
        };
        
        // TODO: Get available exits and pick random one
        // For now, just log the action
        tracing::info!("NPC {:?} is wandering from location {:?}", entity, location.room_id);
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("wander", "Wander")
            .with_precondition("is_idle", true)
            .with_effect("has_moved", true)
            .with_cost(2.0)
    }
}

/// Follow action - NPC follows a target entity
pub struct FollowAction {
    pub target: uuid::Uuid,
}

#[async_trait::async_trait]
impl ActionHandler for FollowAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing follow action for target {}", entity, self.target);
        
        // Get target location
        let target_entity = context.get_entity_by_uuid(self.target).await
            .ok_or("Target not found")?;
        
        let (target_location, npc_location) = {
            let world = context.entities().read().await;
            let target_loc = *world.get::<&Location>(target_entity)
                .map_err(|_| "Target has no location")?;
            let npc_loc = *world.get::<&Location>(entity)
                .map_err(|_| "NPC has no location")?;
            (target_loc, npc_loc)
        };
        
        if target_location.room_id != npc_location.room_id {
            tracing::info!("NPC {:?} following target to room {:?}", entity, target_location.room_id);
            // TODO: Pathfinding and movement
        }
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("follow", "Follow Target")
            .with_precondition("has_target", true)
            .with_effect("near_target", true)
            .with_cost(3.0)
    }
}

/// Attack action - NPC attacks a target
pub struct AttackAction {
    pub target: uuid::Uuid,
}

#[async_trait::async_trait]
impl ActionHandler for AttackAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing attack action on target {}", entity, self.target);
        
        let target_entity = context.get_entity_by_uuid(self.target).await
            .ok_or("Target not found")?;
        
        // TODO: Implement combat system integration
        tracing::info!("NPC {:?} attacking target {:?}", entity, target_entity);
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("attack", "Attack Target")
            .with_precondition("near_target", true)
            .with_precondition("is_hostile", true)
            .with_effect("target_damaged", true)
            .with_cost(1.0)
    }
}

/// Flee action - NPC flees from danger
pub struct FleeAction;

#[async_trait::async_trait]
impl ActionHandler for FleeAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing flee action", entity);
        
        // TODO: Find safe location and move there
        tracing::info!("NPC {:?} is fleeing", entity);
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("flee", "Flee from Danger")
            .with_precondition("in_danger", true)
            .with_effect("is_safe", true)
            .with_effect("in_danger", false)
            .with_cost(2.0)
    }
}

/// Patrol action - NPC patrols between waypoints
pub struct PatrolAction {
    pub waypoints: Vec<uuid::Uuid>,
    pub current_index: usize,
}

#[async_trait::async_trait]
impl ActionHandler for PatrolAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing patrol action", entity);
        
        if self.waypoints.is_empty() {
            return Err("No waypoints defined".to_string());
        }
        
        let target_waypoint = self.waypoints[self.current_index % self.waypoints.len()];
        tracing::info!("NPC {:?} patrolling to waypoint {:?}", entity, target_waypoint);
        
        // TODO: Move to waypoint
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("patrol", "Patrol Area")
            .with_precondition("on_patrol", true)
            .with_effect("at_waypoint", true)
            .with_cost(2.0)
    }
}

/// Guard action - NPC guards a location
pub struct GuardAction {
    pub location: EntityId,
}

#[async_trait::async_trait]
impl ActionHandler for GuardAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing guard action at {:?}", entity, self.location);
        
        let npc_location = {
            let world = context.entities().read().await;
            *world.get::<&Location>(entity)
                .map_err(|_| "NPC has no location")?
        };
        
        if npc_location.room_id != self.location {
            tracing::info!("NPC {:?} returning to guard post {:?}", entity, self.location);
            // TODO: Move to guard location
        } else {
            tracing::debug!("NPC {:?} is guarding location", entity);
        }
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("guard", "Guard Location")
            .with_precondition("is_guard", true)
            .with_effect("at_post", true)
            .with_cost(1.0)
    }
}

/// Rest action - NPC rests to recover
pub struct RestAction;

#[async_trait::async_trait]
impl ActionHandler for RestAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing rest action", entity);
        
        // TODO: Restore health/mana
        tracing::info!("NPC {:?} is resting", entity);
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("rest", "Rest and Recover")
            .with_precondition("is_tired", true)
            .with_effect("is_rested", true)
            .with_effect("is_tired", false)
            .with_cost(5.0)
    }
}

/// Interact action - NPC interacts with an object
pub struct InteractAction {
    pub target: uuid::Uuid,
}

#[async_trait::async_trait]
impl ActionHandler for InteractAction {
    async fn execute(&self, context: Arc<WorldContext>, entity: hecs::Entity) -> Result<(), String> {
        tracing::debug!("NPC {:?}: Executing interact action with {:?}", entity, self.target);
        
        // TODO: Implement interaction system
        tracing::info!("NPC {:?} interacting with {:?}", entity, self.target);
        
        Ok(())
    }
    
    fn definition(&self) -> GoapAction {
        GoapAction::new("interact", "Interact with Object")
            .with_precondition("near_object", true)
            .with_effect("has_interacted", true)
            .with_cost(1.0)
    }
}

/// Action library for managing pre-built actions
pub struct ActionLibrary {
    handlers: std::collections::HashMap<String, Box<dyn ActionHandler>>,
}

impl ActionLibrary {
    /// Create a new action library with default actions
    pub fn new() -> Self {
        let mut library = Self {
            handlers: std::collections::HashMap::new(),
        };
        
        // Register all default actions
        library.register("wander", Box::new(WanderAction));
        library.register("follow", Box::new(FollowAction { target: uuid::Uuid::nil() }));
        library.register("attack", Box::new(AttackAction { target: uuid::Uuid::nil() }));
        library.register("flee", Box::new(FleeAction));
        library.register("patrol", Box::new(PatrolAction { waypoints: vec![], current_index: 0 }));
        library.register("guard", Box::new(GuardAction { location: EntityId::from_uuid(uuid::Uuid::nil()) }));
        library.register("rest", Box::new(RestAction));
        library.register("interact", Box::new(InteractAction { target: uuid::Uuid::nil() }));
        
        library
    }
    
    /// Register an action handler
    pub fn register(&mut self, id: impl Into<String>, handler: Box<dyn ActionHandler>) {
        self.handlers.insert(id.into(), handler);
    }
    
    /// Get an action handler
    pub fn get(&self, id: &str) -> Option<&dyn ActionHandler> {
        self.handlers.get(id).map(|h| h.as_ref())
    }
    
    /// Execute an action
    pub async fn execute(
        &self,
        action_id: &str,
        context: Arc<WorldContext>,
        entity: hecs::Entity,
    ) -> Result<(), String> {
        match self.get(action_id) {
            Some(handler) => handler.execute(context, entity).await,
            None => Err(format!("Unknown action: {}", action_id)),
        }
    }
    
    /// Get all action definitions
    pub fn get_definitions(&self) -> Vec<GoapAction> {
        self.handlers.values().map(|h| h.definition()).collect()
    }
}

impl Default for ActionLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_action_library() {
        let library = ActionLibrary::new();
        assert!(library.get("wander").is_some());
        assert!(library.get("flee").is_some());
        assert!(library.get("rest").is_some());
        assert!(library.get("unknown").is_none());
    }
    
    #[test]
    fn test_action_definitions() {
        let library = ActionLibrary::new();
        let definitions = library.get_definitions();
        assert!(!definitions.is_empty());
        
        let wander = definitions.iter().find(|a| a.id == "wander");
        assert!(wander.is_some());
    }
}


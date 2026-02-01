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

//! AI components for NPC behavior and personality

mod emotion;
mod goap;

pub use self::emotion::{
    Personality, PersonalityBigFive, PersonalityGoal, PersonalityGoals, PersonalityMood,
    PersonalityTraits,
};
pub use self::goap::{
    ActionCost, BehaviorType, GoapAction, GoapGoal, GoapPlanner, StateType, WorldState,
};
use crate::ecs::components::EntityId;
use serde::{Deserialize, Serialize};

/// AI controller component
/// Maps to: entity_ai_controller table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIController {
    pub behavior_type: BehaviorType,
    pub current_goal: Option<String>,
    pub state_type: StateType,
    pub state_target_id: Option<EntityId>,
    pub update_interval: f32,
    pub time_since_update: f32,
}

impl AIController {
    /// Create a new AI controller with the given behavior type
    pub fn new(behavior_type: BehaviorType) -> Self {
        Self {
            behavior_type,
            current_goal: None,
            state_type: StateType::Idle,
            state_target_id: None,
            update_interval: 1.0,
            time_since_update: 0.0,
        }
    }

    /// Check if the AI should update
    pub fn should_update(&self, _delta_time: f32) -> bool {
        self.time_since_update >= self.update_interval
    }

    /// Mark the AI as updated
    pub fn mark_updated(&mut self) {
        self.time_since_update = 0.0;
    }

    /// Update the time since last update
    pub fn update_timer(&mut self, delta_time: f32) {
        self.time_since_update += delta_time;
    }
}

// Legacy compatibility type (deprecated)
#[deprecated(note = "Use StateType instead")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIState {
    Idle,
    Moving { target: EntityId },
    Combat { target: EntityId },
    Fleeing { from: EntityId },
    Following { target: EntityId },
    Dialogue { with: EntityId },
}

#[cfg(test)]
mod tests {
    use super::{AIController, BehaviorType};

    #[test]
    fn test_ai_controller_update() {
        let mut ai = AIController::new(BehaviorType::Wandering);
        assert!(!ai.should_update(0.5));

        ai.update_timer(1.0);
        assert!(ai.should_update(0.0));

        ai.mark_updated();
        assert!(!ai.should_update(0.0));
    }
}

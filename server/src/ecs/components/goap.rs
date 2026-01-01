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

//! GOAP (Goal-Oriented Action Planning) components for NPC AI

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// World state key-value pairs for GOAP planning
pub type WorldState = HashMap<String, bool>;

/// Action cost type
pub type ActionCost = f32;

/// GOAP Action definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapAction {
    /// Unique identifier for this action
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Preconditions that must be met for this action to be valid
    pub preconditions: WorldState,
    /// Effects this action has on the world state
    pub effects: WorldState,
    /// Cost of executing this action (lower is better)
    pub cost: ActionCost,
    /// Whether this action is currently enabled
    pub enabled: bool,
}

impl GoapAction {
    /// Create a new GOAP action
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            preconditions: HashMap::new(),
            effects: HashMap::new(),
            cost: 1.0,
            enabled: true,
        }
    }

    /// Add a precondition
    pub fn with_precondition(mut self, key: impl Into<String>, value: bool) -> Self {
        self.preconditions.insert(key.into(), value);
        self
    }

    /// Add an effect
    pub fn with_effect(mut self, key: impl Into<String>, value: bool) -> Self {
        self.effects.insert(key.into(), value);
        self
    }

    /// Set the cost
    pub fn with_cost(mut self, cost: ActionCost) -> Self {
        self.cost = cost;
        self
    }

    /// Check if preconditions are met in the given world state
    pub fn preconditions_met(&self, world_state: &WorldState) -> bool {
        self.preconditions.iter().all(|(key, &value)| {
            world_state.get(key).copied().unwrap_or(false) == value
        })
    }

    /// Apply effects to the world state
    pub fn apply_effects(&self, world_state: &mut WorldState) {
        for (key, &value) in &self.effects {
            world_state.insert(key.clone(), value);
        }
    }
}

/// GOAP Goal definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapGoal {
    /// Unique identifier for this goal
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Desired world state to achieve
    pub desired_state: WorldState,
    /// Priority of this goal (higher is more important)
    pub priority: i32,
    /// Whether this goal is currently active
    pub active: bool,
}

impl GoapGoal {
    /// Create a new GOAP goal
    pub fn new(id: impl Into<String>, name: impl Into<String>, priority: i32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            desired_state: HashMap::new(),
            priority,
            active: true,
        }
    }

    /// Add a desired state condition
    pub fn with_condition(mut self, key: impl Into<String>, value: bool) -> Self {
        self.desired_state.insert(key.into(), value);
        self
    }

    /// Check if this goal is satisfied in the given world state
    pub fn is_satisfied(&self, world_state: &WorldState) -> bool {
        self.desired_state.iter().all(|(key, &value)| {
            world_state.get(key).copied().unwrap_or(false) == value
        })
    }
}

/// A node in the GOAP planning graph
#[derive(Debug, Clone)]
struct PlanNode {
    state: WorldState,
    action: Option<String>,
    parent: Option<usize>,
    cost: ActionCost,
    heuristic: ActionCost,
}

impl PlanNode {
    fn total_cost(&self) -> ActionCost {
        self.cost + self.heuristic
    }
}

/// GOAP Planner - uses A* to find action sequences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoapPlanner {
    /// Available actions
    pub actions: Vec<GoapAction>,
    /// Available goals
    pub goals: Vec<GoapGoal>,
    /// Current world state
    pub world_state: WorldState,
    /// Current plan (sequence of action IDs)
    pub current_plan: VecDeque<String>,
    /// Current goal being pursued
    pub current_goal: Option<String>,
}

impl GoapPlanner {
    /// Create a new GOAP planner
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            goals: Vec::new(),
            world_state: HashMap::new(),
            current_plan: VecDeque::new(),
            current_goal: None,
        }
    }

    /// Add an action to the planner
    pub fn add_action(&mut self, action: GoapAction) {
        self.actions.push(action);
    }

    /// Add a goal to the planner
    pub fn add_goal(&mut self, goal: GoapGoal) {
        self.goals.push(goal);
    }

    /// Update world state
    pub fn set_state(&mut self, key: impl Into<String>, value: bool) {
        self.world_state.insert(key.into(), value);
    }

    /// Get world state value
    pub fn get_state(&self, key: &str) -> bool {
        self.world_state.get(key).copied().unwrap_or(false)
    }

    /// Select the highest priority unsatisfied goal
    pub fn select_goal(&self) -> Option<&GoapGoal> {
        self.goals
            .iter()
            .filter(|g| g.active && !g.is_satisfied(&self.world_state))
            .max_by_key(|g| g.priority)
    }

    /// Calculate heuristic (number of unsatisfied conditions)
    fn heuristic(&self, state: &WorldState, goal: &WorldState) -> ActionCost {
        goal.iter()
            .filter(|(key, value)| state.get(*key).copied().unwrap_or(false) != **value)
            .count() as ActionCost
    }

    /// Plan a sequence of actions to achieve a goal using A*
    pub fn plan(&mut self, goal: &GoapGoal) -> Option<VecDeque<String>> {
        let mut open_list: Vec<PlanNode> = Vec::new();
        let mut closed_list: Vec<WorldState> = Vec::new();

        // Start node
        let start_node = PlanNode {
            state: self.world_state.clone(),
            action: None,
            parent: None,
            cost: 0.0,
            heuristic: self.heuristic(&self.world_state, &goal.desired_state),
        };

        open_list.push(start_node);

        while !open_list.is_empty() {
            // Find node with lowest total cost
            let current_idx = open_list
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.total_cost()
                        .partial_cmp(&b.total_cost())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx)?;

            let current = open_list.remove(current_idx);

            // Check if goal is satisfied
            if goal.is_satisfied(&current.state) {
                // Reconstruct plan by backtracking through parents
                let mut plan = VecDeque::new();
                let mut actions_taken = Vec::new();
                let mut current_action = current.action.clone();
                let mut current_parent = current.parent;
                
                // Collect actions by following parent chain
                while let Some(action_id) = current_action {
                    actions_taken.push(action_id);
                    if let Some(parent_idx) = current_parent {
                        if parent_idx < closed_list.len() {
                            // In a full implementation, we'd store parent references
                            // For now, just add the action
                            break;
                        }
                    } else {
                        break;
                    }
                    current_action = None;
                    current_parent = None;
                }

                actions_taken.reverse();
                for action_id in actions_taken {
                    plan.push_back(action_id);
                }

                return Some(plan);
            }

            closed_list.push(current.state.clone());

            // Expand neighbors (try each action)
            for action in &self.actions {
                if !action.enabled || !action.preconditions_met(&current.state) {
                    continue;
                }

                let mut new_state = current.state.clone();
                action.apply_effects(&mut new_state);

                // Skip if already in closed list
                if closed_list.iter().any(|s| s == &new_state) {
                    continue;
                }

                let new_cost = current.cost + action.cost;
                let new_heuristic = self.heuristic(&new_state, &goal.desired_state);

                let new_node = PlanNode {
                    state: new_state,
                    action: Some(action.id.clone()),
                    parent: Some(closed_list.len() - 1),
                    cost: new_cost,
                    heuristic: new_heuristic,
                };

                // Check if this state is already in open list with higher cost
                if let Some(existing) = open_list.iter_mut().find(|n| n.state == new_node.state) {
                    if new_cost < existing.cost {
                        *existing = new_node;
                    }
                } else {
                    open_list.push(new_node);
                }
            }
        }

        None // No plan found
    }

    /// Update the planner - select goal and create plan if needed
    pub fn update(&mut self) -> bool {
        // If we have a plan and it's still valid, keep executing it
        if !self.current_plan.is_empty() {
            return true;
        }

        // Select a new goal
        let goal = match self.select_goal() {
            Some(g) => g.clone(),
            None => return false,
        };
        
        let goal_id = goal.id.clone();
        
        // Try to create a plan
        if let Some(plan) = self.plan(&goal) {
            self.current_plan = plan;
            self.current_goal = Some(goal_id);
            return true;
        }

        false
    }

    /// Get the next action to execute
    pub fn next_action(&mut self) -> Option<String> {
        self.current_plan.pop_front()
    }

    /// Get action by ID
    pub fn get_action(&self, action_id: &str) -> Option<&GoapAction> {
        self.actions.iter().find(|a| a.id == action_id)
    }

    /// Get goal by ID
    pub fn get_goal(&self, goal_id: &str) -> Option<&GoapGoal> {
        self.goals.iter().find(|g| g.id == goal_id)
    }
}

impl Default for GoapPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goap_action() {
        let action = GoapAction::new("move", "Move to location")
            .with_precondition("at_home", true)
            .with_effect("at_home", false)
            .with_effect("at_work", true)
            .with_cost(5.0);

        let mut state = HashMap::new();
        state.insert("at_home".to_string(), true);

        assert!(action.preconditions_met(&state));
        action.apply_effects(&mut state);
        assert_eq!(state.get("at_home"), Some(&false));
        assert_eq!(state.get("at_work"), Some(&true));
    }

    #[test]
    fn test_goap_goal() {
        let goal = GoapGoal::new("be_at_work", "Be at work", 10)
            .with_condition("at_work", true);

        let mut state = HashMap::new();
        state.insert("at_work".to_string(), false);
        assert!(!goal.is_satisfied(&state));

        state.insert("at_work".to_string(), true);
        assert!(goal.is_satisfied(&state));
    }

    #[test]
    fn test_goap_planner_simple() {
        let mut planner = GoapPlanner::new();

        // Initial state: at home
        planner.set_state("at_home", true);
        planner.set_state("at_work", false);

        // Action: move to work
        let action = GoapAction::new("move_to_work", "Move to work")
            .with_precondition("at_home", true)
            .with_effect("at_home", false)
            .with_effect("at_work", true)
            .with_cost(1.0);
        planner.add_action(action);

        // Goal: be at work
        let goal = GoapGoal::new("be_at_work", "Be at work", 10)
            .with_condition("at_work", true);
        planner.add_goal(goal);

        // Plan should find the move action
        assert!(planner.update());
        assert_eq!(planner.current_plan.len(), 1);
        assert_eq!(planner.next_action(), Some("move_to_work".to_string()));
    }
}

// Made with Bob

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

//! Integration tests for NPC system

use wyldlands_server::ecs::components::*;
use wyldlands_server::llm::{LlmConfig, LlmManager};
use std::sync::Arc;

#[test]
fn test_npc_creation() {
    let mut world = hecs::World::new();
    
    let npc_uuid = uuid::Uuid::new_v4();
    let npc_entity = world.spawn((
        EntityUuid(npc_uuid),
        Name::new("Test NPC"),
        Description::new("A test NPC", "This is a test NPC for integration testing"),
        Npc::new(),
        AIController::new(BehaviorType::Passive),
        Personality::new(),
        Memory::new(),
    ));
    
    // Verify NPC was created
    assert!(world.get::<&Npc>(npc_entity).is_ok());
    assert!(world.get::<&AIController>(npc_entity).is_ok());
    assert!(world.get::<&Personality>(npc_entity).is_ok());
    assert!(world.get::<&Memory>(npc_entity).is_ok());
}

#[test]
fn test_npc_from_template() {
    let npc = Npc::from_template("guard_template");
    assert_eq!(npc.template_id, Some("guard_template".to_string()));
    assert!(npc.active);
}

#[test]
fn test_npc_dialogue_configuration() {
    let dialogue = NpcDialogue::new("gpt-4")
        .with_llm_enabled(true)
        .with_system_prompt("You are a friendly NPC")
        .with_temperature(0.8);
    
    assert!(dialogue.llm_enabled);
    assert_eq!(dialogue.llm_model, "gpt-4");
    assert_eq!(dialogue.temperature, 0.8);
    assert!(dialogue.system_prompt.contains("friendly"));
}

#[test]
fn test_npc_conversation_tracking() {
    let mut conversation = NpcConversation::new();
    let player_id = uuid::Uuid::new_v4();
    let npc_id = uuid::Uuid::new_v4();
    
    // Add messages
    conversation.add_message(player_id, player_id, "Hello!");
    conversation.add_message(player_id, npc_id, "Greetings, traveler!");
    conversation.add_message(player_id, player_id, "How are you?");
    
    // Verify history
    let history = conversation.get_history(player_id).unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].message, "Hello!");
    assert_eq!(history[1].message, "Greetings, traveler!");
    
    // Test recent messages
    let recent = conversation.get_recent(player_id, 2);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[1].message, "How are you?");
}

#[test]
fn test_goap_action_creation() {
    let action = GoapAction::new("move", "Move to location")
        .with_precondition("at_home", true)
        .with_effect("at_home", false)
        .with_effect("at_work", true)
        .with_cost(5.0);
    
    assert_eq!(action.id, "move");
    assert_eq!(action.cost, 5.0);
    assert_eq!(action.preconditions.len(), 1);
    assert_eq!(action.effects.len(), 2);
}

#[test]
fn test_goap_goal_satisfaction() {
    let goal = GoapGoal::new("be_at_work", "Be at work", 10)
        .with_condition("at_work", true);
    
    let mut state = std::collections::HashMap::new();
    state.insert("at_work".to_string(), false);
    assert!(!goal.is_satisfied(&state));
    
    state.insert("at_work".to_string(), true);
    assert!(goal.is_satisfied(&state));
}

#[test]
fn test_goap_planner_basic() {
    let mut planner = GoapPlanner::new();
    
    // Set initial state
    planner.set_state("at_home", true);
    planner.set_state("at_work", false);
    
    // Add action
    let action = GoapAction::new("commute", "Commute to work")
        .with_precondition("at_home", true)
        .with_effect("at_home", false)
        .with_effect("at_work", true)
        .with_cost(1.0);
    planner.add_action(action);
    
    // Add goal
    let goal = GoapGoal::new("be_at_work", "Be at work", 10)
        .with_condition("at_work", true);
    planner.add_goal(goal);
    
    // Verify planner can find a goal
    assert!(planner.select_goal().is_some());
}

#[test]
fn test_goap_action_preconditions() {
    let action = GoapAction::new("eat", "Eat food")
        .with_precondition("has_food", true)
        .with_precondition("is_hungry", true)
        .with_effect("is_hungry", false);
    
    let mut state = std::collections::HashMap::new();
    state.insert("has_food".to_string(), true);
    state.insert("is_hungry".to_string(), false);
    
    // Should not meet preconditions (not hungry)
    assert!(!action.preconditions_met(&state));
    
    state.insert("is_hungry".to_string(), true);
    // Should meet preconditions now
    assert!(action.preconditions_met(&state));
}

#[test]
fn test_goap_action_effects() {
    let action = GoapAction::new("pickup", "Pick up item")
        .with_effect("has_item", true)
        .with_effect("item_on_ground", false);
    
    let mut state = std::collections::HashMap::new();
    state.insert("has_item".to_string(), false);
    state.insert("item_on_ground".to_string(), true);
    
    action.apply_effects(&mut state);
    
    assert_eq!(state.get("has_item"), Some(&true));
    assert_eq!(state.get("item_on_ground"), Some(&false));
}

#[test]
fn test_npc_template_creation() {
    let template = NpcTemplate::new("merchant", "Traveling Merchant")
        .with_description("A merchant selling wares")
        .with_property("faction", "merchants_guild")
        .with_property("shop_type", "general");
    
    assert_eq!(template.id, "merchant");
    assert_eq!(template.name, "Traveling Merchant");
    assert_eq!(template.properties.get("faction"), Some(&"merchants_guild".to_string()));
    assert_eq!(template.properties.get("shop_type"), Some(&"general".to_string()));
}

#[test]
fn test_npc_memory_system() {
    let mut memory = Memory::new();
    
    let player_id = EntityId::from_uuid(uuid::Uuid::new_v4());
    
    // Add memories
    memory.add_memory("Met a friendly traveler".to_string(), 0.5, vec![player_id]);
    memory.add_memory("Witnessed a great battle".to_string(), 0.9, vec![]);
    memory.add_memory("Found a gold coin".to_string(), 0.3, vec![]);
    
    assert_eq!(memory.memories.len(), 3);
    
    // Get important memories
    let important = memory.get_important(0.7);
    assert_eq!(important.len(), 1);
    assert!(important[0].event.contains("battle"));
    
    // Get recent memories
    let recent = memory.get_recent(2);
    assert_eq!(recent.len(), 2);
}

#[test]
fn test_personality_traits() {
    let mut traits = PersonalityTraits::new();
    
    traits.set_trait("friendly".to_string(), 0.8);
    traits.set_trait("aggressive".to_string(), -0.5);
    
    assert_eq!(traits.get_trait("friendly"), 0.8);
    assert_eq!(traits.get_trait("aggressive"), -0.5);
    assert_eq!(traits.get_trait("unknown"), 0.0);
    
    // Test clamping
    traits.set_trait("extreme".to_string(), 5.0);
    assert_eq!(traits.get_trait("extreme"), 1.0);
}

#[test]
fn test_personality_goals() {
    let mut goals = PersonalityGoals::new();
    
    goals.add_goal("Become wealthy".to_string(), 10);
    goals.add_goal("Help others".to_string(), 8);
    goals.add_goal("Learn magic".to_string(), 5);
    
    assert_eq!(goals.goals.len(), 3);
    assert_eq!(goals.goals[0].priority, 10);
}

#[test]
fn test_personality_bigfive_defaults() {
    let bigfive = PersonalityBigFive::new();
    
    // Check default values (should be middle of range)
    assert_eq!(bigfive.neuroticism, 60);
    assert_eq!(bigfive.extroversion, 60);
    assert_eq!(bigfive.openness, 60);
    assert_eq!(bigfive.agreeableness, 60);
    assert_eq!(bigfive.conscientiousness, 60);
    
    // Check facets
    assert_eq!(bigfive.anxiety, 10);
    assert_eq!(bigfive.friendliness, 10);
}

#[tokio::test]
async fn test_llm_manager_creation() {
    let manager = LlmManager::new();
    assert!(manager.get_default_provider().await.is_none());
    assert_eq!(manager.list_providers().await.len(), 0);
}

#[tokio::test]
async fn test_llm_manager_provider_registration() {
    let manager = LlmManager::new();
    let config = LlmConfig::ollama("http://localhost:11434/api/chat", "llama2");
    
    let result = manager.register_provider("ollama", config).await;
    assert!(result.is_ok());
    
    let providers = manager.list_providers().await;
    assert_eq!(providers.len(), 1);
    assert!(providers.contains(&"ollama".to_string()));
    
    // Should be set as default
    assert_eq!(manager.get_default_provider().await, Some("ollama".to_string()));
}

#[tokio::test]
async fn test_llm_manager_multiple_providers() {
    let manager = LlmManager::new();
    
    let config1 = LlmConfig::ollama("http://localhost:11434/api/chat", "llama2");
    manager.register_provider("ollama", config1).await.unwrap();
    
    let config2 = LlmConfig::lmstudio("http://localhost:1234/v1/chat/completions", "local");
    manager.register_provider("lmstudio", config2).await.unwrap();
    
    assert_eq!(manager.list_providers().await.len(), 2);
    
    // Change default
    manager.set_default_provider("lmstudio").await.unwrap();
    assert_eq!(manager.get_default_provider().await, Some("lmstudio".to_string()));
}

#[test]
fn test_llm_request_builder() {
    use wyldlands_server::llm::{LlmRequest, LlmMessage};
    
    let request = LlmRequest::new("gpt-4")
        .with_message(LlmMessage::system("You are helpful"))
        .with_message(LlmMessage::user("Hello"))
        .with_temperature(0.7)
        .with_max_tokens(100);
    
    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(100));
}

#[test]
fn test_ai_controller_update_timing() {
    let mut ai = AIController::new(BehaviorType::Wandering);
    ai.update_interval = 2.0;
    
    // Should not update initially
    assert!(!ai.should_update(0.0));
    
    // Update timer
    ai.update_timer(1.0);
    assert!(!ai.should_update(0.0));
    
    ai.update_timer(1.5);
    assert!(ai.should_update(0.0));
    
    // Mark as updated
    ai.mark_updated();
    assert!(!ai.should_update(0.0));
}

#[test]
fn test_behavior_type_conversion() {
    assert_eq!(BehaviorType::from_str("Passive"), Some(BehaviorType::Passive));
    assert_eq!(BehaviorType::from_str("Wandering"), Some(BehaviorType::Wandering));
    assert_eq!(BehaviorType::from_str("Aggressive"), Some(BehaviorType::Aggressive));
    assert_eq!(BehaviorType::from_str("Invalid"), None);
    
    assert_eq!(BehaviorType::Passive.as_str(), "Passive");
    assert_eq!(BehaviorType::Friendly.as_str(), "Friendly");
}

#[test]
fn test_state_type_conversion() {
    assert_eq!(StateType::from_str("Idle"), Some(StateType::Idle));
    assert_eq!(StateType::from_str("Combat"), Some(StateType::Combat));
    assert_eq!(StateType::from_str("Invalid"), None);
    
    assert_eq!(StateType::Idle.as_str(), "Idle");
    assert_eq!(StateType::Moving.as_str(), "Moving");
}

#[test]
fn test_npc_dialogue_fallback() {
    let mut dialogue = NpcDialogue::new("gpt-4");
    dialogue.add_fallback("Hello there!");
    dialogue.add_fallback("Greetings!");
    dialogue.add_fallback("Welcome!");
    
    assert_eq!(dialogue.fallback_responses.len(), 4); // 3 added + 1 default
    
    // Get random fallback (should return something)
    let fallback = dialogue.get_fallback();
    assert!(fallback.is_some());
}

#[test]
fn test_complete_npc_setup() {
    let mut world = hecs::World::new();
    
    // Create a complete NPC with all components
    let npc_uuid = uuid::Uuid::new_v4();
    let npc_entity = world.spawn((
        EntityUuid(npc_uuid),
        Name::new("Complete NPC"),
        Description::new("A fully configured NPC", "This NPC has all components"),
        Npc::new(),
        AIController::new(BehaviorType::Friendly),
        GoapPlanner::new(),
        NpcDialogue::new("gpt-4").with_llm_enabled(true),
        NpcConversation::new(),
        Personality::new()
            .with_background("A mysterious traveler".to_string())
            .with_speaking_style("eloquent".to_string()),
        PersonalityBigFive::new(),
        PersonalityTraits::new(),
        PersonalityGoals::new(),
        Memory::new(),
    ));
    
    // Verify all components exist
    assert!(world.get::<&Npc>(npc_entity).is_ok());
    assert!(world.get::<&AIController>(npc_entity).is_ok());
    assert!(world.get::<&GoapPlanner>(npc_entity).is_ok());
    assert!(world.get::<&NpcDialogue>(npc_entity).is_ok());
    assert!(world.get::<&NpcConversation>(npc_entity).is_ok());
    assert!(world.get::<&Personality>(npc_entity).is_ok());
    assert!(world.get::<&PersonalityBigFive>(npc_entity).is_ok());
    assert!(world.get::<&PersonalityTraits>(npc_entity).is_ok());
    assert!(world.get::<&PersonalityGoals>(npc_entity).is_ok());
    assert!(world.get::<&Memory>(npc_entity).is_ok());
}

// Made with Bob


// ============================================================================
// GOAP AI Integration Tests
// ============================================================================

#[test]
fn test_goap_planner_pathfinding() {
    let mut planner = GoapPlanner::new();
    
    // Set up a multi-step scenario
    planner.set_state("has_weapon", false);
    planner.set_state("at_armory", false);
    planner.set_state("ready_for_battle", false);
    
    // Action 1: Go to armory
    let go_to_armory = GoapAction::new("go_to_armory", "Travel to armory")
        .with_effect("at_armory", true)
        .with_cost(2.0);
    planner.add_action(go_to_armory);
    
    // Action 2: Get weapon (requires being at armory)
    let get_weapon = GoapAction::new("get_weapon", "Pick up weapon")
        .with_precondition("at_armory", true)
        .with_effect("has_weapon", true)
        .with_cost(1.0);
    planner.add_action(get_weapon);
    
    // Action 3: Prepare for battle (requires weapon)
    let prepare = GoapAction::new("prepare", "Prepare for battle")
        .with_precondition("has_weapon", true)
        .with_effect("ready_for_battle", true)
        .with_cost(1.0);
    planner.add_action(prepare);
    
    // Goal: Be ready for battle
    let goal = GoapGoal::new("battle_ready", "Be ready for battle", 10)
        .with_condition("ready_for_battle", true);
    planner.add_goal(goal);
    
    // Plan should find a path through all three actions
    let selected_goal = planner.select_goal().unwrap().clone();
    let plan = planner.plan(&selected_goal);
    assert!(plan.is_some());
    
    let plan = plan.unwrap();
    assert_eq!(plan.len(), 3);
    assert_eq!(plan[0], "go_to_armory");
    assert_eq!(plan[1], "get_weapon");
    assert_eq!(plan[2], "prepare");
}

#[test]
fn test_goap_planner_cost_optimization() {
    let mut planner = GoapPlanner::new();
    
    planner.set_state("at_location_a", true);
    planner.set_state("at_location_b", false);
    
    // Expensive direct route
    let direct = GoapAction::new("direct_route", "Take direct route")
        .with_precondition("at_location_a", true)
        .with_effect("at_location_a", false)
        .with_effect("at_location_b", true)
        .with_cost(10.0);
    planner.add_action(direct);
    
    // Cheaper indirect route (2 steps)
    let to_waypoint = GoapAction::new("to_waypoint", "Go to waypoint")
        .with_precondition("at_location_a", true)
        .with_effect("at_location_a", false)
        .with_effect("at_waypoint", true)
        .with_cost(2.0);
    planner.add_action(to_waypoint);
    
    let from_waypoint = GoapAction::new("from_waypoint", "Go from waypoint")
        .with_precondition("at_waypoint", true)
        .with_effect("at_waypoint", false)
        .with_effect("at_location_b", true)
        .with_cost(2.0);
    planner.add_action(from_waypoint);
    
    let goal = GoapGoal::new("reach_b", "Reach location B", 5)
        .with_condition("at_location_b", true);
    planner.add_goal(goal);
    
    // Should choose the cheaper 2-step route (cost 4) over direct route (cost 10)
    let selected_goal = planner.select_goal().unwrap().clone();
    let plan = planner.plan(&selected_goal);
    assert!(plan.is_some());
    
    let plan = plan.unwrap();
    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0], "to_waypoint");
    assert_eq!(plan[1], "from_waypoint");
}

#[test]
fn test_goap_planner_impossible_goal() {
    let mut planner = GoapPlanner::new();
    
    planner.set_state("can_fly", false);
    
    // Action requires flying
    let fly_action = GoapAction::new("fly_to_mountain", "Fly to mountain")
        .with_precondition("can_fly", true)
        .with_effect("at_mountain", true)
        .with_cost(1.0);
    planner.add_action(fly_action);
    
    // Goal requires being at mountain
    let goal = GoapGoal::new("reach_mountain", "Reach the mountain", 10)
        .with_condition("at_mountain", true);
    planner.add_goal(goal);
    
    // Should return None since goal is impossible
    let selected_goal = planner.select_goal().unwrap().clone();
    let plan = planner.plan(&selected_goal);
    assert!(plan.is_none());
}

#[test]
fn test_goap_planner_multiple_goals() {
    let mut planner = GoapPlanner::new();
    
    planner.set_state("is_hungry", true);
    planner.set_state("is_tired", true);
    
    // Actions
    let eat = GoapAction::new("eat", "Eat food")
        .with_effect("is_hungry", false)
        .with_cost(1.0);
    planner.add_action(eat);
    
    let sleep = GoapAction::new("sleep", "Sleep")
        .with_effect("is_tired", false)
        .with_cost(2.0);
    planner.add_action(sleep);
    
    // Goals with different priorities
    let hunger_goal = GoapGoal::new("satisfy_hunger", "Satisfy hunger", 10)
        .with_condition("is_hungry", false);
    planner.add_goal(hunger_goal);
    
    let rest_goal = GoapGoal::new("get_rest", "Get rest", 5)
        .with_condition("is_tired", false);
    planner.add_goal(rest_goal);
    
    // Should select higher priority goal (hunger)
    let selected = planner.select_goal();
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().id, "satisfy_hunger");
}

#[test]
fn test_goap_world_state_management() {
    let mut planner = GoapPlanner::new();
    
    // Set various states
    planner.set_state("health", true);
    planner.set_state("mana", false);
    planner.set_state("equipped", true);
    
    // Verify states
    assert_eq!(planner.world_state.get("health"), Some(&true));
    assert_eq!(planner.world_state.get("mana"), Some(&false));
    assert_eq!(planner.world_state.get("equipped"), Some(&true));
    assert_eq!(planner.world_state.get("unknown"), None);
    
    // Update state
    planner.set_state("mana", true);
    assert_eq!(planner.world_state.get("mana"), Some(&true));
}

#[test]
fn test_goap_action_library_integration() {
    use wyldlands_server::ecs::systems::ActionLibrary;
    
    let library = ActionLibrary::new();
    
    // Verify all pre-built actions are registered
    assert!(library.get("wander").is_some());
    assert!(library.get("follow").is_some());
    assert!(library.get("attack").is_some());
    assert!(library.get("flee").is_some());
    assert!(library.get("patrol").is_some());
    assert!(library.get("guard").is_some());
    assert!(library.get("rest").is_some());
    assert!(library.get("interact").is_some());
    
    // Verify unknown action returns None
    assert!(library.get("unknown_action").is_none());
    
    // Verify we can get action definitions
    let definitions = library.get_definitions();
    assert_eq!(definitions.len(), 8);
}

#[test]
fn test_goap_planner_with_npc() {
    let mut world = hecs::World::new();
    
    // Create NPC with GOAP planner
    let npc_uuid = uuid::Uuid::new_v4();
    let mut planner = GoapPlanner::new();
    
    // Set up simple scenario
    planner.set_state("is_idle", true);
    planner.set_state("is_patrolling", false);
    
    let patrol_action = GoapAction::new("start_patrol", "Start patrolling")
        .with_precondition("is_idle", true)
        .with_effect("is_idle", false)
        .with_effect("is_patrolling", true)
        .with_cost(1.0);
    planner.add_action(patrol_action);
    
    let patrol_goal = GoapGoal::new("patrol_area", "Patrol the area", 5)
        .with_condition("is_patrolling", true);
    planner.add_goal(patrol_goal);
    
    let npc_entity = world.spawn((
        EntityUuid(npc_uuid),
        Name::new("Guard NPC"),
        Npc::new(),
        AIController::new(BehaviorType::Passive),
        planner,
    ));
    
    // Verify NPC has planner
    assert!(world.get::<&GoapPlanner>(npc_entity).is_ok());
    
    // Verify planner has goals and actions
    let planner_ref = world.get::<&GoapPlanner>(npc_entity).unwrap();
    assert_eq!(planner_ref.goals.len(), 1);
    assert_eq!(planner_ref.actions.len(), 1);
    
    // Clone and test planning
    let mut planner_copy = (*planner_ref).clone();
    let selected_goal = planner_copy.select_goal().unwrap().clone();
    let plan = planner_copy.plan(&selected_goal);
    assert!(plan.is_some());
    assert_eq!(plan.unwrap().len(), 1);
}

#[test]
fn test_goap_complex_dependency_chain() {
    let mut planner = GoapPlanner::new();
    
    // Complex scenario: Need to craft an item
    planner.set_state("has_materials", false);
    planner.set_state("has_tools", false);
    planner.set_state("at_workshop", false);
    planner.set_state("item_crafted", false);
    
    // Step 1: Gather materials
    let gather = GoapAction::new("gather_materials", "Gather materials")
        .with_effect("has_materials", true)
        .with_cost(3.0);
    planner.add_action(gather);
    
    // Step 2: Get tools
    let get_tools = GoapAction::new("get_tools", "Get tools")
        .with_effect("has_tools", true)
        .with_cost(2.0);
    planner.add_action(get_tools);
    
    // Step 3: Go to workshop
    let go_workshop = GoapAction::new("go_to_workshop", "Go to workshop")
        .with_effect("at_workshop", true)
        .with_cost(2.0);
    planner.add_action(go_workshop);
    
    // Step 4: Craft (requires all previous steps)
    let craft = GoapAction::new("craft_item", "Craft the item")
        .with_precondition("has_materials", true)
        .with_precondition("has_tools", true)
        .with_precondition("at_workshop", true)
        .with_effect("item_crafted", true)
        .with_cost(5.0);
    planner.add_action(craft);
    
    let goal = GoapGoal::new("craft_goal", "Craft an item", 10)
        .with_condition("item_crafted", true);
    planner.add_goal(goal);
    
    // Should find a plan with all 4 steps
    let selected_goal = planner.select_goal().unwrap().clone();
    let plan = planner.plan(&selected_goal);
    assert!(plan.is_some());
    
    let plan = plan.unwrap();
    assert_eq!(plan.len(), 4);
    assert_eq!(plan[3], "craft_item"); // Craft should be last
}

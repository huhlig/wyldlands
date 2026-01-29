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

//! NPC AI system integrating GOAP planning with behavior execution

use crate::ecs::components::*;
use crate::ecs::context::WorldContext;
use crate::llm::{LlmManager, LLMMessage, LLMRequest};
use hecs::Entity;
use std::sync::Arc;

/// NPC AI system for updating NPC behavior
pub struct NpcAiSystem {
    llm_manager: Arc<LlmManager>,
}

impl NpcAiSystem {
    /// Create a new NPC AI system
    pub fn new(llm_manager: Arc<LlmManager>) -> Self {
        Self { llm_manager }
    }

    /// Update all NPCs
    pub async fn update(&self, context: Arc<WorldContext>, delta_time: f32) {
        let world = context.entities().read().await;

        // Collect entities that need updating
        let mut entities_to_update = Vec::new();
        for (entity, npc, ai_controller) in
            world.query::<(Entity, &Npc, &mut AIController)>().iter()
        {
            if npc.active {
                ai_controller.update_timer(delta_time);
                if ai_controller.should_update(delta_time) {
                    entities_to_update.push(entity);
                }
            }
        }

        drop(world);

        // Update each entity
        for entity in entities_to_update {
            self.update_npc(context.clone(), entity).await;
        }
    }

    /// Update a single NPC
    async fn update_npc(&self, context: Arc<WorldContext>, entity: hecs::Entity) {
        let mut world = context.entities().write().await;

        // Get NPC components
        let has_goap = world.get::<&GoapPlanner>(entity).is_ok();

        if has_goap {
            // Use GOAP planning
            self.update_with_goap(&mut world, entity).await;
        } else {
            // Use traditional AI controller
            self.update_traditional(&mut world, entity).await;
        }

        // Mark as updated
        if let Ok(mut ai_controller) = world.get::<&mut AIController>(entity) {
            ai_controller.mark_updated();
        }
    }

    /// Update NPC using GOAP planning
    async fn update_with_goap(&self, world: &mut hecs::World, entity: hecs::Entity) {
        // Get GOAP planner
        let mut planner = match world.get::<&mut GoapPlanner>(entity) {
            Ok(p) => p,
            Err(_) => return,
        };

        // Update planner to select goal and create plan
        if !planner.update() {
            tracing::debug!("NPC {:?}: No valid plan found", entity);
            return;
        }

        // Get next action
        let action_id = match planner.next_action() {
            Some(id) => id,
            None => return,
        };

        tracing::debug!("NPC {:?}: Executing action '{}'", entity, action_id);

        // Execute the action (simplified - in a real implementation,
        // you'd have action handlers)
        if let Some(action) = planner.get_action(&action_id) {
            // Apply action effects to world state
            let mut new_state = planner.world_state.clone();
            action.apply_effects(&mut new_state);
            planner.world_state = new_state;
        }
    }

    /// Update NPC using traditional AI controller
    async fn update_traditional(&self, world: &mut hecs::World, entity: hecs::Entity) {
        let ai_controller = match world.get::<&AIController>(entity) {
            Ok(ai) => ai.clone(),
            Err(_) => return,
        };

        match ai_controller.state_type {
            StateType::Idle => {
                // Check if should start wandering, etc.
                if ai_controller.behavior_type == BehaviorType::Wandering {
                    // TODO: Implement wandering behavior
                    tracing::debug!("NPC {:?}: Wandering", entity);
                }
            }
            StateType::Moving => {
                // TODO: Continue movement
                tracing::debug!("NPC {:?}: Moving", entity);
            }
            StateType::Combat => {
                // TODO: Combat behavior
                tracing::debug!("NPC {:?}: In combat", entity);
            }
            StateType::Fleeing => {
                // TODO: Fleeing behavior
                tracing::debug!("NPC {:?}: Fleeing", entity);
            }
            StateType::Following => {
                // TODO: Following behavior
                tracing::debug!("NPC {:?}: Following", entity);
            }
            StateType::Dialogue => {
                // TODO: Dialogue behavior
                tracing::debug!("NPC {:?}: In dialogue", entity);
            }
        }
    }

    /// Handle NPC dialogue using LLM
    pub async fn handle_dialogue(
        &self,
        context: Arc<WorldContext>,
        npc_entity: hecs::Entity,
        player_entity: hecs::Entity,
        message: String,
    ) -> Result<String, String> {
        // Get all data we need from the world
        enum DialogueData {
            Fallback(String),
            LlmEnabled {
                dialogue_config: NpcDialogue,
                personality: Option<Personality>,
                conversation: NpcConversation,
                player_uuid: uuid::Uuid,
                npc_uuid: uuid::Uuid,
            },
        }

        let data = {
            let world = context.entities().read().await;

            // Get dialogue config
            let dialogue_config = match world.get::<&NpcDialogue>(npc_entity) {
                Ok(c) => (*c).clone(),
                Err(_) => return Err("NPC has no dialogue configuration".to_string()),
            };

            // Check if LLM is enabled
            if !dialogue_config.llm_enabled {
                let fallback = dialogue_config.get_fallback().unwrap_or("...").to_string();
                DialogueData::Fallback(fallback)
            } else {
                // Get personality and conversation
                let personality = world
                    .get::<&Personality>(npc_entity)
                    .ok()
                    .map(|p| (*p).clone());
                let conversation = match world.get::<&NpcConversation>(npc_entity) {
                    Ok(c) => (*c).clone(),
                    Err(_) => NpcConversation::new(),
                };

                // Get UUIDs
                let player_uuid = match world.get::<&EntityId>(player_entity) {
                    Ok(id) => id.uuid(),
                    Err(_) => return Err("Player has no UUID".to_string()),
                };

                let npc_uuid = match world.get::<&EntityId>(npc_entity) {
                    Ok(id) => id.uuid(),
                    Err(_) => return Err("NPC has no UUID".to_string()),
                };

                DialogueData::LlmEnabled {
                    dialogue_config,
                    personality,
                    conversation,
                    player_uuid,
                    npc_uuid,
                }
            }
        };

        // Handle based on data type
        let (dialogue_config, personality, conversation, player_uuid, npc_uuid) = match data {
            DialogueData::Fallback(msg) => return Ok(msg),
            DialogueData::LlmEnabled {
                dialogue_config,
                personality,
                conversation,
                player_uuid,
                npc_uuid,
            } => (
                dialogue_config,
                personality,
                conversation,
                player_uuid,
                npc_uuid,
            ),
        };

        // Build LLM request
        let mut request = LLMRequest::new(&dialogue_config.llm_model)
            .with_temperature(dialogue_config.temperature)
            .with_max_tokens(dialogue_config.max_tokens);

        // Add system prompt
        let mut system_prompt = dialogue_config.system_prompt.clone();
        if let Some(p) = personality {
            system_prompt.push_str(&format!(
                "\n\nYour background: {}\nYour speaking style: {}",
                p.background, p.speaking_style
            ));
        }
        request = request.with_message(LLMMessage::system(system_prompt));

        // Add conversation history
        let history = conversation.get_recent(player_uuid, dialogue_config.history_limit);
        for msg in history {
            let role = if msg.speaker == npc_uuid {
                LLMMessage::assistant(&msg.message)
            } else {
                LLMMessage::user(&msg.message)
            };
            request = request.with_message(role);
        }

        // Add current message
        request = request.with_message(LLMMessage::user(&message));

        // Send to LLM
        let response = if let Some(provider) = &dialogue_config.llm_provider {
            self.llm_manager
                .complete_with_provider(provider, request)
                .await
        } else {
            self.llm_manager.complete(request).await
        };

        match response {
            Ok(resp) => {
                // Update conversation history
                let mut world = context.entities().write().await;
                if let Ok(mut conv) = world.get::<&mut NpcConversation>(npc_entity) {
                    conv.add_message(player_uuid, player_uuid, message);
                    conv.add_message(player_uuid, npc_uuid, resp.content.clone());
                }
                Ok(resp.content)
            }
            Err(e) => {
                tracing::error!("LLM error: {}", e);
                Ok(dialogue_config
                    .get_fallback()
                    .unwrap_or("I'm having trouble thinking right now.")
                    .to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_npc_ai_system_creation() {
        let llm_manager = Arc::new(LlmManager::new());
        let system = NpcAiSystem::new(llm_manager);
        // System should be created successfully
        assert!(true);
    }
}



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

//! Event type definitions

use crate::ecs::EcsEntity;
use serde::{Deserialize, Serialize};

/// All possible game events
///
/// Note: Events use EcsEntity (hecs::Entity) rather than EntityId because:
/// 1. Events are internal runtime events, not persisted
/// 2. Systems emitting events typically only have access to EcsEntity
/// 3. Event handlers can look up EntityId from registry if needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // Entity lifecycle
    EntitySpawned {
        entity: EcsEntity,
        entity_type: String,
        location: Option<(EcsEntity, EcsEntity)>,
    },
    EntityDespawned {
        entity: EcsEntity,
    },

    // Movement
    EntityMoved {
        entity: EcsEntity,
        from: (uuid::Uuid, uuid::Uuid),  // (area_id, room_id)
        to: (uuid::Uuid, uuid::Uuid),    // (area_id, room_id)
    },
    EntityEnteredRoom {
        entity: EcsEntity,
        room: EcsEntity,
    },
    EntityLeftRoom {
        entity: EcsEntity,
        room: EcsEntity,
    },

    // Combat
    CombatStarted {
        attacker: EcsEntity,
        defender: EcsEntity,
    },
    CombatEnded {
        participants: Vec<EcsEntity>,
    },
    EntityAttacked {
        attacker: EcsEntity,
        defender: EcsEntity,
        damage: i32,
    },
    EntityDefended {
        entity: EcsEntity,
    },
    EntityFled {
        entity: EcsEntity,
    },
    EntityDied {
        entity: EcsEntity,
        killer: Option<EcsEntity>,
    },

    // Items
    ItemPickedUp {
        entity: EcsEntity,
        item: EcsEntity,
    },
    ItemDropped {
        entity: EcsEntity,
        item: EcsEntity,
    },
    ItemUsed {
        entity: EcsEntity,
        item: EcsEntity,
    },
    ItemEquipped {
        entity: EcsEntity,
        item: EcsEntity,
        slot: String,
    },
    ItemUnequipped {
        entity: EcsEntity,
        item: EcsEntity,
        slot: String,
    },

    // Commands
    CommandExecuted {
        entity: EcsEntity,
        command: String,
        success: bool,
    },

    // Communication
    MessageSent {
        sender: EcsEntity,
        recipients: Vec<EcsEntity>,
        message: String,
        channel: MessageChannel,
    },

    // Experience and progression
    ExperienceGained {
        entity: EcsEntity,
        amount: u64,
    },
    LevelUp {
        entity: EcsEntity,
        new_level: u32,
    },

    // Custom events
    Custom {
        event_type: String,
        data: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageChannel {
    Say,
    Tell,
    Shout,
    Emote,
    System,
    Group,
    Guild,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = GameEvent::Custom {
            event_type: "test".to_string(),
            data: "test data".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: GameEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            GameEvent::Custom { event_type, data } => {
                assert_eq!(event_type, "test");
                assert_eq!(data, "test data");
            }
            _ => panic!("Wrong event type"),
        }
    }
}



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

//! Integration tests for builder commands (area/room editor)

use hecs::Entity;
use wyldlands_server::ecs::{
    EcsEntity, components,
    context::WorldContext,
    events::EventBus,
    systems::{CommandResult, CommandSystem},
};
use wyldlands_server::persistence::PersistenceManager;

/// Helper function to create a test context with database
async fn create_test_context() -> Option<std::sync::Arc<WorldContext>> {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost/test".to_string());

    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
    {
        Ok(pool) => pool,
        Err(_) => {
            eprintln!("Skipping test - no test database available");
            return None;
        }
    };

    let persistence_manager = std::sync::Arc::new(PersistenceManager::new(pool, 60));
    Some(std::sync::Arc::new(WorldContext::new(persistence_manager)))
}

/// Helper function to create a test builder entity
async fn create_test_builder(context: &std::sync::Arc<WorldContext>) -> EcsEntity {
    let mut world = context.entities().write().await;
    let builder_uuid = components::EntityUuid::new();
    let area_id = uuid::Uuid::new_v4();
    let room_id = uuid::Uuid::new_v4();

    world.spawn((
        builder_uuid,
        components::Name::new("TestBuilder"),
        components::Location::new(
            components::EntityId::from_uuid(area_id),
            components::EntityId::from_uuid(room_id),
        ),
    ))
}

/// Helper function to extract UUID from command result message
fn extract_uuid_from_result(result: &CommandResult) -> String {
    match result {
        CommandResult::Success(msg) => {
            // Extract UUID from messages like "Area created successfully: <uuid>"
            if let Some(uuid_start) = msg.rfind(": ") {
                let uuid_str = &msg[uuid_start + 2..];
                // Find the end of the UUID (first whitespace or end of string)
                let uuid_end = uuid_str
                    .find(|c: char| c.is_whitespace())
                    .unwrap_or(uuid_str.len());
                uuid_str[..uuid_end].trim().to_string()
            } else if let Some(uuid_start) = msg.find("UUID: ") {
                let uuid_str = &msg[uuid_start + 6..];
                let uuid_end = uuid_str
                    .find(|c: char| c.is_whitespace())
                    .unwrap_or(uuid_str.len());
                uuid_str[..uuid_end].trim().to_string()
            } else {
                // Try to find any UUID pattern in the message
                let words: Vec<&str> = msg.split_whitespace().collect();
                for word in words {
                    if word.len() == 36 && word.contains('-') {
                        return word.to_string();
                    }
                }
                panic!("Could not extract UUID from result: {:?}", msg);
            }
        }
        _ => panic!("Expected success result, got: {:?}", result),
    }
}

/// Helper function to move builder to a specific room
async fn move_builder_to_room(
    context: &std::sync::Arc<WorldContext>,
    builder: EcsEntity,
    room_uuid: &str,
) {
    let room_uuid = uuid::Uuid::parse_str(room_uuid).expect("Invalid room UUID");
    let mut world = context.entities().write().await;

    // Find the room entity
    let mut room_entity = None;
    for (entity, entity_uuid) in world.query::<(Entity, &components::EntityUuid)>().iter() {
        if entity_uuid.0 == room_uuid {
            room_entity = Some(entity);
            break;
        }
    }

    let room_entity = room_entity.expect("Room not found");

    // Get the room's area
    let area_id = if let Ok(room) = world.get::<&components::Room>(room_entity) {
        room.area_id
    } else {
        panic!("Entity is not a room");
    };

    // Update builder's location
    if let Ok(mut location) = world.get::<&mut components::Location>(builder) {
        location.area_id = area_id;
        location.room_id = components::EntityId::from_uuid(room_uuid);
    }
}

#[tokio::test]
async fn test_area_create_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Test successful area creation
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec![
                "create".to_string().to_string(),
                "Test Area".to_string().to_string(),
            ],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Area created successfully"));
            assert!(msg.contains("Test Area"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Test area creation without name
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string()],
        )
        .await;

    match result {
        CommandResult::Failure(msg) => {
            assert!(msg.contains("Usage"));
        }
        _ => panic!("Expected failure for missing name"),
    }
}

#[tokio::test]
async fn test_area_list_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create a test area first
    let _ = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "List Test Area".to_string()],
        )
        .await;

    // Test area list
    let result = command_system
        .execute(context.clone(), builder, "area", &vec!["list".to_string()])
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Areas"));
            assert!(msg.contains("List Test Area"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_area_info_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create a test area
    let create_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Info Test Area".to_string()],
        )
        .await;

    // Extract UUID from success message
    let uuid_str = if let CommandResult::Success(msg) = create_result {
        // Parse UUID from message like "UUID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID in message")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    // Test area info
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["info".to_string(), uuid_str.clone()],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Area Information"));
            assert!(msg.contains("Info Test Area"));
            assert!(msg.contains(&uuid_str));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Test with invalid UUID
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["info".to_string(), "invalid-uuid".to_string()],
        )
        .await;

    match result {
        CommandResult::Failure(msg) => {
            assert!(msg.contains("Invalid UUID"));
        }
        _ => panic!("Expected failure for invalid UUID"),
    }
}

#[tokio::test]
async fn test_area_edit_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create a test area
    let create_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Edit Test Area".to_string()],
        )
        .await;

    let uuid_str = if let CommandResult::Success(msg) = create_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    // Test editing name
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec![
                "edit".to_string(),
                uuid_str.clone(),
                "name".to_string(),
                "Renamed Area".to_string(),
            ],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("updated successfully"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Verify the change
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["info".to_string(), uuid_str.clone()],
        )
        .await;

    if let CommandResult::Success(msg) = result {
        assert!(msg.contains("Renamed Area"));
    }
}

#[tokio::test]
async fn test_area_delete_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create a test area
    let create_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Delete Test Area".to_string()],
        )
        .await;

    let uuid_str = if let CommandResult::Success(msg) = create_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    // Test deletion
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["delete".to_string(), uuid_str.clone()],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("deleted successfully"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Verify it's gone
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["info".to_string(), uuid_str.clone()],
        )
        .await;

    match result {
        CommandResult::Failure(msg) => {
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected failure after deletion"),
    }
}

#[tokio::test]
async fn test_room_create_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create an area first
    let area_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Room Test Area".to_string()],
        )
        .await;

    let area_uuid = if let CommandResult::Success(msg) = area_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    // Create a room
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Test Room".to_string(),
            ],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Room created successfully"));
            assert!(msg.contains("Test Room"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Test with invalid area UUID
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                "invalid-uuid".to_string(),
                "Test Room".to_string(),
            ],
        )
        .await;

    match result {
        CommandResult::Failure(msg) => {
            assert!(msg.contains("Invalid UUID"));
        }
        _ => panic!("Expected failure for invalid UUID"),
    }
}

#[tokio::test]
async fn test_room_list_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create area and room
    let area_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "List Room Area".to_string()],
        )
        .await;

    let area_uuid = if let CommandResult::Success(msg) = area_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    let _ = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Listed Room".to_string(),
            ],
        )
        .await;

    // Test room list
    let result = command_system
        .execute(context.clone(), builder, "room", &vec!["list".to_string()])
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Rooms"));
            assert!(msg.contains("Listed Room"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Test filtered list
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec!["list".to_string(), area_uuid.clone()],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Listed Room"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_room_goto_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create area and room
    let area_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Goto Test Area".to_string()],
        )
        .await;

    let area_uuid = if let CommandResult::Success(msg) = area_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    let room_result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Destination Room".to_string(),
            ],
        )
        .await;

    let room_uuid = if let CommandResult::Success(msg) = room_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Room creation failed");
    };

    // Test goto
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec!["goto".to_string(), room_uuid.clone()],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Teleported"));
            assert!(msg.contains("Destination Room"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_exit_commands() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create area and two rooms
    let area_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Exit Test Area".to_string()],
        )
        .await;

    let area_uuid = if let CommandResult::Success(msg) = area_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    let room1_result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Room 1".to_string(),
            ],
        )
        .await;

    let room1_uuid = if let CommandResult::Success(msg) = room1_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Room creation failed");
    };

    let room2_result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Room 2".to_string(),
            ],
        )
        .await;

    let room2_uuid = if let CommandResult::Success(msg) = room2_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Room creation failed");
    };

    // Go to room 1
    let _ = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec!["goto".to_string(), room1_uuid.clone()],
        )
        .await;

    // Add exit from room 1 to room 2
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "exit",
            &vec!["add".to_string(), "north".to_string(), room2_uuid.clone()],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Exit added successfully"));
            assert!(msg.contains("north"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // List exits
    let result = command_system
        .execute(context.clone(), builder, "exit", &vec!["list".to_string()])
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("north"));
            assert!(msg.contains(&room2_uuid));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Remove exit
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "exit",
            &vec!["remove".to_string(), "north".to_string()],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Exit removed successfully"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_dig_command() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create area and starting room
    let area_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Dig Test Area".to_string()],
        )
        .await;

    let area_uuid = if let CommandResult::Success(msg) = area_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    let room_result = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Starting Room".to_string(),
            ],
        )
        .await;

    let room_uuid = if let CommandResult::Success(msg) = room_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Room creation failed");
    };

    // Go to starting room
    let _ = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec!["goto".to_string(), room_uuid.clone()],
        )
        .await;

    // Dig a new room to the east
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "dig",
            &vec![
                "east".to_string(),
                area_uuid.clone(),
                "Dug Room".to_string(),
            ],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Room created and exits added"));
            assert!(msg.contains("Dug Room"));
            assert!(msg.contains("east"));
            assert!(msg.contains("west")); // Reverse direction
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Verify exits were created
    let result = command_system
        .execute(context.clone(), builder, "exit", &vec!["list".to_string()])
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("east"));
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // Test oneway dig
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "dig",
            &vec![
                "south".to_string(),
                area_uuid.clone(),
                "Oneway Room".to_string(),
                "oneway".to_string(),
            ],
        )
        .await;

    match result {
        CommandResult::Success(msg) => {
            assert!(msg.contains("Room created"));
            assert!(msg.contains("south"));
            assert!(!msg.contains("north")); // No reverse
        }
        _ => panic!("Expected success, got: {:?}", result),
    }

    // ============================================================================
    // Phase 2: Advanced Builder Features Tests
    // ============================================================================

    #[tokio::test]
    async fn test_room_edit_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Create area and room
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Original Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);

        // Test editing room name
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "edit".to_string(),
                    room_uuid.clone(),
                    "name".to_string(),
                    "New Room Name".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Room name updated"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Test editing room description
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "edit".to_string(),
                    room_uuid.clone(),
                    "description".to_string(),
                    "A newly described room".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Room description updated"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_exit_edit_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Create area and rooms
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room1_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Room 1".to_string(),
                ],
            )
            .await;

        let room1_uuid = extract_uuid_from_result(&room1_result);

        let room2_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Room 2".to_string(),
                ],
            )
            .await;

        let room2_uuid = extract_uuid_from_result(&room2_result);

        // Move builder to room1
        move_builder_to_room(&context, builder, &room1_uuid).await;

        // Create exit
        let _result = command_system
            .execute(
                context.clone(),
                builder,
                "exit",
                &vec![
                    "create".to_string(),
                    "north".to_string(),
                    room2_uuid.clone(),
                ],
            )
            .await;

        // Test making exit closeable
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "exit",
                &vec![
                    "edit".to_string(),
                    "north".to_string(),
                    "closeable".to_string(),
                    "true".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("closeable set to true"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Test making exit lockable
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "exit",
                &vec![
                    "edit".to_string(),
                    "north".to_string(),
                    "lockable".to_string(),
                    "true".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("lockable set to true"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Test setting door rating
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "exit",
                &vec![
                    "edit".to_string(),
                    "north".to_string(),
                    "door_rating".to_string(),
                    "5".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("door rating set to 5"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_area_search_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Create multiple areas
        let _result1 = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Dark Forest".to_string()],
            )
            .await;

        let _result2 = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Bright Plains".to_string()],
            )
            .await;

        let _result3 = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Dark Cave".to_string()],
            )
            .await;

        // Search for "dark"
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["search".to_string(), "dark".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Dark Forest"));
                assert!(msg.contains("Dark Cave"));
                assert!(!msg.contains("Bright Plains"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_room_search_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Create area
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        // Create multiple rooms
        let _result1 = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Ancient Temple".to_string(),
                ],
            )
            .await;

        let _result2 = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Temple Courtyard".to_string(),
                ],
            )
            .await;

        let _result3 = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Dark Corridor".to_string(),
                ],
            )
            .await;

        // Search for "temple"
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec!["search".to_string(), "temple".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Ancient Temple"));
                assert!(msg.contains("Temple Courtyard"));
                assert!(!msg.contains("Dark Corridor"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_room_delete_bulk_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Create area
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        // Create multiple rooms
        for i in 1..=5 {
            let _result = command_system
                .execute(
                    context.clone(),
                    builder,
                    "room",
                    &vec![
                        "create".to_string(),
                        area_uuid.clone(),
                        format!("Room {}", i),
                    ],
                )
                .await;
        }

        // Verify rooms exist
        let list_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec!["list".to_string(), area_uuid.clone()],
            )
            .await;

        match list_result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Room 1"));
                assert!(msg.contains("Room 5"));
            }
            _ => panic!("Expected success"),
        }

        // Delete all rooms
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec!["delete".to_string(), "bulk".to_string(), area_uuid.clone()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("5 rooms"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    // ============================================================================
    // Phase 3: Item Editor Tests
    // ============================================================================

    #[tokio::test]
    async fn test_item_create_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Create area and room
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Test Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);
        move_builder_to_room(&context, builder, &room_uuid).await;

        // Create basic item
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "create".to_string(),
                    "Test Sword | A basic sword | 3.5".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item created"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Create weapon item
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "create".to_string(),
                    "Steel Sword | A steel sword | 4.0 | weapon 5 10 slashing".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item created"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Create armor item
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "create".to_string(),
                    "Leather Armor | Sturdy armor | 8.0 | armor 3".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item created"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_item_edit_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Setup
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Test Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);
        move_builder_to_room(&context, builder, &room_uuid).await;

        // Create item
        let item_result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "create".to_string(),
                    "Test Item | A test item | 1.0".to_string(),
                ],
            )
            .await;

        let item_uuid = extract_uuid_from_result(&item_result);

        // Edit name
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "edit".to_string(),
                    item_uuid.clone(),
                    "name".to_string(),
                    "New Name".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item name updated"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Edit weight
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "edit".to_string(),
                    item_uuid.clone(),
                    "weight".to_string(),
                    "2.5".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item weight set to 2.5"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_item_clone_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Setup
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Test Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);
        move_builder_to_room(&context, builder, &room_uuid).await;

        // Create item
        let item_result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "create".to_string(),
                    "Original Item | An original item | 1.0 | weapon 3 6 slashing".to_string(),
                ],
            )
            .await;

        let item_uuid = extract_uuid_from_result(&item_result);

        // Clone without new name
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["clone".to_string(), item_uuid.clone()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item cloned"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Clone with new name
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "clone".to_string(),
                    item_uuid.clone(),
                    "Cloned Item".to_string(),
                ],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Item cloned"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_item_list_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Setup
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Test Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);
        move_builder_to_room(&context, builder, &room_uuid).await;

        // Create items
        let _result1 = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["create".to_string(), "Sword | A sword | 3.0".to_string()],
            )
            .await;

        let _result2 = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["create".to_string(), "Shield | A shield | 5.0".to_string()],
            )
            .await;

        // List all items
        let result = command_system
            .execute(context.clone(), builder, "item", &vec!["list".to_string()])
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Sword"));
                assert!(msg.contains("Shield"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // List with filter
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["list".to_string(), "sword".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Sword"));
                assert!(!msg.contains("Shield"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_item_info_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Setup
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Test Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);
        move_builder_to_room(&context, builder, &room_uuid).await;

        // Create item with weapon stats
        let item_result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec![
                    "create".to_string(),
                    "Magic Sword | A magical sword | 4.0 | weapon 5 10 arcane".to_string(),
                ],
            )
            .await;

        let item_uuid = extract_uuid_from_result(&item_result);

        // Get item info
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["info".to_string(), item_uuid.clone()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Magic Sword"));
                assert!(msg.contains("4.0 lbs"));
                assert!(msg.contains("Weapon"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    // ============================================================================
    // Phase 4: Item Template System Tests
    // ============================================================================

    #[tokio::test]
    async fn test_item_spawn_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // Setup
        let area_result = command_system
            .execute(
                context.clone(),
                builder,
                "area",
                &vec!["create".to_string(), "Test Area".to_string()],
            )
            .await;

        let area_uuid = extract_uuid_from_result(&area_result);

        let room_result = command_system
            .execute(
                context.clone(),
                builder,
                "room",
                &vec![
                    "create".to_string(),
                    area_uuid.clone(),
                    "Test Room".to_string(),
                ],
            )
            .await;

        let room_uuid = extract_uuid_from_result(&room_result);
        move_builder_to_room(&context, builder, &room_uuid).await;

        // Spawn single item
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["spawn".to_string(), "longsword".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Spawned"));
                assert!(msg.contains("Long Sword"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Spawn multiple items
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["spawn".to_string(), "potion".to_string(), "5".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Spawned"));
                assert!(msg.contains("x5"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Test invalid template
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["spawn".to_string(), "nonexistent".to_string()],
            )
            .await;

        match result {
            CommandResult::Failure(msg) => {
                assert!(msg.contains("Unknown template"));
            }
            _ => panic!("Expected failure, got: {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_item_templates_command() {
        let context = match create_test_context().await {
            Some(ctx) => ctx,
            None => return,
        };

        let builder = create_test_builder(&context).await;
        let event_bus = EventBus::new();
        let mut command_system = CommandSystem::new(event_bus);

        // List all templates
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["templates".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("Weapons:"));
                assert!(msg.contains("Armor:"));
                assert!(msg.contains("longsword"));
                assert!(msg.contains("leather_armor"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }

        // Filter templates
        let result = command_system
            .execute(
                context.clone(),
                builder,
                "item",
                &vec!["templates".to_string(), "sword".to_string()],
            )
            .await;

        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("sword"));
                assert!(!msg.contains("armor"));
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }
}

#[tokio::test]
async fn test_area_delete_with_rooms_fails() {
    let Some(context) = create_test_context().await else {
        return;
    };
    let builder = create_test_builder(&context).await;
    let event_bus = EventBus::new();
    let mut command_system = CommandSystem::new(event_bus);

    // Create area with a room
    let area_result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["create".to_string(), "Protected Area".to_string()],
        )
        .await;

    let area_uuid = if let CommandResult::Success(msg) = area_result {
        msg.lines()
            .find(|line| line.contains("UUID:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Should have UUID")
            .to_string()
    } else {
        panic!("Area creation failed");
    };

    // Create a room in the area
    let _ = command_system
        .execute(
            context.clone(),
            builder,
            "room",
            &vec![
                "create".to_string(),
                area_uuid.clone(),
                "Protected Room".to_string(),
            ],
        )
        .await;

    // Try to delete area with rooms
    let result = command_system
        .execute(
            context.clone(),
            builder,
            "area",
            &vec!["delete".to_string(), area_uuid.clone()],
        )
        .await;

    match result {
        CommandResult::Failure(msg) => {
            assert!(msg.contains("Cannot delete area"));
            assert!(msg.contains("rooms"));
        }
        _ => panic!("Expected failure when deleting area with rooms"),
    }
}



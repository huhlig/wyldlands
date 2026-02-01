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

//! Integration tests for the Memory System
//!
//! These tests verify the complete memory persistence layer including:
//! - Memory creation and storage
//! - Memory retrieval and querying
//! - Memory updates and deletion
//! - Memory importance and decay
//! - Entity relationships
//! - Error handling

use chrono::Utc;
use sqlx::PgPool;
use wyldlands_server::ecs::components::EntityId;
use wyldlands_server::ecs::memory::{
    MemoryConfig, MemoryError, MemoryId, MemoryKind, MemoryResource, MemoryTagMode,
};

/// Helper to create a test database pool
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://wyldlands:wyldlands@localhost/wyldlands_test".to_string()
    });

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Helper to create a test entity
fn create_test_entity() -> EntityId {
    EntityId::from_uuid(uuid::Uuid::new_v4())
}

/// Helper to clean up test memories for an entity
async fn cleanup_entity_memories(pool: &PgPool, entity_id: EntityId) {
    let _ = sqlx::query("DELETE FROM wyldlands.entity_memory WHERE entity_id = $1")
        .bind(entity_id.uuid())
        .execute(pool)
        .await;
}

#[tokio::test]
async fn test_memory_resource_creation() {
    let pool = setup_test_db().await;

    // Test default configuration
    let memory = MemoryResource::new(pool.clone());
    assert_eq!(memory.config().max_tokens, 4096);
    assert_eq!(memory.config().max_recall_results, 10);

    // Test custom configuration
    let config = MemoryConfig {
        max_tokens: 2048,
        max_recall_results: 5,
        similarity_threshold: 0.8,
        max_memories_per_entity: 500,
        min_importance_threshold: 0.2,
        consolidation_threshold: 400,
        base_decay_rate: 0.02,
        cache_max_capacity: 1000,
        cache_ttl_seconds: 300,
        cache_tti_seconds: 60,
        embedding_cache_capacity: 100,
        embedding_cache_ttl_seconds: 600,
    };

    let memory = MemoryResource::with_config(pool, config.clone());
    assert_eq!(memory.config().max_tokens, 2048);
    assert_eq!(memory.config().max_recall_results, 5);
    assert_eq!(memory.config().similarity_threshold, 0.8);
}

#[tokio::test]
async fn test_retain_memory_basic() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    // Clean up any existing test data
    cleanup_entity_memories(&pool, entity_id).await;

    // Create a basic memory
    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Met a friendly merchant in the tavern",
            Utc::now(),
            Some("social interaction"),
            [("location", "tavern"), ("mood", "friendly")],
            [],
            ["social", "merchant", "tavern"],
        )
        .await
        .expect("Failed to create memory");

    // Verify memory was created
    let retrieved = memory
        .get_memory(memory_id)
        .await
        .expect("Failed to retrieve memory");

    assert_eq!(retrieved.entity_id.uuid(), entity_id.uuid());
    assert_eq!(retrieved.kind, MemoryKind::Experience);
    assert_eq!(retrieved.content, "Met a friendly merchant in the tavern");
    assert_eq!(retrieved.context, Some("social interaction".to_string()));
    assert_eq!(
        retrieved.metadata.get("location"),
        Some(&"tavern".to_string())
    );
    assert_eq!(
        retrieved.metadata.get("mood"),
        Some(&"friendly".to_string())
    );
    assert!(retrieved.tags.contains("social"));
    assert!(retrieved.tags.contains("merchant"));
    assert!(retrieved.tags.contains("tavern"));

    // Clean up
    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_retain_memory_with_entities() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();
    let merchant_id = create_test_entity();
    let guard_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memory with involved entities
    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Witnessed a dispute between merchant and guard",
            Utc::now(),
            Some("conflict"),
            [("severity", "moderate")],
            [(merchant_id, "merchant"), (guard_id, "guard")],
            ["conflict", "witness"],
        )
        .await
        .expect("Failed to create memory");

    // Verify entities were stored
    let retrieved = memory.get_memory(memory_id).await.unwrap();
    assert_eq!(retrieved.entities.len(), 2);
    assert_eq!(
        retrieved.entities.get(&merchant_id),
        Some(&"merchant".to_string())
    );
    assert_eq!(
        retrieved.entities.get(&guard_id),
        Some(&"guard".to_string())
    );

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_retain_memory_validation() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool);
    let entity_id = create_test_entity();

    // Test empty content
    let result = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidContent(_))));

    // Test whitespace-only content
    let result = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "   \n\t  ",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidContent(_))));

    // Test content too long (>10000 chars)
    let long_content = "a".repeat(10001);
    let result = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            &long_content,
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await;

    assert!(matches!(result, Err(MemoryError::InvalidContent(_))));
}

#[tokio::test]
async fn test_list_memories() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create multiple memories
    let memories_to_create = vec![
        ("First memory", MemoryKind::Experience),
        ("Second memory", MemoryKind::World),
        ("Third memory", MemoryKind::Opinion),
    ];

    for (content, kind) in memories_to_create {
        memory
            .retain(entity_id, kind, content, Utc::now(), None, [], [], [])
            .await
            .expect("Failed to create memory");
    }

    // List all memories
    let memories = memory
        .list_memories(entity_id)
        .await
        .expect("Failed to list memories");

    assert_eq!(memories.len(), 3);

    // Verify they're sorted by timestamp (newest first)
    assert_eq!(memories[0].content, "Third memory");
    assert_eq!(memories[1].content, "Second memory");
    assert_eq!(memories[2].content, "First memory");

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_count_memories() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Initially should be 0
    let count = memory.count_memories(entity_id).await.unwrap();
    assert_eq!(count, 0);

    // Add some memories
    for i in 0..5 {
        memory
            .retain(
                entity_id,
                MemoryKind::Experience,
                &format!("Memory {}", i),
                Utc::now(),
                None,
                [],
                [],
                [],
            )
            .await
            .unwrap();
    }

    // Should now be 5
    let count = memory.count_memories(entity_id).await.unwrap();
    assert_eq!(count, 5);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_get_memory_not_found() {
    let pool = setup_test_db().await;
    let memory = MemoryResource::new(pool);

    let non_existent_id = MemoryId::new();
    let result = memory.get_memory(non_existent_id).await;

    assert!(matches!(result, Err(MemoryError::NotFound(_))));
}

#[tokio::test]
async fn test_alter_memory_content() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create a memory
    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Original content",
            Utc::now(),
            Some("original context"),
            [],
            [],
            ["original"],
        )
        .await
        .unwrap();

    // Update content
    memory
        .alter_memory(memory_id.clone(), Some("Updated content"), None, None)
        .await
        .expect("Failed to update memory");

    // Verify update
    let retrieved = memory.get_memory(memory_id).await.unwrap();
    assert_eq!(retrieved.content, "Updated content");
    assert_eq!(retrieved.context, Some("original context".to_string()));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_alter_memory_context() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Test content",
            Utc::now(),
            Some("old context"),
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Update context
    memory
        .alter_memory(memory_id.clone(), None, Some("new context"), None)
        .await
        .unwrap();

    let retrieved = memory.get_memory(memory_id).await.unwrap();
    assert_eq!(retrieved.context, Some("new context".to_string()));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_alter_memory_tags() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Test content",
            Utc::now(),
            None,
            [],
            [],
            ["old", "tags"],
        )
        .await
        .unwrap();

    // Update tags
    memory
        .alter_memory(
            memory_id.clone(),
            None,
            None,
            Some(&["new", "tags", "updated"]),
        )
        .await
        .unwrap();

    let retrieved = memory.get_memory(memory_id).await.unwrap();
    assert_eq!(retrieved.tags.len(), 3);
    assert!(retrieved.tags.contains("new"));
    assert!(retrieved.tags.contains("tags"));
    assert!(retrieved.tags.contains("updated"));
    assert!(!retrieved.tags.contains("old"));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_alter_memory_validation() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Original content",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Try to update with empty content
    let result = memory
        .alter_memory(memory_id.clone(), Some(""), None, None)
        .await;
    assert!(matches!(result, Err(MemoryError::InvalidContent(_))));

    // Try to update with too long content
    let long_content = "a".repeat(10001);
    let result = memory
        .alter_memory(memory_id.clone(), Some(&long_content), None, None)
        .await;
    assert!(matches!(result, Err(MemoryError::InvalidContent(_))));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_delete_memory() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create a memory
    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "To be deleted",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Verify it exists
    assert!(memory.get_memory(memory_id.clone()).await.is_ok());

    // Delete it
    memory
        .delete_memory(memory_id.clone())
        .await
        .expect("Failed to delete memory");

    // Verify it's gone
    let result = memory.get_memory(memory_id).await;
    assert!(matches!(result, Err(MemoryError::NotFound(_))));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_delete_memory_not_found() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool);

    let non_existent_id = MemoryId::new();
    let result = memory.delete_memory(non_existent_id).await;

    assert!(matches!(result, Err(MemoryError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_memory_cascades_entities() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();
    let other_entity = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memory with entity relationships
    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Memory with entities",
            Utc::now(),
            None,
            [],
            [(other_entity, "participant")],
            [],
        )
        .await
        .unwrap();

    // Verify entity relationship exists
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM wyldlands.entity_memory_entities WHERE memory_id = $1",
    )
    .bind(memory_id.uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1);

    // Delete memory
    memory.delete_memory(memory_id.clone()).await.unwrap();

    // Verify entity relationship was also deleted (cascade)
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM wyldlands.entity_memory_entities WHERE memory_id = $1",
    )
    .bind(memory_id.uuid())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_memory_kinds() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create one of each kind
    let kinds = vec![
        MemoryKind::World,
        MemoryKind::Experience,
        MemoryKind::Opinion,
        MemoryKind::Observation,
    ];

    for kind in &kinds {
        memory
            .retain(
                entity_id,
                *kind,
                &format!("{:?} memory", kind),
                Utc::now(),
                None,
                [],
                [],
                [],
            )
            .await
            .unwrap();
    }

    // Retrieve and verify all kinds
    let memories = memory.list_memories(entity_id).await.unwrap();
    assert_eq!(memories.len(), 4);

    for kind in kinds {
        assert!(memories.iter().any(|m| m.kind == kind));
    }

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_memory_importance_calculation() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Test memory",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    let mut node = memory.get_memory(memory_id).await.unwrap();

    // Test initial importance
    let initial_importance = node.calculate_current_importance(Utc::now());
    assert!(initial_importance > 0.0 && initial_importance <= 1.0);

    // Test access boost
    node.mark_accessed(Utc::now());
    node.mark_accessed(Utc::now());
    let boosted_importance = node.calculate_current_importance(Utc::now());
    assert!(boosted_importance > initial_importance);

    // Test should_prune
    assert!(!node.should_prune(Utc::now(), 0.1));
    assert!(node.should_prune(Utc::now(), 1.0));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_memory_isolation_between_entities() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity1 = create_test_entity();
    let entity2 = create_test_entity();

    cleanup_entity_memories(&pool, entity1).await;
    cleanup_entity_memories(&pool, entity2).await;

    // Create memories for entity1
    memory
        .retain(
            entity1,
            MemoryKind::Experience,
            "Entity 1 memory",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Create memories for entity2
    memory
        .retain(
            entity2,
            MemoryKind::Experience,
            "Entity 2 memory",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Verify isolation
    let entity1_memories = memory.list_memories(entity1).await.unwrap();
    let entity2_memories = memory.list_memories(entity2).await.unwrap();

    assert_eq!(entity1_memories.len(), 1);
    assert_eq!(entity2_memories.len(), 1);
    assert_eq!(entity1_memories[0].content, "Entity 1 memory");
    assert_eq!(entity2_memories[0].content, "Entity 2 memory");

    cleanup_entity_memories(&pool, entity1).await;
    cleanup_entity_memories(&pool, entity2).await;
}

#[tokio::test]
async fn test_concurrent_memory_operations() {
    let pool = setup_test_db().await;
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create multiple memories concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let pool_clone = pool.clone();
        let entity_id_clone = entity_id;

        let handle = tokio::spawn(async move {
            let mut memory = MemoryResource::new(pool_clone);
            memory
                .retain(
                    entity_id_clone,
                    MemoryKind::Experience,
                    &format!("Concurrent memory {}", i),
                    Utc::now(),
                    None,
                    [],
                    [],
                    [],
                )
                .await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().expect("Failed to create memory");
    }

    // Verify all were created
    let memory = MemoryResource::new(pool.clone());
    let count = memory.count_memories(entity_id).await.unwrap();
    assert_eq!(count, 10);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_recall_with_text_matching() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memories with different content
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Fought a dragon in the mountains",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "dragon"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Met a friendly merchant in the tavern",
            Utc::now(),
            None,
            [],
            [],
            ["social", "merchant"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Defeated a dragon near the castle",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "dragon"],
        )
        .await
        .unwrap();

    // Recall memories about dragons
    let results = memory
        .recall(entity_id, "dragon", [], [], MemoryTagMode::Any)
        .await
        .unwrap();

    // Should return dragon-related memories
    assert!(results.len() >= 2);
    assert!(results.iter().any(|m| m.content.contains("dragon")));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_recall_with_kind_filtering() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create different kinds of memories
    memory
        .retain(
            entity_id,
            MemoryKind::World,
            "Dragons are powerful creatures",
            Utc::now(),
            None,
            [],
            [],
            ["dragon"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "I fought a dragon yesterday",
            Utc::now(),
            None,
            [],
            [],
            ["dragon", "combat"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Opinion,
            "I think dragons are dangerous",
            Utc::now(),
            None,
            [],
            [],
            ["dragon"],
        )
        .await
        .unwrap();

    // Recall only experiences
    let results = memory
        .recall(
            entity_id,
            "dragon",
            [MemoryKind::Experience],
            [],
            MemoryTagMode::Any,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, MemoryKind::Experience);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_recall_with_tag_filtering() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memories with different tags
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Combat with dragon",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "dragon"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Social interaction",
            Utc::now(),
            None,
            [],
            [],
            ["social"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Combat with goblin",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "goblin"],
        )
        .await
        .unwrap();

    // Recall only combat memories
    let results = memory
        .recall(entity_id, "", [], ["combat"], MemoryTagMode::Any)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|m| m.tags.contains("combat")));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_recall_marks_accessed() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Test memory",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Get initial access count
    let initial = memory.get_memory(memory_id.clone()).await.unwrap();
    assert_eq!(initial.access_count, 0);

    // Recall the memory
    memory
        .recall(entity_id, "test", [], [], MemoryTagMode::Any)
        .await
        .unwrap();

    // Check access count increased
    let after_recall = memory.get_memory(memory_id).await.unwrap();
    assert_eq!(after_recall.access_count, 1);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_reflect_basic() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create some memories
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Fought a dragon",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "dragon"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Opinion,
            "Dragons are dangerous",
            Utc::now(),
            None,
            [],
            [],
            ["dragon"],
        )
        .await
        .unwrap();

    // Reflect on dragons (without LLM manager for testing)
    let (response, used_memories) = memory
        .reflect(
            entity_id,
            "What do you know about dragons?",
            Some("The player is asking"),
            ["dragon"],
            MemoryTagMode::Any,
            None, // No LLM manager
            None, // No model
        )
        .await
        .unwrap();

    assert!(!response.is_empty());
    assert!(used_memories.len() >= 1);
    assert!(used_memories.iter().any(|m| m.content.contains("dragon")));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_prune_low_importance() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memories with different importance
    for i in 0..10 {
        memory
            .retain(
                entity_id,
                MemoryKind::Experience,
                &format!("Memory {}", i),
                Utc::now(),
                None,
                [],
                [],
                [],
            )
            .await
            .unwrap();
    }

    // Manually set some memories to low importance
    let memories = memory.list_memories(entity_id).await.unwrap();
    for (i, mem) in memories.iter().enumerate().take(5) {
        sqlx::query("UPDATE wyldlands.entity_memory SET importance = $1 WHERE memory_id = $2")
            .bind(0.05f32) // Very low importance
            .bind(mem.memory_id.uuid())
            .execute(&pool)
            .await
            .unwrap();
    }

    // Prune with min_keep = 3
    let pruned = memory
        .prune_low_importance_memories(entity_id, 3)
        .await
        .unwrap();

    // Should have pruned some but kept at least 3
    assert!(pruned > 0);
    let remaining = memory.count_memories(entity_id).await.unwrap();
    assert!(remaining >= 3);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_prune_respects_min_keep() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create only 5 memories
    for i in 0..5 {
        memory
            .retain(
                entity_id,
                MemoryKind::Experience,
                &format!("Memory {}", i),
                Utc::now(),
                None,
                [],
                [],
                [],
            )
            .await
            .unwrap();
    }

    // Try to prune with min_keep = 10 (more than we have)
    let pruned = memory
        .prune_low_importance_memories(entity_id, 10)
        .await
        .unwrap();

    // Should not prune anything
    assert_eq!(pruned, 0);
    assert_eq!(memory.count_memories(entity_id).await.unwrap(), 5);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_consolidate_similar_memories() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create similar memories
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Fought a goblin in the forest",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "goblin", "forest"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Defeated another goblin in the forest",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "goblin", "forest"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Encountered goblins while traveling",
            Utc::now(),
            None,
            [],
            [],
            ["combat", "goblin"],
        )
        .await
        .unwrap();

    let initial_count = memory.count_memories(entity_id).await.unwrap();

    // Consolidate
    let consolidated = memory
        .consolidate(
            entity_id,
            "goblin",
            None,
            ["goblin"],
            MemoryTagMode::Any,
            None, // llm_manager
            None, // similarity_threshold
        )
        .await
        .unwrap();

    // Should have consolidated some memories
    assert!(consolidated > 0);

    let final_count = memory.count_memories(entity_id).await.unwrap();
    assert!(final_count < initial_count);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_consolidate_preserves_tags() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memories with overlapping tags
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "First memory",
            Utc::now(),
            None,
            [],
            [],
            ["tag1", "tag2"],
        )
        .await
        .unwrap();

    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Second memory",
            Utc::now(),
            None,
            [],
            [],
            ["tag2", "tag3"],
        )
        .await
        .unwrap();

    // Consolidate
    memory
        .consolidate(
            entity_id,
            "",
            None,
            [],
            MemoryTagMode::Any,
            None, // llm_manager
            None, // similarity_threshold
        )
        .await
        .unwrap();

    // Check that consolidated memory has all tags
    let memories = memory.list_memories(entity_id).await.unwrap();
    if let Some(consolidated) = memories.first() {
        assert!(
            consolidated.tags.contains("tag1")
                || consolidated.tags.contains("tag2")
                || consolidated.tags.contains("tag3")
        );
    }

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_consolidate_no_effect_on_single_memory() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create only one memory
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Single memory",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Try to consolidate
    let consolidated = memory
        .consolidate(
            entity_id,
            "",
            None,
            [],
            MemoryTagMode::Any,
            None, // llm_manager
            None, // similarity_threshold
        )
        .await
        .unwrap();

    // Should not consolidate anything
    assert_eq!(consolidated, 0);
    assert_eq!(memory.count_memories(entity_id).await.unwrap(), 1);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_auto_consolidate_trigger() {
    let pool = setup_test_db().await;

    // Create config with low threshold for testing
    let config = MemoryConfig {
        max_memories_per_entity: 10,
        consolidation_threshold: 8,
        ..Default::default()
    };

    let mut memory = MemoryResource::with_config(pool.clone(), config);
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memories below threshold
    for i in 0..7 {
        memory
            .retain(
                entity_id,
                MemoryKind::Experience,
                &format!("Memory {}", i),
                Utc::now(),
                None,
                [],
                [],
                ["test"],
            )
            .await
            .unwrap();
    }

    // Should not trigger
    let triggered = memory
        .auto_consolidate_if_needed(entity_id, None)
        .await
        .unwrap();
    assert!(!triggered);

    // Add one more to reach threshold
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Memory 8",
            Utc::now(),
            None,
            [],
            [],
            ["test"],
        )
        .await
        .unwrap();

    // Should trigger now
    let triggered = memory
        .auto_consolidate_if_needed(entity_id, None)
        .await
        .unwrap();
    assert!(triggered);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_memory_importance_decay_over_time() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create a memory with known timestamp
    let old_time = Utc::now() - chrono::Duration::days(30);
    let memory_id = memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Old memory",
            old_time,
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Manually update timestamp to be old
    sqlx::query("UPDATE wyldlands.entity_memory SET timestamp = $1 WHERE memory_id = $2")
        .bind(old_time)
        .bind(memory_id.uuid())
        .execute(&pool)
        .await
        .unwrap();

    let node = memory.get_memory(memory_id).await.unwrap();
    let current_importance = node.calculate_current_importance(Utc::now());

    // Should be less than initial importance due to decay
    assert!(current_importance < node.importance);

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_tag_mode_any_strict() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memory with tags
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Tagged memory",
            Utc::now(),
            None,
            [],
            [],
            ["tag1", "tag2"],
        )
        .await
        .unwrap();

    // Create memory without tags
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Untagged memory",
            Utc::now(),
            None,
            [],
            [],
            [],
        )
        .await
        .unwrap();

    // Recall with AnyStrict (should exclude untagged)
    let results = memory
        .recall(entity_id, "", [], ["tag1"], MemoryTagMode::AnyStrict)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].tags.contains("tag1"));

    cleanup_entity_memories(&pool, entity_id).await;
}

#[tokio::test]
async fn test_tag_mode_all_strict() {
    let pool = setup_test_db().await;
    let mut memory = MemoryResource::new(pool.clone());
    let entity_id = create_test_entity();

    cleanup_entity_memories(&pool, entity_id).await;

    // Create memory with both tags
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "Both tags",
            Utc::now(),
            None,
            [],
            [],
            ["tag1", "tag2"],
        )
        .await
        .unwrap();

    // Create memory with only one tag
    memory
        .retain(
            entity_id,
            MemoryKind::Experience,
            "One tag",
            Utc::now(),
            None,
            [],
            [],
            ["tag1"],
        )
        .await
        .unwrap();

    // Recall with AllStrict (must have both tags)
    let results = memory
        .recall(
            entity_id,
            "",
            [],
            ["tag1", "tag2"],
            MemoryTagMode::AllStrict,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].tags.contains("tag1") && results[0].tags.contains("tag2"));

    cleanup_entity_memories(&pool, entity_id).await;
}



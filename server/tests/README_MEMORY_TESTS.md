# Memory System Integration Tests

This document describes the comprehensive test suite for the AI Memory System.

## Overview

The `memory_integration_tests.rs` file contains 20+ integration tests that verify all aspects of the memory persistence layer, including CRUD operations, validation, concurrency, and data integrity.

## Test Categories

### 1. Resource Creation Tests
- `test_memory_resource_creation` - Verifies default and custom configuration

### 2. Memory Creation (Retain) Tests
- `test_retain_memory_basic` - Basic memory creation with metadata and tags
- `test_retain_memory_with_entities` - Memory creation with entity relationships
- `test_retain_memory_validation` - Content validation (empty, whitespace, too long)

### 3. Memory Retrieval Tests
- `test_list_memories` - List all memories for an entity (sorted by timestamp)
- `test_count_memories` - Count memories for an entity
- `test_get_memory_not_found` - Error handling for non-existent memories

### 4. Memory Update (Alter) Tests
- `test_alter_memory_content` - Update memory content
- `test_alter_memory_context` - Update memory context
- `test_alter_memory_tags` - Update memory tags
- `test_alter_memory_validation` - Validation during updates

### 5. Memory Deletion Tests
- `test_delete_memory` - Basic deletion
- `test_delete_memory_not_found` - Error handling for non-existent memories
- `test_delete_memory_cascades_entities` - Verify cascade deletion of entity relationships

### 6. Memory Types Tests
- `test_memory_kinds` - All four memory kinds (World, Experience, Opinion, Observation)

### 7. Memory Importance Tests
- `test_memory_importance_calculation` - Importance decay and access boost algorithms

### 8. Data Isolation Tests
- `test_memory_isolation_between_entities` - Verify entities can't access each other's memories

### 9. Concurrency Tests
- `test_concurrent_memory_operations` - Parallel memory creation

## Running the Tests

### Prerequisites

1. **PostgreSQL Database**: Tests require a PostgreSQL database with the wyldlands schema
2. **pgvector Extension**: Must be installed for vector operations
3. **Environment Variable**: Set `DATABASE_URL` or use default test database

```bash
# Set database URL (optional)
export DATABASE_URL="postgresql://wyldlands:wyldlands@localhost/wyldlands_test"

# Or use .env file
echo "DATABASE_URL=postgresql://wyldlands:wyldlands@localhost/wyldlands_test" > server/.env.test
```

### Run All Memory Tests

```bash
cd server
cargo test --test memory_integration_tests
```

### Run Specific Test

```bash
cargo test --test memory_integration_tests test_retain_memory_basic
```

### Run with Output

```bash
cargo test --test memory_integration_tests -- --nocapture
```

### Run in Parallel (default)

```bash
cargo test --test memory_integration_tests -- --test-threads=4
```

### Run Sequentially

```bash
cargo test --test memory_integration_tests -- --test-threads=1
```

## Test Database Setup

### Create Test Database

```sql
-- Connect to PostgreSQL
psql -U postgres

-- Create test database
CREATE DATABASE wyldlands_test;
CREATE USER wyldlands WITH PASSWORD 'wyldlands';
GRANT ALL PRIVILEGES ON DATABASE wyldlands_test TO wyldlands;

-- Connect to test database
\c wyldlands_test

-- Install pgvector extension
CREATE EXTENSION vector;

-- Grant schema permissions
GRANT ALL ON SCHEMA public TO wyldlands;
```

### Run Migrations

```bash
# From project root
sqlx migrate run --database-url postgresql://wyldlands:wyldlands@localhost/wyldlands_test
```

## Test Data Cleanup

Each test:
1. Creates a unique test entity with a random UUID
2. Cleans up any existing test data before running
3. Cleans up after itself (optional, can be disabled for debugging)

To inspect test data after a failed test, comment out the cleanup calls:

```rust
// cleanup_entity_memories(&pool, entity_id).await;
```

## Coverage

The test suite covers:

✅ **CRUD Operations**: Create, Read, Update, Delete
✅ **Validation**: Content length, empty strings, data types
✅ **Error Handling**: Not found, invalid data, database errors
✅ **Data Integrity**: Foreign keys, cascading deletes, isolation
✅ **Concurrency**: Parallel operations, race conditions
✅ **Business Logic**: Importance calculation, access tracking
✅ **Relationships**: Entity associations, metadata, tags

## Test Metrics

- **Total Tests**: 20
- **Lines of Code**: ~717
- **Average Test Duration**: ~50-100ms per test
- **Total Suite Duration**: ~2-3 seconds

## Continuous Integration

These tests are designed to run in CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Run Memory Tests
  run: |
    docker-compose up -d postgres
    cargo test --test memory_integration_tests
  env:
    DATABASE_URL: postgresql://wyldlands:wyldlands@localhost/wyldlands_test
```

## Debugging Failed Tests

### Enable SQL Logging

```bash
RUST_LOG=sqlx=debug cargo test --test memory_integration_tests -- --nocapture
```

### Check Database State

```sql
-- Connect to test database
psql -U wyldlands wyldlands_test

-- Check memories
SELECT memory_id, entity_id, kind, content, timestamp 
FROM wyldlands.entity_memory 
ORDER BY timestamp DESC 
LIMIT 10;

-- Check entity relationships
SELECT * FROM wyldlands.entity_memory_entities;

-- Count memories by entity
SELECT entity_id, COUNT(*) 
FROM wyldlands.entity_memory 
GROUP BY entity_id;
```

### Common Issues

1. **Database Connection Failed**
   - Verify PostgreSQL is running
   - Check DATABASE_URL is correct
   - Ensure database exists

2. **Migration Not Run**
   - Run `sqlx migrate run` with test database URL
   - Verify schema exists: `\dt wyldlands.*`

3. **pgvector Extension Missing**
   - Install: `CREATE EXTENSION vector;`
   - Verify: `SELECT * FROM pg_extension WHERE extname = 'vector';`

4. **Permission Denied**
   - Grant permissions: `GRANT ALL ON SCHEMA wyldlands TO wyldlands;`
   - Grant table access: `GRANT ALL ON ALL TABLES IN SCHEMA wyldlands TO wyldlands;`

## Future Test Additions

Planned tests for future features:

- [ ] `test_recall_semantic_search` - Vector similarity search
- [ ] `test_reflect_with_llm` - LLM-based reflection
- [ ] `test_consolidate_memories` - Memory consolidation
- [ ] `test_prune_low_importance` - Automatic pruning
- [ ] `test_memory_limit_enforcement` - Max memories per entity
- [ ] `test_auto_consolidation_trigger` - Automatic consolidation
- [ ] `test_embedding_generation` - Vector embedding creation
- [ ] `test_tag_filtering` - Tag-based memory filtering
- [ ] `test_metadata_queries` - JSONB metadata queries
- [ ] `test_related_memories` - Memory relationship graphs

## Contributing

When adding new memory features:

1. Add corresponding integration tests
2. Follow existing test patterns
3. Include cleanup in test teardown
4. Document test purpose in comments
5. Update this README with new test descriptions
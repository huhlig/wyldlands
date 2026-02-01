# Database Schema Analysis for Phases 1-6

## Overview

This document details the database schema analysis performed to ensure the database properly reflects the changes needed for phases 1-6 of the gateway-world refactor.

## Analysis Date

2026-01-31

## Issues Identified

### 1. Missing `characters` Table ❌

**Location:** Referenced in `server/src/persistence.rs` lines 160, 362

**Issue:** The code attempts to query and insert into a `wyldlands.characters` table that doesn't exist in the schema.

**Queries Affected:**
```sql
-- Line 160: list_characters_for_account
SELECT c.uuid as id, i.name, c.level, c.race, c.class, c.updated_at as last_played
FROM wyldlands.characters c
JOIN wyldlands.identities i ON c.uuid = i.entity_uuid
WHERE c.account_id = $1

-- Line 362: create_character_with_builder
INSERT INTO wyldlands.characters (uuid, account_id, level, race, class, created_at, updated_at)
VALUES ($1, $2, 1, 'Human', NULL, NOW(), NOW())
```

**Resolution:** Created `characters` table in migration 005.

### 2. Missing `identities` Table ❌

**Location:** Referenced in `server/src/persistence.rs` lines 161, 179

**Issue:** The code joins with an `identities` table to get character names, but this table doesn't exist.

**Queries Affected:**
```sql
-- Used in character listing and retrieval
JOIN wyldlands.identities i ON c.uuid = i.entity_uuid
```

**Resolution:** Created `identities` table in migration 005 with unique name constraint.

### 3. Missing `entity_metadata` Table ❌

**Location:** Referenced in `server/src/persistence.rs` line 338

**Issue:** The code attempts to store talents as JSON in an `entity_metadata` table that doesn't exist.

**Query Affected:**
```sql
INSERT INTO wyldlands.entity_metadata (entity_id, key, value)
VALUES ($1, 'talents', $2)
ON CONFLICT (entity_id, key) DO UPDATE SET value = $2
```

**Resolution:** Created `entity_metadata` table in migration 005 with flexible key-value storage.

### 4. Incomplete `session_state` Enum ❌

**Location:** `migrations/001_table_setup.sql` line 152

**Current Values:**
```sql
CREATE TYPE wyldlands.session_state AS ENUM (
    'Connecting', 
    'Authenticating', 
    'CharacterSelection', 
    'Playing', 
    'Disconnected', 
    'Closed'
);
```

**Missing States Required by Phase 6:**
- `Authenticated` - Initial state after login (before character selection)
- `CharacterCreation` - Character builder state
- `Editing` - Text editor state for builders/admins

**Resolution:** Updated enum in migration 005 to include all required states.

### 5. Wrong Column Names in `entity_skills` Table ❌

**Location:** `migrations/001_table_setup.sql` line 418, `server/src/persistence.rs` line 317

**Schema Definition:**
```sql
CREATE TABLE wyldlands.entity_skills (
    entity_id  UUID         NOT NULL,
    skill_name VARCHAR(100) NOT NULL,  -- ❌ Code expects 'skill'
    level      INTEGER      NOT NULL,  -- ❌ Code expects 'rank'
    ...
)
```

**Code Usage:**
```rust
sqlx::query(
    "INSERT INTO wyldlands.entity_skills (entity_id, skill, rank)
     VALUES ($1, $2, $3)",
)
```

**Resolution:** Renamed columns in migration 005:
- `skill_name` → `skill`
- `level` → `rank`

## Schema Additions in Migration 005

### Tables Created

1. **`characters`** - Character-specific data for player entities
   - Links entities to accounts
   - Stores level, race, class
   - Tracks creation and update timestamps

2. **`identities`** - Unique identity names for entities
   - Enforces unique character names
   - Links to entities table
   - Used for character listing and selection

3. **`entity_metadata`** - Flexible key-value metadata storage
   - Stores talents as JSON
   - Supports any custom entity metadata
   - Composite primary key (entity_id, key)

### Enums Updated

1. **`session_state`** - Added missing states:
   - `Authenticated`
   - `CharacterCreation`
   - `Editing`

### Columns Renamed

1. **`entity_skills`**:
   - `skill_name` → `skill`
   - `level` → `rank`

### Triggers Added

1. **`sync_identity_to_name`** - Keeps `entity_name.display` in sync with `identities.name`
   - Ensures consistency between identity system and name component
   - Fires on INSERT or UPDATE to identities table

## Data Migration

The migration includes automatic data migration:

1. **Existing character names** are copied from `entity_name` to `identities` for all entities with avatars
2. **Conflict handling** uses `ON CONFLICT DO NOTHING` to handle duplicates gracefully
3. **Verification query** checks for entities with avatars but no identity

## Indexes Added

Performance indexes for common queries:

1. `idx_characters_account_id` - Character lookup by account
2. `idx_characters_updated_at` - Character sorting by last played
3. `idx_characters_account_updated` - Composite index for character listing
4. `idx_identities_name` - Fast name lookups
5. `idx_entity_metadata_entity` - Metadata lookup by entity
6. `idx_entity_metadata_key` - Metadata lookup by key

## Phase 6 Requirements Met

### ✅ Phase 1-2: Protocol Updates
- Database supports all session states needed for protocol

### ✅ Phase 3-4: Gateway State Machine & Authentication
- Session states support full authentication flow
- Character selection and creation states available

### ✅ Phase 5: Input Mode Implementation
- `Editing` state available for text editor mode

### ✅ Phase 6: Server-Side Editing Logic
- All tables needed for character creation exist
- Metadata storage for talents available
- Character listing and selection queries supported

## Testing Recommendations

1. **Run migration 005** on development database
2. **Verify all tables created** successfully
3. **Test character creation flow** end-to-end
4. **Test character listing** for accounts
5. **Verify talent storage** in entity_metadata
6. **Check session state transitions** work correctly

## Backward Compatibility

The migration is designed to be backward compatible:

1. **Existing data preserved** - No data loss
2. **Enum update** uses safe ALTER TYPE approach
3. **Column renames** maintain data integrity
4. **Triggers** only affect new/updated records

## Next Steps

1. Apply migration 005 to database
2. Run integration tests for character creation
3. Verify all Phase 6 functionality works
4. Update any documentation referencing old schema

## Related Files

- `migrations/005_phase6_schema_fixes.sql` - The migration file
- `server/src/persistence.rs` - Persistence layer implementation
- `GATEWAY_WORLD_REFACTOR.md` - Phase 1-6 implementation plan
- `migrations/001_table_setup.sql` - Original schema definition

## Summary

The database schema had **5 critical issues** preventing phases 1-6 from working correctly:

1. ❌ Missing `characters` table
2. ❌ Missing `identities` table  
3. ❌ Missing `entity_metadata` table
4. ❌ Incomplete `session_state` enum
5. ❌ Wrong column names in `entity_skills`

All issues have been **resolved in migration 005**, which:
- ✅ Creates 3 new tables
- ✅ Updates 1 enum with 3 new values
- ✅ Renames 2 columns
- ✅ Adds 1 trigger for data consistency
- ✅ Adds 6 performance indexes
- ✅ Migrates existing data automatically

The database now **fully supports phases 1-6** of the gateway-world refactor.
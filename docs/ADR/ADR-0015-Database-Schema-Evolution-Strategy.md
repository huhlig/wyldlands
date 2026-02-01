---
parent: ADR
nav_order: 0015
title: Database Schema Evolution Strategy
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0015: Database Schema Evolution Strategy

## Context and Problem Statement

Database schemas evolve as the application grows. We need a strategy to:
- Track schema changes over time
- Apply migrations reliably
- Support rollback when needed
- Maintain data integrity during migrations
- Enable zero-downtime deployments
- Document schema evolution

How should we manage database schema evolution to ensure reliability and maintainability?

## Decision Drivers

* **Reliability**: Migrations must be safe and reversible
* **Traceability**: Track all schema changes
* **Automation**: Automated migration application
* **Data Integrity**: Preserve data during migrations
* **Team Coordination**: Multiple developers working on schema
* **Production Safety**: Safe deployment to production
* **Documentation**: Clear migration history

## Considered Options

* SQL Migration Files with Manual Tracking
* ORM-Based Migrations (Diesel, SeaORM)
* Custom Migration Framework
* Database Versioning Tools (Flyway, Liquibase)

## Decision Outcome

Chosen option: "SQL Migration Files with Manual Tracking", because it provides simplicity, full SQL control, and PostgreSQL-specific features while avoiding ORM complexity.

### Migration Strategy

**Structure:**
```
migrations/
├── 001_table_setup.sql           # Initial schema
├── 002_settings_data.sql         # Configuration data
├── 003_world_data.sql            # World content
├── 004_help_data.sql             # Help system
├── 005_phase6_schema_fixes.sql   # Schema corrections
└── README.md                     # Migration guide
```

**Naming Convention:**
- `NNN_description.sql` where NNN is sequential number
- Descriptive names indicating purpose
- One migration per logical change

### Migration Phases

**Phase 1-4**: Initial schema and data
**Phase 5**: Character creation system
**Phase 6**: Session state and schema fixes

### Positive Consequences

* **Full SQL Control**: Use all PostgreSQL features
* **Simple**: No ORM complexity
* **Transparent**: Migrations are readable SQL
* **Flexible**: Can handle complex schema changes
* **Documented**: Each migration is self-documenting
* **Testable**: Can test migrations in isolation

### Negative Consequences

* **Manual Tracking**: Must manually track applied migrations
* **No Automatic Rollback**: Must write rollback scripts manually
* **Coordination**: Developers must coordinate migration numbers

## Implementation Details

### Migration File Format

```sql
-- Migration: 005_phase6_schema_fixes.sql
-- Description: Fix schema issues identified in phase 6
-- Date: 2026-01-31

-- Add missing tables
CREATE TABLE IF NOT EXISTS wyldlands.characters (
    uuid UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES wyldlands.accounts(uuid),
    level INTEGER NOT NULL DEFAULT 1,
    race VARCHAR(50),
    class VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add indexes
CREATE INDEX idx_characters_account ON wyldlands.characters(account_id);

-- Update enums
ALTER TYPE wyldlands.session_state ADD VALUE IF NOT EXISTS 'CharacterCreation';
ALTER TYPE wyldlands.session_state ADD VALUE IF NOT EXISTS 'Editing';
```

### Application Process

**Docker Compose:**
```yaml
services:
  postgres:
    image: postgres:15
    volumes:
      - ./migrations:/docker-entrypoint-initdb.d
```

**Manual Application:**
```bash
psql -U wyldlands -d wyldlands -f migrations/001_table_setup.sql
psql -U wyldlands -d wyldlands -f migrations/002_settings_data.sql
# ... etc
```

### Schema Versioning

**Track applied migrations:**
```sql
CREATE TABLE IF NOT EXISTS wyldlands.schema_migrations (
    version INTEGER PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Key Schema Components

**Core Tables:**
- `sessions`: Session state and metadata
- `accounts`: User accounts
- `characters`: Player characters
- `identities`: Entity names
- `entities`: All game entities
- `entity_attributes`: Character attributes
- `entity_skills`: Character skills
- `entity_metadata`: Flexible key-value storage

**Enums:**
- `session_state`: Session lifecycle states
- `help_category`: Help topic categories

## Validation

Schema evolution is validated by:

1. **Migration Testing**: Test each migration on clean database
2. **Data Integrity**: Verify foreign keys and constraints
3. **Rollback Testing**: Test rollback procedures
4. **Production Simulation**: Test on production-like data
5. **Documentation Review**: Ensure migrations are documented

## More Information

### Best Practices

1. **Idempotent Migrations**: Use `IF NOT EXISTS` and `IF EXISTS`
2. **Data Preservation**: Never drop columns with data without backup
3. **Incremental Changes**: Small, focused migrations
4. **Testing**: Test migrations on copy of production data
5. **Rollback Plan**: Document rollback procedure for each migration

### Migration Checklist

- [ ] Migration number is sequential
- [ ] Migration is idempotent
- [ ] Foreign keys are valid
- [ ] Indexes are created for common queries
- [ ] Data migration preserves existing data
- [ ] Rollback procedure is documented
- [ ] Migration is tested on clean database

### Future Enhancements

1. **Automated Tracking**: Track applied migrations in database
2. **Rollback Scripts**: Separate rollback SQL files
3. **Migration Tool**: Custom tool for applying migrations
4. **Schema Validation**: Automated schema validation
5. **Zero-Downtime Migrations**: Online schema changes

### Related Decisions

- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - PostgreSQL enables rich schema features
- [ADR-0011](ADR-0011-Character-Creation-System-Architecture.md) - Character tables added in phase 5
- [ADR-0012](ADR-0012-Session-State-Management-Strategy.md) - Session state enum evolution

### References

- Migrations Directory: [migrations/](../../migrations/)
- Schema Analysis: [docs/development/DATABASE_SCHEMA_PHASE6_ANALYSIS.md](../development/DATABASE_SCHEMA_PHASE6_ANALYSIS.md)
- Initial Schema: [migrations/001_table_setup.sql](../../migrations/001_table_setup.sql)
- Phase 6 Fixes: [migrations/005_phase6_schema_fixes.sql](../../migrations/005_phase6_schema_fixes.sql)
# Help System

Database-driven help system providing detailed information about commands, skills, and game features.

## Quick Start

```
help                # Basic help overview
help commands       # List all commands
help <keyword>      # Detailed help for topic
```

## Commands

### help
Shows basic help information and available help commands.

### help commands
Lists all available commands organized by category.

### help <keyword>
Shows detailed help for a specific topic. Supports aliases (e.g., `help i` → `help inventory`).

**Example:**
```
help look
help inventory
help i              # Alias for inventory
```

## Features

- **Database-driven** - Easy to update without code changes
- **Searchable** - Organized by category
- **Aliased** - Convenient shortcuts
- **Permission-based** - Admin-only topics
- **Related topics** - Cross-references

## Database Schema

### help_topics
Main help content storage.

**Columns:**
- `keyword` - Unique identifier (PRIMARY KEY)
- `category` - Category classification
- `title` - Display title
- `content` - Main help text
- `syntax` - Command syntax/usage (optional)
- `examples` - Usage examples (optional)
- `see_also` - Related topics array
- `min_level` - Minimum level to view (default: 0)
- `admin_only` - Admin-only flag
- `created_at`, `updated_at` - Timestamps
- `created_by`, `updated_by` - User tracking

### help_aliases
Alternative keywords mapping to primary topics.

**Columns:**
- `alias` - Alternative keyword (PRIMARY KEY)
- `keyword` - Points to help_topics.keyword
- `created_at` - Timestamp

### Categories
- `Command` - Game commands
- `Skill` - Character skills
- `Talent` - Character talents
- `Spell` - Magic spells
- `Combat` - Combat mechanics
- `Building` - World building
- `Social` - Social interactions
- `System` - System information
- `Lore` - Game lore
- `General` - General information

## Pre-Populated Topics

### System
- `help` - Help system overview
- `commands` - Command list
- `world` - World management (admin)

### Commands
- `look` - Examine surroundings
- `inventory` - Check inventory
- `say` - Speak to others
- `yell` - Shout messages
- `emote` - Perform actions
- `score` - View character stats
- `exit` - Leave game
- `movement` - Movement commands

### Building (Admin)
- `building` - Building system overview
- `area` - Area management
- `room` - Room management
- `dig` - Quick room creation

### Common Aliases
- `l` → `look`
- `i`, `inv` → `inventory`
- `'` → `say`
- `"` → `yell`
- `:`, `em` → `emote`
- `stats` → `score`
- `quit`, `logout`, `logoff` → `exit`
- Direction shortcuts (`n`, `s`, `e`, `w`, etc.) → `movement`

## Adding Help Topics

### Via SQL
```sql
-- Add topic
INSERT INTO wyldlands.help_topics 
(keyword, category, title, content, syntax, examples, see_also, admin_only)
VALUES (
    'newcommand',
    'Command',
    'New Command',
    'This command does something useful.',
    'newcommand <arg1> [arg2]',
    'newcommand test
newcommand test optional',
    ARRAY['relatedcommand', 'anothercommand'],
    FALSE
);

-- Add alias
INSERT INTO wyldlands.help_aliases (alias, keyword)
VALUES ('nc', 'newcommand');
```

## Implementation

### Code Structure
- `server/src/ecs/systems/command/help.rs` - Help system implementation
  - `get_help_topic()` - Fetches help with alias resolution
  - `format_help_topic()` - Formats for display
  - `help_command()` - Basic help handler
  - `help_commands_command()` - Command list handler
  - `help_keyword_command()` - Specific topic handler

- `server/src/ecs/systems/command.rs` - Command system integration
  - Special handling for help commands in `execute()` method

- `migrations/004_help_data.sql` - Database schema and initial data

### Database Access
```rust
context.persistence_manager().database()
```

## Migration

Apply the help system to your database:

```bash
psql -U wyldlands -d wyldlands -f migrations/004_help_data.sql
```

## Testing

1. Start the server
2. Connect as a player
3. Try commands:
   ```
   help
   help commands
   help look
   help i
   ```
4. Connect as admin for admin-only help:
   ```
   help building
   help world
   ```

## Troubleshooting

### Help topic not found
- Check if topic exists in `wyldlands.help_topics`
- Check for alias in `wyldlands.help_aliases`
- Verify keyword is lowercase

### Admin help not visible
- Ensure user has admin privileges
- Check `admin_only` flag on topic

### Database connection errors
- Verify database is running
- Check connection pool configuration
- Review server logs

## See Also
- [Builder Commands](BUILDER_COMMANDS.md) - World building help
- [Combat System](COMBAT_SYSTEM.md) - Combat help
- [NPC System](NPC_SYSTEM.md) - NPC help
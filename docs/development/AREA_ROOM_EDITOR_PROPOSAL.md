# Area/Room Editor System Proposal

## Overview

This document proposes a comprehensive area and room editor system for Wyldlands that allows builders to create and modify the game world through both telnet and websocket interfaces.

## Design Goals

1. **Safe Operations**: Prevent accidental deletion of areas/rooms with entities
2. **Intuitive Interface**: Easy-to-use commands accessible from both telnet and websocket
3. **Comprehensive Editing**: Full control over area/room properties, exits, and relationships
4. **Efficient Workflow**: Support "digging" new rooms from existing ones
5. **Persistence**: All changes automatically saved to database
6. **Validation**: Prevent invalid states (orphaned rooms, circular exits, etc.)

## Command Structure

### Area Commands

#### `area create <name>`
Create a new area with the specified name.

**Example:**
```
area create "The Dark Forest"
```

**Output:**
```
Area created successfully!
UUID: 12345678-1234-1234-1234-123456789012
Name: The Dark Forest
Kind: Overworld (default)
Flags: (none)

Use 'area edit <uuid>' to modify properties.
```

#### `area list [filter]`
List all areas, optionally filtered by name pattern.

**Example:**
```
area list forest
```

**Output:**
```
Areas (2 matching):
================================================================================
UUID: 12345678-1234-1234-1234-123456789012
  Name: The Dark Forest
  Kind: Overworld
  Flags: (none)
  Rooms: 15

UUID: 87654321-4321-4321-4321-210987654321
  Name: Forest Temple
  Kind: Building
  Flags: (none)
  Rooms: 8
================================================================================
```

#### `area edit <uuid> <property> <value>`
Edit area properties.

**Properties:**
- `name <new name>` - Change area name
- `description <new description>` - Change area description
- `kind <Overworld|Vehicle|Building|Dungeon>` - Change area type
- `flag add <flag>` - Add an area flag
- `flag remove <flag>` - Remove an area flag

**Examples:**
```
area edit 12345678-1234-1234-1234-123456789012 name "The Haunted Forest"
area edit 12345678-1234-1234-1234-123456789012 kind Dungeon
area edit 12345678-1234-1234-1234-123456789012 flag add Underwater
```

#### `area delete <uuid>`
Delete an area (only if it has no rooms).

**Example:**
```
area delete 12345678-1234-1234-1234-123456789012
```

**Output (success):**
```
Area deleted successfully.
UUID: 12345678-1234-1234-1234-123456789012
Name: The Dark Forest
```

**Output (failure):**
```
Error: Cannot delete area - it contains 15 rooms.
Use 'room list <area-uuid>' to see rooms in this area.
Delete or move all rooms before deleting the area.
```

#### `area info <uuid>`
Display detailed information about an area.

**Example:**
```
area info 12345678-1234-1234-1234-123456789012
```

**Output:**
```
Area Information
================================================================================
UUID: 12345678-1234-1234-1234-123456789012
Name: The Dark Forest
Description: A mysterious forest shrouded in darkness and ancient magic.

Kind: Dungeon
Flags: Underwater

Statistics:
  Total Rooms: 15
  Connected Rooms: 15
  Isolated Rooms: 0
  Total Exits: 42

Recent Activity:
  Created: 2026-01-01 10:30:00 UTC
  Last Modified: 2026-01-01 18:15:00 UTC
  Modified By: admin (account-uuid)
================================================================================
```

### Room Commands

#### `room create <area-uuid> <name>`
Create a new room in the specified area.

**Example:**
```
room create 12345678-1234-1234-1234-123456789012 "Forest Entrance"
```

**Output:**
```
Room created successfully!
UUID: abcdef12-3456-7890-abcd-ef1234567890
Name: Forest Entrance
Area: The Dark Forest (12345678-1234-1234-1234-123456789012)
Flags: Breathable (default)

You are now in the new room.
Use 'room edit <uuid>' to modify properties.
Use 'dig <direction> <name>' to create connected rooms.
```

#### `room list [area-uuid]`
List all rooms, optionally filtered by area.

**Example:**
```
room list 12345678-1234-1234-1234-123456789012
```

**Output:**
```
Rooms in Area: The Dark Forest (15 rooms)
================================================================================
UUID: abcdef12-3456-7890-abcd-ef1234567890
  Name: Forest Entrance
  Exits: North, East
  Flags: Breathable

UUID: fedcba09-8765-4321-fedc-ba0987654321
  Name: Dark Path
  Exits: South, West, Up
  Flags: Breathable

... (13 more rooms)
================================================================================
```

#### `room edit <uuid> <property> <value>`
Edit room properties.

**Properties:**
- `name <new name>` - Change room name
- `description short <text>` - Change short description
- `description long <text>` - Change long description
- `area <area-uuid>` - Move room to different area
- `flag add <flag>` - Add a room flag
- `flag remove <flag>` - Remove a room flag

**Examples:**
```
room edit abcdef12-3456-7890-abcd-ef1234567890 name "Misty Entrance"
room edit abcdef12-3456-7890-abcd-ef1234567890 description short "A foggy forest entrance"
room edit abcdef12-3456-7890-abcd-ef1234567890 description long "Dense fog rolls through this entrance to the dark forest. Ancient trees loom overhead, their branches creating a natural archway. The path ahead disappears into shadow."
room edit abcdef12-3456-7890-abcd-ef1234567890 area 87654321-4321-4321-4321-210987654321
room edit abcdef12-3456-7890-abcd-ef1234567890 flag remove Breathable
```

#### `room delete <uuid>`
Delete a room (only if no entities are present).

**Example:**
```
room delete abcdef12-3456-7890-abcd-ef1234567890
```

**Output (success):**
```
Room deleted successfully.
UUID: abcdef12-3456-7890-abcd-ef1234567890
Name: Forest Entrance
All exits to/from this room have been removed.
```

**Output (failure):**
```
Error: Cannot delete room - it contains 3 entities.
Entities present:
  - Player: TestUser (uuid)
  - NPC: Forest Guardian (uuid)
  - Item: Rusty Sword (uuid)

Move or remove all entities before deleting the room.
```

#### `room info [uuid]`
Display detailed information about a room (defaults to current room).

**Example:**
```
room info
```

**Output:**
```
Room Information
================================================================================
UUID: abcdef12-3456-7890-abcd-ef1234567890
Name: Forest Entrance
Area: The Dark Forest (12345678-1234-1234-1234-123456789012)

Short Description:
  A foggy forest entrance

Long Description:
  Dense fog rolls through this entrance to the dark forest. Ancient trees
  loom overhead, their branches creating a natural archway. The path ahead
  disappears into shadow.

Flags: Breathable

Exits (2):
  North -> Dark Path (fedcba09-8765-4321-fedc-ba0987654321)
  East  -> Forest Clearing (11111111-2222-3333-4444-555555555555)

Entities Present (3):
  - Player: TestUser
  - NPC: Forest Guardian
  - Item: Rusty Sword

Recent Activity:
  Created: 2026-01-01 10:35:00 UTC
  Last Modified: 2026-01-01 18:20:00 UTC
================================================================================
```

#### `room goto <uuid>`
Teleport to a specific room.

**Example:**
```
room goto abcdef12-3456-7890-abcd-ef1234567890
```

**Output:**
```
You teleport to the room...

Forest Entrance
A foggy forest entrance

Dense fog rolls through this entrance to the dark forest. Ancient trees
loom overhead, their branches creating a natural archway. The path ahead
disappears into shadow.

Exits: North, East
```

### Exit Commands

#### `exit add <direction> <dest-room-uuid>`
Add an exit from current room to destination room.

**Example:**
```
exit add north fedcba09-8765-4321-fedc-ba0987654321
```

**Output:**
```
Exit added successfully!
Direction: North
From: Forest Entrance (current room)
To: Dark Path (fedcba09-8765-4321-fedc-ba0987654321)

Use 'exit edit north <property> <value>' to add doors, locks, etc.
```

#### `exit remove <direction>`
Remove an exit from current room.

**Example:**
```
exit remove north
```

**Output:**
```
Exit removed successfully!
Direction: North
From: Forest Entrance (current room)
To: Dark Path (fedcba09-8765-4321-fedc-ba0987654321)
```

#### `exit edit <direction> <property> <value>`
Edit exit properties.

**Properties:**
- `door <rating>` - Add a door with strength rating
- `lock <rating> <code>` - Add a lock with rating and unlock code
- `close` - Close the door
- `open` - Open the door
- `lock` - Lock the door
- `unlock` - Unlock the door
- `transparent <true|false>` - Set transparency
- `remove door` - Remove door (and lock if present)
- `remove lock` - Remove lock only

**Examples:**
```
exit edit north door 50
exit edit north lock 75 "forest_key_001"
exit edit north close
exit edit north lock
exit edit north transparent true
exit edit north remove lock
```

#### `exit info <direction>`
Display detailed information about an exit.

**Example:**
```
exit info north
```

**Output:**
```
Exit Information
================================================================================
Direction: North
From: Forest Entrance (abcdef12-3456-7890-abcd-ef1234567890)
To: Dark Path (fedcba09-8765-4321-fedc-ba0987654321)

Door: Yes (Rating: 50)
  Status: Closed
  Transparent: No

Lock: Yes (Rating: 75)
  Status: Locked
  Unlock Code: forest_key_001

Use 'exit edit north <property> <value>' to modify.
================================================================================
```

#### `exit list`
List all exits from current room.

**Example:**
```
exit list
```

**Output:**
```
Exits from: Forest Entrance
================================================================================
North -> Dark Path
  Door: Closed (Rating: 50)
  Lock: Locked (Rating: 75, Code: forest_key_001)

East -> Forest Clearing
  (no door)

Up -> Tree Platform
  Door: Open (Rating: 30)
  (no lock)
================================================================================
```

### Digging Commands

#### `dig <direction> <room-name>`
Create a new room and connect it to the current room in the specified direction.
Automatically creates a reverse exit.

**Example:**
```
dig north "Dark Path"
```

**Output:**
```
Digging north...

New room created!
UUID: fedcba09-8765-4321-fedc-ba0987654321
Name: Dark Path
Area: The Dark Forest (same as current room)

Exits created:
  From Forest Entrance -> North -> Dark Path
  From Dark Path -> South -> Forest Entrance

You are now in the new room: Dark Path

Use 'room edit <uuid>' to modify the room.
Use 'exit edit <direction>' to modify exits.
```

#### `dig <direction> <room-name> oneway`
Create a new room with only a one-way exit (no reverse exit).

**Example:**
```
dig down "Hidden Cave" oneway
```

**Output:**
```
Digging down (one-way)...

New room created!
UUID: 99999999-8888-7777-6666-555555555555
Name: Hidden Cave
Area: The Dark Forest (same as current room)

Exit created:
  From Forest Entrance -> Down -> Hidden Cave
  (No reverse exit created)

You are now in the new room: Hidden Cave

Note: This room has no exit back. Use 'exit add' to create return path.
```

#### `dig <direction> <room-name> area <area-uuid>`
Create a new room in a different area.

**Example:**
```
dig east "Temple Entrance" area 87654321-4321-4321-4321-210987654321
```

**Output:**
```
Digging east into area: Forest Temple...

New room created!
UUID: 22222222-3333-4444-5555-666666666666
Name: Temple Entrance
Area: Forest Temple (87654321-4321-4321-4321-210987654321)

Exits created:
  From Forest Entrance -> East -> Temple Entrance
  From Temple Entrance -> West -> Forest Entrance

You are now in the new room: Temple Entrance

Note: This room is in a different area than the previous room.
```

## Implementation Plan

### Phase 1: Core Infrastructure

1. **Create new command module**: `server/src/ecs/systems/command/builder.rs`
   - Area management functions
   - Room management functions
   - Exit management functions
   - Validation utilities

2. **Extend persistence layer**: `server/src/persistence.rs`
   - Add area CRUD operations
   - Add room CRUD operations
   - Add exit CRUD operations
   - Add validation for safe deletion

3. **Database helpers**:
   - Add queries for entity counting in rooms/areas
   - Add queries for exit validation
   - Add queries for orphaned room detection

### Phase 2: Command Registration

1. **Register area commands** in `server/src/ecs/systems/command.rs`:
   - `area create`
   - `area list`
   - `area edit`
   - `area delete`
   - `area info`

2. **Register room commands**:
   - `room create`
   - `room list`
   - `room edit`
   - `room delete`
   - `room info`
   - `room goto`

3. **Register exit commands**:
   - `exit add`
   - `exit remove`
   - `exit edit`
   - `exit info`
   - `exit list`

4. **Register digging commands**:
   - `dig`

### Phase 3: Safety & Validation

1. **Implement safety checks**:
   - Prevent deletion of areas with rooms
   - Prevent deletion of rooms with entities
   - Validate area/room UUIDs exist
   - Validate exit destinations exist
   - Prevent circular references

2. **Add permission system**:
   - Check admin status for builder commands
   - Add builder permission level (between player and admin)
   - Log all builder actions for audit trail

3. **Add undo/rollback**:
   - Track recent builder actions
   - Allow undo of last N operations
   - Implement transaction-like rollback for complex operations

### Phase 4: Enhanced Features

1. **Batch operations**:
   - `area clone <uuid>` - Clone entire area with all rooms
   - `room clone <uuid>` - Clone a single room
   - `exit mirror <direction>` - Create matching reverse exit

2. **Templates**:
   - Save room templates
   - Apply templates to new rooms
   - Template library for common room types

3. **Visualization**:
   - `area map <uuid>` - ASCII map of area layout
   - `room connections` - Show connection graph
   - Export area to DOT format for graphviz

4. **Search & Filter**:
   - Search rooms by name/description
   - Find orphaned rooms
   - Find rooms with no exits
   - Find dead-end rooms

## Database Schema Additions

No new tables required! The existing schema already supports all operations:

- `wyldlands.entities` - Stores all entities (areas, rooms)
- `wyldlands.entity_name` - Names for areas/rooms
- `wyldlands.entity_description` - Descriptions
- `wyldlands.entity_areas` - Area-specific data
- `wyldlands.entity_rooms` - Room-specific data
- `wyldlands.entity_room_exits` - Exit data

Optional enhancement:
```sql
-- Track builder actions for audit/undo
CREATE TABLE wyldlands.builder_actions (
    id SERIAL PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES wyldlands.accounts(id),
    action_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    old_data JSONB,
    new_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Security Considerations

1. **Permission Checks**: All builder commands require admin or builder permission
2. **Validation**: All UUIDs validated before operations
3. **Audit Trail**: All builder actions logged with account ID
4. **Rate Limiting**: Prevent spam creation of areas/rooms
5. **Backup**: Automatic backups before bulk operations

## Testing Strategy

1. **Unit Tests**:
   - Test each command function independently
   - Test validation logic
   - Test safety checks

2. **Integration Tests**:
   - Test command execution through command system
   - Test persistence of changes
   - Test complex workflows (dig, edit, delete)

3. **Manual Testing**:
   - Test through telnet interface
   - Test through websocket interface
   - Test with multiple concurrent builders

## Documentation

1. **In-game help**: Each command has detailed help text
2. **Builder guide**: Comprehensive guide for world builders
3. **API documentation**: Document all builder functions
4. **Examples**: Provide example workflows for common tasks

## Future Enhancements

1. **Collaborative Building**: Multiple builders working on same area
2. **Version Control**: Track changes to areas/rooms over time
3. **Import/Export**: Export areas to JSON/YAML for sharing
4. **Visual Editor**: Web-based visual room editor
5. **Scripting**: Lua/Python scripts for procedural generation
6. **AI Assistance**: AI-powered description generation

## Conclusion

This area/room editor system provides a comprehensive, safe, and intuitive interface for building the Wyldlands world. It leverages the existing ECS and persistence infrastructure while adding powerful new capabilities for world creation and management.

The phased implementation approach allows for incremental development and testing, with each phase building on the previous one. The system is designed to be accessible from both telnet and websocket interfaces, making it available to all builders regardless of their preferred connection method.
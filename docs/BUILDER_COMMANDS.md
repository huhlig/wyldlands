# Builder Commands

World creation and editing commands for users with builder privileges.

## Quick Reference

```
# Areas
area create <name>              # Create area
area list                       # List all areas
area edit <uuid> <field> <val>  # Edit area

# Rooms
room create <name>              # Create room in current area
dig <direction> <name>          # Create room + bidirectional exits
room list [area_uuid]           # List rooms
room edit <uuid> <field> <val>  # Edit room

# Exits
exit create <dir> <target_uuid> # Create one-way exit
exit edit <dir> <prop> <val>    # Edit exit properties
exit delete <dir>               # Delete exit

# Items
item create <name> | <desc> | <weight> [| weapon/armor]
item spawn <template> [qty]     # Spawn from template
item edit <uuid> <field> <val>  # Edit item
```

## Area Management

### area create
```
area create <name>
acreate <name>
```
Creates a new area.

### area list
```
area list
alist / areas
```
Lists all areas with UUIDs and names.

### area info
```
area info <uuid>
ainfo <uuid>
```
Shows detailed area information.

### area edit
```
area edit <uuid> <field> <value>
aedit <uuid> <field> <value>
```

**Fields:** `name`, `description`, `flags`

**Flags:** `safe`, `no_combat`, `no_magic`, `no_recall`, `player_kill`

**Example:**
```
area edit <uuid> name The Enchanted Forest
area edit <uuid> flags safe,no_combat
```

### area delete
```
area delete <uuid>
adelete <uuid>
```
Deletes an area (must be empty).

### area search
```
area search <query>
asearch <query>
```
Searches areas by name (case-insensitive).

## Room Management

### room create
```
room create <name>
rcreate <name>
```
Creates a room in your current area.

### room list
```
room list [area_uuid]
rlist / rooms [area_uuid]
```
Lists rooms in specified area or current area.

### room info
```
room info <uuid>
rinfo <uuid>
```
Shows detailed room information including exits.

### room edit
```
room edit <uuid> <field> <value>
redit <uuid> <field> <value>
```

**Fields:** `name`, `description`, `area`

**Example:**
```
room edit <uuid> name The Ancient Oak
room edit <uuid> description A massive oak tree towers above you
room edit <uuid> area <area_uuid>
```

### room delete
```
room delete <uuid>
rdelete <uuid>
```
Deletes a room and removes all exits leading to it.

### room delete bulk
```
room delete bulk <area_uuid>
rdelete bulk <area_uuid>
```
Deletes all rooms in an area. **Use with caution!**

### room search
```
room search <query> [area_uuid]
rsearch <query> [area_uuid]
```
Searches rooms by name, optionally filtered by area.

## Exit Management

### exit create
```
exit create <direction> <target_room_uuid>
xcreate <direction> <target_room_uuid>
```

Creates one-way exit from current room.

**Directions:** `north`, `south`, `east`, `west`, `up`, `down`, `northeast`, `northwest`, `southeast`, `southwest`

### exit delete
```
exit delete <direction>
xdelete <direction>
```
Deletes exit from current room.

### exit edit
```
exit edit <direction> <property> <value>
xedit <direction> <property> <value>
```

**Properties:**
- `closeable` (true/false) - Can be closed
- `closed` (true/false) - Currently closed (requires closeable)
- `lockable` (true/false) - Can be locked (requires closeable)
- `locked` (true/false) - Currently locked (requires lockable)
- `door_rating` (number) - Door strength (requires closeable)
- `lock_rating` (number) - Lock difficulty (requires lockable)
- `unlock_code` (text) - Key/code needed (requires lockable)

**Example:**
```
exit edit north closeable true
exit edit north closed true
exit edit north lockable true
exit edit north locked true
exit edit north door_rating 5
exit edit north lock_rating 10
exit edit north unlock_code secret123
```

### dig
```
dig <direction> <room_name>
```

Creates a new room and bidirectional exits in one command. **Fastest way to build!**

**Example:**
```
dig north A Dark Cave
```

## Item Management

### item create
```
item create <name> | <description> | <weight> [| weapon <min> <max> <type>] [| armor <defense>]
icreate <name> | <description> | <weight> [| weapon <min> <max> <type>] [| armor <defense>]
```

Creates an item in current room.

**Damage Types:** `slashing`, `piercing`, `blunt`, `fire`, `acid`, `arcane`, `psychic`

**Examples:**
```
item create Rusty Sword | An old, rusty sword | 3.5
item create Steel Longsword | A finely crafted longsword | 4.0 | weapon 5 10 slashing
item create Leather Armor | Sturdy leather armor | 8.0 | armor 3
item create Magic Staff | A staff crackling with energy | 3.0 | weapon 3 8 arcane
```

### item edit
```
item edit <uuid> <field> <value>
iedit <uuid> <field> <value>
```

**Fields:** `name`, `description`, `weight`, `weapon`, `armor`

**Examples:**
```
item edit <uuid> name Shining Sword
item edit <uuid> description A sword that gleams in the light
item edit <uuid> weight 4.5
item edit <uuid> weapon 6 12 slashing
item edit <uuid> armor 5
```

### item clone
```
item clone <uuid> [new_name]
iclone <uuid> [new_name]
```
Creates a copy of an item.

### item list
```
item list [query]
ilist / items [query]
```
Lists items in current room, optionally filtered by name.

### item info
```
item info <uuid>
iinfo <uuid>
```
Shows detailed item information.

### item spawn
```
item spawn <template_name> [quantity]
ispawn <template_name> [quantity]
```

Spawns items from templates (1-100 quantity).

**Example:**
```
item spawn longsword
item spawn potion 10
```

### item templates
```
item templates [filter]
itemplates [filter]
```

Lists available templates, optionally filtered.

**Available Templates:**

**Weapons:**
- `shortsword` - Short Sword [3-6 Slashing]
- `longsword` - Long Sword [5-10 Slashing]
- `dagger` - Dagger [2-4 Piercing]
- `mace` - Mace [4-8 Blunt]
- `staff` - Wooden Staff [2-6 Arcane]

**Armor:**
- `leather_armor` - Leather Armor [Defense: 2]
- `chainmail` - Chainmail Armor [Defense: 5]
- `plate_armor` - Plate Armor [Defense: 8]

**Miscellaneous:**
- `torch` - Torch
- `rope` - Rope
- `backpack` - Backpack
- `potion` - Health Potion

## Tips

1. **Get UUIDs first** - Use `area list` and `room list` before editing
2. **Use `dig` for speed** - Creates room + exits in one command
3. **Test exits** - Use `room info` to verify connections
4. **Use templates** - Much faster than manual item creation
5. **Search commands** - Find things quickly with `area search` and `room search`
6. **Be careful with bulk** - `room delete bulk` cannot be undone
7. **Set exit properties after** - Create exits first, then edit properties
8. **Clone for variations** - Use `item clone` then `item edit`
9. **Descriptive names** - Good names help players navigate
10. **Document areas** - Use area descriptions for builder notes

## Command Summary

| Command | Aliases | Purpose |
|---------|---------|---------|
| area create | acreate | Create area |
| area list | alist, areas | List areas |
| area info | ainfo | Area details |
| area edit | aedit | Edit area |
| area delete | adelete | Delete area |
| area search | asearch | Search areas |
| room create | rcreate | Create room |
| room list | rlist, rooms | List rooms |
| room info | rinfo | Room details |
| room edit | redit | Edit room |
| room delete | rdelete | Delete room |
| room delete bulk | - | Delete all in area |
| room search | rsearch | Search rooms |
| exit create | xcreate | Create exit |
| exit delete | xdelete | Delete exit |
| exit edit | xedit | Edit exit |
| dig | - | Create room + exits |
| item create | icreate | Create item |
| item edit | iedit | Edit item |
| item clone | iclone | Clone item |
| item list | ilist, items | List items |
| item info | iinfo | Item details |
| item spawn | ispawn | Spawn from template |
| item templates | itemplates | List templates |

## See Also
- [LLM Generation](LLM_GENERATION.md) - AI-powered content generation
- [NPC System](NPC_SYSTEM.md) - Creating and managing NPCs
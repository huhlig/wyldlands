-- Migration: Add Help System
-- This migration adds tables for storing help topics for commands, skills, talents, etc.

BEGIN;

SET search_path TO wyldlands, public;

-- Insert default help topics

-- General help
INSERT INTO wyldlands.help_topics (keyword, category, title, content, see_also)
VALUES ('help', 'System', 'Help System', 
'The help system provides information about commands, skills, talents, and other game features.

Available help commands:
  help              - Show basic help and available help commands
  help commands     - List all available commands
  help <keyword>    - Get detailed help about a specific topic

You can get help on any command, skill, talent, or game feature by typing "help" followed by the keyword. For example:
  help look
  help inventory
  help combat

The help system is searchable and will try to find the most relevant topic for your query.',
ARRAY['commands', 'topics']);

-- Commands help
INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, see_also)
VALUES ('commands', 'System', 'Available Commands',
'This shows a complete list of all available commands in the game. Commands are organized by category for easier navigation.

Use "help <command>" to get detailed information about any specific command.',
'help commands',
ARRAY['help']);

-- Movement commands
INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('movement', 'Command', 'Movement Commands',
'Movement commands allow you to navigate through the game world. You can move in cardinal directions (north, south, east, west), diagonal directions (northeast, northwest, southeast, southwest), and vertically (up, down).

Each room may have different exits available. Use the "look" command to see available exits.',
'<direction>
north, south, east, west, up, down, ne, nw, se, sw',
'north
n
northeast
ne',
ARRAY['look', 'exits', 'goto']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('look', 'Command', 'Look Command',
'The look command allows you to examine your surroundings or specific objects, characters, or features in the room.

When used without arguments, it shows the current room description, visible exits, items, and other characters present.

When used with a target, it provides detailed information about that specific thing.',
'look [target]',
'look
look sword
look merchant
l',
ARRAY['movement', 'examine', 'inventory']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('inventory', 'Command', 'Inventory Command',
'The inventory command shows all items you are currently carrying. It displays item names, quantities, and may show weight or other relevant information depending on your character''s carrying capacity.',
'inventory',
'inventory
inv
i',
ARRAY['look', 'get', 'drop', 'equipment']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('say', 'Command', 'Say Command',
'The say command allows you to speak to other characters in the same room. Your message will be visible to everyone present.',
'say <message>',
'say Hello everyone!
'' Hello everyone!',
ARRAY['yell', 'emote', 'tell', 'whisper']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('yell', 'Command', 'Yell Command',
'The yell command allows you to shout a message that can be heard in nearby rooms, not just your current location. The range depends on the area and environmental factors.',
'yell <message>',
'yell Help! I''m under attack!
" Help! I''m under attack!',
ARRAY['say', 'emote', 'tell']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('emote', 'Command', 'Emote Command',
'The emote command allows you to perform actions or express emotions. It displays your character name followed by your action.',
'emote <action>',
'emote waves hello
em smiles warmly
: laughs heartily',
ARRAY['say', 'yell', 'social']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('score', 'Command', 'Score Command',
'The score command displays your character''s statistics, including attributes, skills, health, and other important information about your character''s current state.',
'score',
'score
stats',
ARRAY['inventory', 'skills', 'attributes']);

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also)
VALUES ('exit', 'Command', 'Exit Command',
'The exit command saves your character and returns you to the character selection screen. Your character''s progress is automatically saved.',
'exit',
'exit
quit
logout
logoff',
ARRAY['save']);

-- Builder commands
INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, see_also, min_role)
VALUES ('building', 'Building', 'Building System',
'The building system allows authorized builders to create and modify areas, rooms, items, and NPCs in the game world.

Main building commands:
  area   - Create and manage areas
  room   - Create and manage rooms
  exit   - Create and manage exits between rooms
  dig    - Quick command to create and connect rooms
  item   - Create and manage items
  npc    - Create and manage NPCs

Use "help <command>" for detailed information about each building command.',
'See individual command help topics',
ARRAY['area', 'room', 'exit', 'dig', 'item', 'npc'],
'builder');

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also, min_role)
VALUES ('area', 'Building', 'Area Commands',
'Area commands allow you to create and manage game areas. Areas are large regions that contain multiple rooms.

Available area commands:
  area create <name>                    - Create a new area
  area list [filter]                    - List all areas
  area info <uuid>                      - Show detailed area information
  area edit <uuid> <property> <value>   - Edit area properties
  area delete <uuid>                    - Delete an area (if empty)
  area search <query>                   - Search for areas by name',
'area <subcommand> [arguments]',
'area create "Dark Forest"
area list
area info 12345678-1234-1234-1234-123456789abc
area edit 12345678-1234-1234-1234-123456789abc name "Darker Forest"',
ARRAY['room', 'building'],
'builder');

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also, min_role)
VALUES ('room', 'Building', 'Room Commands',
'Room commands allow you to create and manage individual rooms within areas.

Available room commands:
  room create <area-uuid> <name>        - Create a new room
  room list [area-uuid]                 - List rooms in area or all rooms
  room goto <uuid>                      - Teleport to a room
  room edit <uuid> <field> <value>      - Edit room properties
  room deleteall <area-uuid>            - Delete all rooms in an area',
'room <subcommand> [arguments]',
'room create 12345678-1234-1234-1234-123456789abc "Forest Clearing"
room list
room goto 87654321-4321-4321-4321-cba987654321
room edit 87654321-4321-4321-4321-cba987654321 description "A peaceful clearing"',
ARRAY['area', 'exit', 'dig', 'building'],
'builder');

INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, examples, see_also, min_role)
VALUES ('dig', 'Building', 'Dig Command',
'The dig command is a quick way to create a new room and automatically connect it to your current location with exits in both directions.

Options:
  oneway        - Create only a one-way exit (from current room to new room)
  area <uuid>   - Create the new room in a specific area (otherwise uses current area)',
'dig <direction> <name> [oneway] [area <uuid>]',
'dig north "Northern Path"
dig east "Secret Cave" oneway
dig south "Garden" area 12345678-1234-1234-1234-123456789abc',
ARRAY['room', 'exit', 'building'],
'builder');

-- Admin commands
INSERT INTO wyldlands.help_topics (keyword, category, title, content, syntax, see_also, min_role)
VALUES ('world', 'System', 'World Commands',
'World commands provide administrative control over the game world and entity system.

Available world commands:
  world inspect <uuid>  - Query all components of an entity
  world list            - List all entities with their UUIDs
  world save            - Save all persistent entities to database
  world reload          - Clear ECS and reload entities from database',
'world <subcommand> [arguments]',
ARRAY['admin', 'building'],
'admin');

-- Create aliases for common help topics
INSERT INTO wyldlands.help_aliases (alias, keyword) VALUES
('l', 'look'),
('inv', 'inventory'),
('i', 'inventory'),
('''', 'say'),
('"', 'yell'),
('em', 'emote'),
(':', 'emote'),
('stats', 'score'),
('quit', 'exit'),
('logout', 'exit'),
('logoff', 'exit'),
('n', 'movement'),
('s', 'movement'),
('e', 'movement'),
('w', 'movement'),
('ne', 'movement'),
('nw', 'movement'),
('se', 'movement'),
('sw', 'movement'),
('u', 'movement'),
('d', 'movement'),
('north', 'movement'),
('south', 'movement'),
('east', 'movement'),
('west', 'movement'),
('up', 'movement'),
('down', 'movement'),
('northeast', 'movement'),
('northwest', 'movement'),
('southeast', 'movement'),
('southwest', 'movement');

COMMIT;

-- Made with Bob
-- Migration: Insert World Data
-- This migration adds default banner values to the settings table

BEGIN;

SET search_path TO wyldlands, public;

-- Insert Special Area Zero (Developer Area)
INSERT INTO wyldlands.entities (uuid)
VALUES ('00000000-0000-0000-0000-000000000000');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('00000000-0000-0000-0000-000000000000', 'The Void', ARRAY ['void', 'nowhere']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('00000000-0000-0000-0000-000000000000',
        'A dark void of nothingness',
        'You float in an endless void. This is the default location for entities that have no proper location.');
INSERT INTO wyldlands.entity_areas (entity_id, area_kind, area_flags)
VALUES ('00000000-0000-0000-0000-000000000000', 'Overworld', ARRAY []::area_flag[]);

-- Insert Special Room Zero (Default Spawn Room)
INSERT INTO wyldlands.entities (uuid)
VALUES ('00000000-0000-0000-0000-000000000001');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('00000000-0000-0000-0000-000000000001', 'The Void Room', ARRAY ['void', 'room']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('00000000-0000-0000-0000-000000000001',
        'A small pocket of stability in the void',
        'You stand in a small bubble of reality within the endless void. Somehow, you can breathe here.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Area
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000000');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000000', 'Developer Testing Grounds',
        ARRAY ['developer', 'testing', 'grounds', 'dev']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000000',
        'A special area for developers to test features',
        'This area exists outside of normal space and time, created specifically for testing game mechanics and features. The laws of reality are more... flexible here.');
INSERT INTO wyldlands.entity_areas (entity_id, area_kind, area_flags)
VALUES ('10000000-0000-0000-0000-000000000000', 'Building', ARRAY []::area_flag[]);

-- Developer Room 1: Central Hub
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000001');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000001', 'Developer Hub', ARRAY ['hub', 'center', 'central']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000001',
        'A bright, clean room with exits in all directions',
        'This is the central hub of the developer testing area. The room is perfectly square with white walls that seem to glow with their own light. Exits lead in all cardinal directions, and there are stairs leading both up and down. A large sign on the wall reads: "DEVELOPER TESTING AREA - ALL FEATURES ENABLED".');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Room 2: North Room (Combat Testing)
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000002');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000002', 'Combat Testing Arena', ARRAY ['combat', 'arena', 'testing', 'north']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000002',
        'A sandy arena for testing combat mechanics',
        'This circular arena is covered in sand and surrounded by high walls. Weapon racks line the perimeter, and training dummies stand ready for abuse. The air smells of sweat and determination. This is where combat systems are put through their paces.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Room 3: South Room (Item Testing)
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000003');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000003', 'Item Repository', ARRAY ['item', 'repository', 'storage', 'south']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000003',
        'A warehouse filled with testing items',
        'Shelves upon shelves stretch into the distance, each laden with items of every conceivable type. Weapons, armor, potions, scrolls, and stranger things fill this vast repository. A clipboard on the wall tracks which items have been tested and which still need attention.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Room 4: East Room (NPC/AI Testing)
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000004');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000004', 'AI Behavior Lab', ARRAY ['ai', 'behavior', 'lab', 'npc', 'east']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000004',
        'A laboratory for testing NPC behaviors',
        'This sterile laboratory contains observation windows, control panels, and several containment areas. Various NPCs wander about, each exhibiting different behavioral patterns. Monitors display real-time data about their decision-making processes and goal states.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000004', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Room 5: West Room (Environment Testing)
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000005');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000005', 'Environment Simulator',
        ARRAY ['environment', 'simulator', 'weather', 'west']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000005',
        'A room that can simulate any environment',
        'This remarkable room can simulate any environmental condition imaginable. Currently, it appears to be cycling through different weather patterns - one moment sunny, the next raining, then snowing. Control panels on the walls allow you to adjust temperature, humidity, lighting, and more.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000005', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Room 6: Upper Room (Command Testing)
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000006');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000006', 'Command Console',
        ARRAY ['command', 'console', 'terminal', 'upper', 'up']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000006',
        'A high-tech command center',
        'Banks of monitors and terminals fill this elevated command center. From here, developers can execute any command, monitor system performance, and observe the entire testing area. The hum of computers fills the air, and the glow of screens provides the only illumination.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000006', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Developer Room 7: Lower Room (Database Testing)
INSERT INTO wyldlands.entities (uuid)
VALUES ('10000000-0000-0000-0000-000000000007');
INSERT INTO wyldlands.entity_name (entity_id, display, keywords)
VALUES ('10000000-0000-0000-0000-000000000007', 'Data Vault', ARRAY ['data', 'vault', 'database', 'lower', 'down']);
INSERT INTO wyldlands.entity_description (entity_id, short, long)
VALUES ('10000000-0000-0000-0000-000000000007',
        'A secure vault for persistence testing',
        'This underground vault is where all data persistence is tested. Rows of server racks line the walls, their LEDs blinking in complex patterns. The cool air and constant hum of cooling fans create an almost meditative atmosphere. Here, you can test save/load functionality and database operations.');
INSERT INTO wyldlands.entity_rooms (entity_id, area_id, room_flags)
VALUES ('10000000-0000-0000-0000-000000000007', '10000000-0000-0000-0000-000000000000',
        ARRAY ['Breathable']::room_flag[]);

-- Exits: Hub to North (Combat Arena)
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000002', 'North', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: North (Combat Arena) to Hub
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001', 'South', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Hub to South (Item Repository)
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000003', 'South', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: South (Item Repository) to Hub
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000001', 'North', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Hub to East (AI Lab)
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000004', 'East', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: East (AI Lab) to Hub
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000004', '10000000-0000-0000-0000-000000000001', 'West', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Hub to West (Environment Simulator)
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000005', 'West', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: West (Environment Simulator) to Hub
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000005', '10000000-0000-0000-0000-000000000001', 'East', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Hub to Up (Command Console)
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000006', 'Up', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Up (Command Console) to Hub
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000006', '10000000-0000-0000-0000-000000000001', 'Down', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Hub to Down (Data Vault)
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000007', 'Down', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Exits: Down (Data Vault) to Hub
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, lockable, locked,
                                         transparent)
VALUES ('10000000-0000-0000-0000-000000000007', '10000000-0000-0000-0000-000000000001', 'Up', FALSE, FALSE, FALSE,
        FALSE, TRUE);

-- Add a locked door example between Combat Arena and AI Lab
INSERT INTO wyldlands.entity_room_exits (entity_id, dest_id, direction, closeable, closed, door_rating, lockable,
                                         locked, unlock_code, lock_rating, transparent)
VALUES ('10000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000004', 'East', TRUE, TRUE, 50, TRUE,
        TRUE, 'dev_key_001', 75, FALSE);


-- Insert default starting locations
INSERT INTO wyldlands.starting_locations (id, name, description, room_id, sort_order)
VALUES ('dev_hub', 'Developer Hub',
        'The central testing area for developers. A safe place to start your journey with access to all testing facilities.',
        '10000000-0000-0000-0000-000000000001', 1),
       ('void_room', 'The Void Room',
        'A pocket of stability in the endless void. The default starting location for those who seek mystery.',
        '00000000-0000-0000-0000-000000000001', 2),
       ('combat_arena', 'Combat Testing Arena',
        'Jump right into combat testing. For the brave and battle-ready who want to test their skills immediately.',
        '10000000-0000-0000-0000-000000000002', 3);


COMMIT;



--
-- PostgreSQL database setup script
--

BEGIN;

--
-- Extensions
--

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS hstore;

--
-- Schema
--

CREATE SCHEMA IF NOT EXISTS wyldlands;

--
-- Name: settings; Type: TABLE; Schema: wyldlands; Owner: wyldlands
--

CREATE TABLE wyldlands.settings
(
    key         VARCHAR(100) PRIMARY KEY,
    value       TEXT,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by  UUID
);

COMMENT ON TABLE wyldlands.settings IS 'Wyldlands Settings';
COMMENT ON COLUMN wyldlands.settings.key IS 'Property Key (e.g., banner.welcome, banner.motd, account.creation_enabled)';
COMMENT ON COLUMN wyldlands.settings.value IS 'Property Value (can be large text for banners)';
COMMENT ON COLUMN wyldlands.settings.description IS 'Information about property';
COMMENT ON COLUMN wyldlands.settings.created_at IS 'When property was created';
COMMENT ON COLUMN wyldlands.settings.updated_at IS 'When property was updated';
COMMENT ON COLUMN wyldlands.settings.updated_by IS 'Who last updated this property';

--
-- Name: accounts; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Matches: common/src/account.rs::Account
--

CREATE TABLE wyldlands.accounts
(
    id         UUID PRIMARY KEY,
    login      VARCHAR     NOT NULL UNIQUE,
    display    VARCHAR     NOT NULL,
    password   VARCHAR     NOT NULL,
    timezone   VARCHAR,
    discord    VARCHAR,
    email      VARCHAR,
    rating     INT         NOT NULL DEFAULT 0,
    active     BOOLEAN     NOT NULL DEFAULT TRUE,
    admin      BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE wyldlands.accounts IS 'Player Accounts';
COMMENT ON COLUMN wyldlands.accounts.id IS 'Account ID';
COMMENT ON COLUMN wyldlands.accounts.login IS 'Account Login Name';
COMMENT ON COLUMN wyldlands.accounts.display IS 'Account Display Name';
COMMENT ON COLUMN wyldlands.accounts.password IS 'Account Password (bcrypt hashed via pgcrypto)';
COMMENT ON COLUMN wyldlands.accounts.timezone IS 'Timezone of the Account';
COMMENT ON COLUMN wyldlands.accounts.discord IS 'Discord ID of the Account';
COMMENT ON COLUMN wyldlands.accounts.email IS 'Email Address of the Account';
COMMENT ON COLUMN wyldlands.accounts.rating IS 'Account Rating';
COMMENT ON COLUMN wyldlands.accounts.active IS 'Account Status';
COMMENT ON COLUMN wyldlands.accounts.admin IS 'Account Admin Status';

--
-- Name: area_kind; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Kinds of Areas
--

CREATE TYPE area_kind AS ENUM ('Overworld', 'Vehicle', 'Building', 'Dungeon');

--
-- Name: area_flag; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Area Flags
--

CREATE TYPE area_flag AS ENUM ('Underwater');

--
-- Name: room_flag; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Room Flags
--

CREATE TYPE room_flag AS ENUM ('Breathable');

--
-- Name: size_class; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Entity Sizes Flags
--

CREATE TYPE size_class AS ENUM ('Fine', 'Diminutive', 'Tiny', 'Small', 'Medium', 'Large', 'Huge', 'Gargantuan', 'Colossal');

--
-- Name: slot_kind; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Entity Equipment Slots
--

CREATE TYPE slot_kind AS ENUM ('Head', 'Chest', 'Legs', 'Feet', 'Hands', 'MainHand', 'OffHand', 'Ring1',
    'Ring2', 'Neck', 'Back', 'Tail', 'Wings');

--
-- Name: damage_type; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Damage Types
--

CREATE TYPE damage_type AS ENUM ('Blunt', 'Piercing', 'Slashing', 'Cold', 'Poison', 'Fire', 'Necrotic', 'Radiant',
    'Electric', 'Acid', 'Arcane', 'Psychic', 'Sonic', 'Force');

--
-- Name: ai_behavior; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of AI Behaviors
--

CREATE TYPE ai_behavior AS ENUM ('Passive', 'Wandering', 'Aggressive', 'Defensive', 'Friendly', 'Merchant',
    'Quest', 'Custom');

--
-- Name: ai_state; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of AI States
--

CREATE TYPE ai_state AS ENUM ('Idle', 'Moving', 'Combat', 'Fleeing', 'Following', 'Dialogue');

--
-- Name: session_state; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Session States
--

CREATE TYPE session_state AS ENUM ('Connecting', 'Authenticating', 'CharacterSelection', 'Playing', 'Disconnected', 'Closed');

--
-- Name: session_protocol; Type: ENUM; Schema: wyldlands; Owner: wyldlands
-- Enumeration of Session Protocol
--

CREATE TYPE session_protocol AS ENUM ('Telnet', 'WebSocket');

--
-- Name: entities; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Base table for all ECS entities
--

CREATE TABLE wyldlands.entities
(
    uuid       UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_entities_updated_at ON wyldlands.entities (updated_at);

COMMENT ON TABLE wyldlands.entities IS 'Base table for all ECS entities';
COMMENT ON COLUMN wyldlands.entities.uuid IS 'Entity UUID (matches ECS EntityUuid component)';
COMMENT ON COLUMN wyldlands.entities.created_at IS 'When entity was first created';
COMMENT ON COLUMN wyldlands.entities.updated_at IS 'Last save timestamp';

------------------------------------------------------------------------------------------------------------------------
-- ECS Component Tables                                                                                              ---
------------------------------------------------------------------------------------------------------------------------

--
-- Name: entity_avatars; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Minimal linkage table between accounts and player entities
--

CREATE TABLE wyldlands.entity_avatars
(
    entity_id   UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    account_id  UUID        NOT NULL REFERENCES wyldlands.accounts (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_played TIMESTAMPTZ,
    available   BOOLEAN     NOT NULL DEFAULT TRUE,
    PRIMARY KEY (account_id, entity_id)
);

CREATE INDEX idx_avatars_account_id ON wyldlands.entity_avatars (account_id);
CREATE INDEX idx_avatars_entity_id ON wyldlands.entity_avatars (entity_id);
CREATE INDEX idx_avatars_last_played ON wyldlands.entity_avatars (last_played DESC);

COMMENT ON TABLE wyldlands.entity_avatars IS 'Links player accounts to their character entities';
COMMENT ON COLUMN wyldlands.entity_avatars.account_id IS 'Account that owns this avatar';
COMMENT ON COLUMN wyldlands.entity_avatars.entity_id IS 'Entity ID of the player character';
COMMENT ON COLUMN wyldlands.entity_avatars.created_at IS 'When avatar was created';
COMMENT ON COLUMN wyldlands.entity_avatars.last_played IS 'Last time this avatar was played';
COMMENT ON COLUMN wyldlands.entity_avatars.available IS 'Whether this avatar is available for play (can be disabled)';

-- Identity Components

--
-- Name: entity_name; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component providing a Name and keywords for an entity
--

CREATE TABLE wyldlands.entity_name
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    display   VARCHAR(255) NOT NULL,
    keywords  TEXT[]       NOT NULL DEFAULT '{}'
);

COMMENT ON TABLE wyldlands.entity_name IS 'Name component - display name and keywords';
COMMENT ON COLUMN wyldlands.entity_name.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_name.display IS 'Display Name of the object';
COMMENT ON COLUMN wyldlands.entity_name.keywords IS 'Keywords for object access';

--
-- Name: entity_description; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component providing a Name and keywords for an entity
--

CREATE TABLE wyldlands.entity_description
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    short     TEXT NOT NULL,
    long      TEXT NOT NULL
);

COMMENT ON TABLE wyldlands.entity_description IS 'Description component - short and long descriptions';
COMMENT ON COLUMN wyldlands.entity_description.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_description.short IS 'Short description of Entity';
COMMENT ON COLUMN wyldlands.entity_description.long IS 'Long description of entity';

-- Map Components


--
-- Name: entity_areas; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Areas represent groups of rooms forming a region
--

CREATE TABLE wyldlands.entity_areas
(
    entity_id  UUID PRIMARY KEY NOT NULL,
    area_kind  area_kind,
    area_flags area_flag[]
);

COMMENT ON TABLE wyldlands.entity_areas IS 'Component for Areas';
COMMENT ON COLUMN wyldlands.entity_areas.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_areas.area_kind IS 'Type of the Area';
COMMENT ON COLUMN wyldlands.entity_areas.area_flags IS 'Area Flags';

--
-- Name: entity_rooms; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component for indicating an entity is a room
--

CREATE TABLE wyldlands.entity_rooms
(
    entity_id  UUID PRIMARY KEY NOT NULL,
    area_id    UUID             NOT NULL,
    room_flags room_flag[]
);

COMMENT ON TABLE wyldlands.entity_rooms IS 'Component for Room entities';
COMMENT ON COLUMN wyldlands.entity_rooms.entity_id IS 'Entity ID of the Room';
COMMENT ON COLUMN wyldlands.entity_rooms.area_id IS 'ID of Area Entity this room belongs to';
COMMENT ON COLUMN wyldlands.entity_rooms.room_flags IS 'Room Flags';

--
-- Name: entity_; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Individual exits to a room entity
--

CREATE TABLE wyldlands.entity_room_exits
(
    entity_id   UUID    NOT NULL,
    dest_id     UUID    NOT NULL,
    direction   VARCHAR(10),
    closeable   BOOLEAN NOT NULL DEFAULT FALSE,
    closed      BOOLEAN NOT NULL DEFAULT FALSE,
    door_rating INT,
    lockable    BOOLEAN NOT NULL DEFAULT FALSE,
    locked      BOOLEAN NOT NULL DEFAULT FALSE,
    unlock_code VARCHAR(100),
    lock_rating INT,
    transparent BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (entity_id, direction)
);

COMMENT ON TABLE wyldlands.entity_room_exits IS 'Exits from room entities';
COMMENT ON COLUMN wyldlands.entity_room_exits.entity_id IS 'ID of the Room containing this exit';
COMMENT ON COLUMN wyldlands.entity_room_exits.dest_id IS 'Destination for this exit';
COMMENT ON COLUMN wyldlands.entity_room_exits.direction IS 'Direction this exit occupies (North, South, East West, Up, Down, NorthWest, etc)';
COMMENT ON COLUMN wyldlands.entity_room_exits.closeable IS 'Does this exit have a door';
COMMENT ON COLUMN wyldlands.entity_room_exits.closed IS 'Is this door closed';
COMMENT ON COLUMN wyldlands.entity_room_exits.door_rating IS 'Quality of this door against attacks';
COMMENT ON COLUMN wyldlands.entity_room_exits.lockable IS 'Does this exit have a lock';
COMMENT ON COLUMN wyldlands.entity_room_exits.locked IS 'Is this exit currently locked';
COMMENT ON COLUMN wyldlands.entity_room_exits.unlock_code IS 'Unlock code a spell, key, or ability must provide';
COMMENT ON COLUMN wyldlands.entity_room_exits.lock_rating IS 'Quality of this lock against attacks';
COMMENT ON COLUMN wyldlands.entity_room_exits.transparent IS 'Is the Door See through';


-- Character Components

--
-- Name: entity_body_attributes; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component providing Sapient Body Attributes
--

CREATE TABLE wyldlands.entity_body_attributes
(
    entity_id      UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    score_offence  INTEGER NOT NULL DEFAULT 10,
    score_finesse  INTEGER NOT NULL DEFAULT 10,
    score_defence  INTEGER NOT NULL DEFAULT 10,
    health_current REAL    NOT NULL,
    health_maximum REAL    NOT NULL,
    health_regen   REAL    NOT NULL DEFAULT 1.0,
    energy_current REAL    NOT NULL,
    energy_maximum REAL    NOT NULL,
    energy_regen   REAL    NOT NULL DEFAULT 1.0
);

COMMENT ON TABLE wyldlands.entity_body_attributes IS 'Attributes component - core character stats';
COMMENT ON COLUMN wyldlands.entity_body_attributes.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_body_attributes.score_offence IS 'Physical Offense Score';
COMMENT ON COLUMN wyldlands.entity_body_attributes.score_finesse IS 'Physical Finesse Score';
COMMENT ON COLUMN wyldlands.entity_body_attributes.score_defence IS 'Physical Defence Score';
COMMENT ON COLUMN wyldlands.entity_body_attributes.health_current IS 'Current Physical Health';
COMMENT ON COLUMN wyldlands.entity_body_attributes.health_maximum IS 'Maximum Physical Health';
COMMENT ON COLUMN wyldlands.entity_body_attributes.health_regen IS 'Physical Health Regeneration';
COMMENT ON COLUMN wyldlands.entity_body_attributes.energy_current IS 'Current Physical Energy';
COMMENT ON COLUMN wyldlands.entity_body_attributes.energy_maximum IS 'Maximum Physical Energy';
COMMENT ON COLUMN wyldlands.entity_body_attributes.energy_regen IS 'Physical Energy Regeneration';

--
-- Name: entity_mind_attributes; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component providing Sapient Mind Attributes
--

CREATE TABLE wyldlands.entity_mind_attributes
(
    entity_id      UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    score_offence  INTEGER NOT NULL DEFAULT 10,
    score_finesse  INTEGER NOT NULL DEFAULT 10,
    score_defence  INTEGER NOT NULL DEFAULT 10,
    health_current REAL    NOT NULL,
    health_maximum REAL    NOT NULL,
    health_regen   REAL    NOT NULL DEFAULT 1.0,
    energy_current REAL    NOT NULL,
    energy_maximum REAL    NOT NULL,
    energy_regen   REAL    NOT NULL DEFAULT 1.0
);

COMMENT ON TABLE wyldlands.entity_mind_attributes IS 'Attributes component - core character stats';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.score_offence IS 'Mental Offense Score';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.score_finesse IS 'Mental Finesse Score';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.score_defence IS 'Mental Defence Score';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.health_current IS 'Current Mental Health';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.health_maximum IS 'Maximum Mental Health';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.health_regen IS 'Mental Health Regeneration';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.energy_current IS 'Current Mental Energy';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.energy_maximum IS 'Maximum Mental Energy';
COMMENT ON COLUMN wyldlands.entity_mind_attributes.energy_regen IS 'Mental Energy Regeneration';

--
-- Name: entity_soul_attributes; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component providing Sapient Soul Attributes
--

CREATE TABLE wyldlands.entity_soul_attributes
(
    entity_id      UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    score_offence  INTEGER NOT NULL DEFAULT 10,
    score_finesse  INTEGER NOT NULL DEFAULT 10,
    score_defence  INTEGER NOT NULL DEFAULT 10,
    health_current REAL    NOT NULL,
    health_maximum REAL    NOT NULL,
    health_regen   REAL    NOT NULL DEFAULT 1.0,
    energy_current REAL    NOT NULL,
    energy_maximum REAL    NOT NULL,
    energy_regen   REAL    NOT NULL DEFAULT 1.0
);

COMMENT ON TABLE wyldlands.entity_soul_attributes IS 'Attributes component - core character stats';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.score_offence IS 'Spiritual Offense Score';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.score_finesse IS 'Spiritual Finesse Score';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.score_defence IS 'Spiritual Defence Score';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.health_current IS 'Current Spiritual Health';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.health_maximum IS 'Maximum Spiritual Health';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.health_regen IS 'Mental Spiritual Regeneration';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.energy_current IS 'Current Spiritual Energy';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.energy_maximum IS 'Maximum Spiritual Energy';
COMMENT ON COLUMN wyldlands.entity_soul_attributes.energy_regen IS 'Spiritual Energy Regeneration';

--
-- Name: entity_skills; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Component providing an Entities Skill list
--

CREATE TABLE wyldlands.entity_skills
(
    entity_id  UUID         NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    skill_name VARCHAR(100) NOT NULL,
    level      INTEGER      NOT NULL DEFAULT 1,
    experience INTEGER      NOT NULL DEFAULT 0,
    knowledge  INTEGER      NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_id, skill_name)
);

COMMENT ON TABLE wyldlands.entity_skills IS 'Skills component - individual skill levels';
COMMENT ON COLUMN wyldlands.entity_skills.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_skills.skill_name IS 'Name of Skill';
COMMENT ON COLUMN wyldlands.entity_skills.level IS 'Current Skill level';
COMMENT ON COLUMN wyldlands.entity_skills.experience IS 'How much experience is in the skill';
COMMENT ON COLUMN wyldlands.entity_skills.knowledge IS 'How much knowledge is in the skill';

-- Spatial Components

--
-- Name: entity_location; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Location of an Entity in the World (Room_id is enough)
--

CREATE TABLE wyldlands.entity_location
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    room_id   UUID NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE
);

CREATE INDEX idx_position_area_room ON wyldlands.entity_location (room_id);

COMMENT ON TABLE wyldlands.entity_location IS 'Position component - Location in world';
COMMENT ON COLUMN wyldlands.entity_location.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_location.room_id IS 'Room ID the object is in';

--
-- Name: entity_container; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity is a container that things can be put in. Separate from a room.
--

CREATE TABLE wyldlands.entity_container
(
    entity_id        UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    capacity         INTEGER,
    max_weight       REAL,
    closeable        BOOLEAN NOT NULL DEFAULT FALSE,
    closed           BOOLEAN NOT NULL DEFAULT FALSE,
    container_rating INT,
    lockable         BOOLEAN NOT NULL DEFAULT FALSE,
    locked           BOOLEAN NOT NULL DEFAULT FALSE,
    unlock_code      VARCHAR(100),
    lock_rating      INT,
    transparent      BOOLEAN NOT NULL DEFAULT FALSE
);

COMMENT ON TABLE wyldlands.entity_container IS 'Container component - holds other entities';
COMMENT ON COLUMN wyldlands.entity_container.entity_id IS 'Entity ID of the object';
COMMENT ON COLUMN wyldlands.entity_container.capacity IS 'Capacity of Container';
COMMENT ON COLUMN wyldlands.entity_container.max_weight IS 'Max Weight of Container';
COMMENT ON COLUMN wyldlands.entity_container.closeable IS 'Is Container Closable';
COMMENT ON COLUMN wyldlands.entity_container.closed IS 'Is Container Closed';
COMMENT ON COLUMN wyldlands.entity_container.container_rating IS 'Durability Rating of Container';
COMMENT ON COLUMN wyldlands.entity_container.lockable IS 'Is Container Lockable';
COMMENT ON COLUMN wyldlands.entity_container.locked IS 'Is object currently locked';
COMMENT ON COLUMN wyldlands.entity_container.unlock_code IS 'Internal Code a Skill, Spell, or Key must have to unlock';
COMMENT ON COLUMN wyldlands.entity_container.lock_rating IS 'Rating of Container Lock';
COMMENT ON COLUMN wyldlands.entity_container.transparent IS 'Is Container Transparent';

--
-- Name: entity_container; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity is a container that things can be put in. Separate from a room.
--

CREATE TABLE wyldlands.entity_container_contents
(
    container_id UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    content_id   UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    added_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (container_id, content_id)
);

CREATE INDEX idx_container_contents_container ON wyldlands.entity_container_contents (container_id);

COMMENT ON TABLE wyldlands.entity_container_contents IS 'Contents of containers';
COMMENT ON COLUMN wyldlands.entity_container_contents.container_id IS 'Entity ID of container';
COMMENT ON COLUMN wyldlands.entity_container_contents.content_id IS 'Entity ID of Object IN container';
COMMENT ON COLUMN wyldlands.entity_container_contents.added_at IS 'When the object was put in the container';

--
-- Name: entity_containable; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Indicates and entity can be placed in a container.
--


CREATE TABLE wyldlands.entity_containable
(
    entity_id  UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    weight     REAL       NOT NULL,
    size       size_class NOT NULL,
    stackable  BOOLEAN    NOT NULL DEFAULT FALSE,
    stack_size INTEGER    NOT NULL DEFAULT 1
);

COMMENT ON TABLE wyldlands.entity_containable IS 'Containable component - properties of items that can be contained';
COMMENT ON COLUMN wyldlands.entity_containable.entity_id IS 'Entity ID of containable object';
COMMENT ON COLUMN wyldlands.entity_containable.weight IS 'Weight of containable object';
COMMENT ON COLUMN wyldlands.entity_containable.size IS 'Size of containable object';
COMMENT ON COLUMN wyldlands.entity_containable.stackable IS 'Stackable Object';
COMMENT ON COLUMN wyldlands.entity_containable.stack_size IS 'Maximum Stack Size';

--
-- Name: entity_enterable; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Indicates an entity can be entered, serves as an exit to/from an entity.
--

CREATE TABLE wyldlands.entity_enterable
(
    entity_id   UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    dest_id     UUID    NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    closeable   BOOLEAN NOT NULL DEFAULT FALSE,
    closed      BOOLEAN NOT NULL DEFAULT FALSE,
    door_rating INT,
    lockable    BOOLEAN NOT NULL DEFAULT FALSE,
    locked      BOOLEAN NOT NULL DEFAULT FALSE,
    unlock_code VARCHAR(100),
    lock_rating INT,
    transparent BOOLEAN NOT NULL DEFAULT FALSE
);

COMMENT ON TABLE wyldlands.entity_enterable IS 'Enterable component - rooms, vehicles, etc.';
COMMENT ON COLUMN wyldlands.entity_enterable.entity_id IS 'Entity ID of containable object';
COMMENT ON COLUMN wyldlands.entity_enterable.dest_id IS 'Room associated with this entrance';
COMMENT ON COLUMN wyldlands.entity_enterable.closeable IS 'Does this exit have a door';
COMMENT ON COLUMN wyldlands.entity_enterable.closed IS 'Is this door closed';
COMMENT ON COLUMN wyldlands.entity_enterable.door_rating IS 'Quality of this door against attacks';
COMMENT ON COLUMN wyldlands.entity_enterable.lockable IS 'Does this exit have a lock';
COMMENT ON COLUMN wyldlands.entity_enterable.unlock_code IS 'Unlock code a spell, key, or ability must provide';
COMMENT ON COLUMN wyldlands.entity_enterable.lock_rating IS 'Quality of this door against attacks';
COMMENT ON COLUMN wyldlands.entity_enterable.transparent IS 'Is the Door See through';

--
-- Name: entity_occupants; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Occupants of any room.
--

CREATE TABLE wyldlands.entity_occupants
(
    entity_id   UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    occupant_id UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    entered_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, occupant_id)
);

CREATE INDEX idx_entity_occupant ON wyldlands.entity_occupants (occupant_id);

COMMENT ON TABLE wyldlands.entity_occupants IS 'Occupants of Room Entities';
COMMENT ON COLUMN wyldlands.entity_occupants.entity_id IS 'Entity Of Room';
COMMENT ON COLUMN wyldlands.entity_occupants.occupant_id IS 'Entity inside Room';
COMMENT ON COLUMN wyldlands.entity_occupants.entered_at IS 'When entity entered room';

-- Combat Components

--
-- Name: entity_combatant; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Combat Capable Entity.
--

CREATE TABLE wyldlands.entity_combatant
(
    entity_id         UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    in_combat         BOOLEAN NOT NULL DEFAULT FALSE,
    target_id         UUID    REFERENCES wyldlands.entities (uuid) ON DELETE SET NULL,
    initiative        INTEGER NOT NULL DEFAULT 0,
    action_cooldown   REAL    NOT NULL DEFAULT 1.0,
    time_since_action REAL    NOT NULL DEFAULT 0.0
);

CREATE INDEX idx_combatant_target ON wyldlands.entity_combatant (target_id);

COMMENT ON TABLE wyldlands.entity_combatant IS 'Combatant component - combat state';
COMMENT ON COLUMN wyldlands.entity_combatant.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_combatant.in_combat IS 'Is Entity in Combat';
COMMENT ON COLUMN wyldlands.entity_combatant.target_id IS 'Target of Attack';
COMMENT ON COLUMN wyldlands.entity_combatant.initiative IS 'Current Initiative';
COMMENT ON COLUMN wyldlands.entity_combatant.action_cooldown IS 'Cooldown till next action';
COMMENT ON COLUMN wyldlands.entity_combatant.time_since_action IS 'Time since last action';

--
-- Name: entity_equipment; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity which can wear equipment, All Slots created on creation. Missing slots cannot be equipped.
--

CREATE TABLE wyldlands.entity_equipment
(
    entity_id   UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    slot        slot_kind   NOT NULL,
    item_id     UUID        REFERENCES wyldlands.entities (uuid) ON DELETE SET NULL,
    equipped_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, slot)
);

CREATE INDEX idx_equipment_entity ON wyldlands.entity_equipment (entity_id);

COMMENT ON TABLE wyldlands.entity_equipment IS 'Equipment component - worn/wielded items';
COMMENT ON COLUMN wyldlands.entity_equipment.entity_id IS 'Entity ID of entity with Equipment';
COMMENT ON COLUMN wyldlands.entity_equipment.slot IS 'Slot on Entity';
COMMENT ON COLUMN wyldlands.entity_equipment.item_id IS 'Item being Equipped';
COMMENT ON COLUMN wyldlands.entity_equipment.equipped_at IS 'When Item was Equipped';

--
-- Name: entity_equipable; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity which can be equipped
--

CREATE TABLE wyldlands.entity_equipable
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    slots     slot_kind[] NOT NULL
);

COMMENT ON TABLE wyldlands.entity_equipable IS 'Equippable Item';
COMMENT ON COLUMN wyldlands.entity_equipable.entity_id IS 'Entity ID of Equippable Item';
COMMENT ON COLUMN wyldlands.entity_equipable.slots IS 'What slots is this item valid for';

--
-- Name: entity_weapon; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity which can deal damage when equipped
--

CREATE TABLE wyldlands.entity_weapon
(
    entity_id    UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    damage_min   INTEGER     NOT NULL,
    damage_max   INTEGER     NOT NULL,
    damage_cap   INTEGER     NOT NULL,
    damage_type  damage_type NOT NULL,
    attack_speed REAL        NOT NULL DEFAULT 1.0,
    range        REAL        NOT NULL DEFAULT 1.0
);

COMMENT ON TABLE wyldlands.entity_weapon IS 'Weapon component - weapon properties';
COMMENT ON COLUMN wyldlands.entity_weapon.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_weapon.damage_min IS 'Damage Range Minimum Value';
COMMENT ON COLUMN wyldlands.entity_weapon.damage_max IS 'Damage Range Maximum Value';
COMMENT ON COLUMN wyldlands.entity_weapon.damage_cap IS 'Damage Maximum after Modifiers';
COMMENT ON COLUMN wyldlands.entity_weapon.damage_type IS 'Type of Damage done';
COMMENT ON COLUMN wyldlands.entity_weapon.attack_speed IS 'Attack Speed Modifier';
COMMENT ON COLUMN wyldlands.entity_weapon.range IS 'Range in Rooms of weapon';

--
-- Name: entity_material; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Made of one or more materials
-- TODO: Create Materials Table
--

CREATE TABLE wyldlands.entity_material
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    material  VARCHAR(20) NOT NULL
);

COMMENT ON TABLE wyldlands.entity_material IS 'Material Item is made from';
COMMENT ON COLUMN wyldlands.entity_material.entity_id IS 'Entity ID of Item';
COMMENT ON COLUMN wyldlands.entity_material.material IS 'Kind of Material Used';

--
-- Name: entity_armor_defense; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity which can defend against damage of a kind
--

CREATE TABLE wyldlands.entity_armor_defense
(
    entity_id   UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    damage_kind damage_type NOT NULL,
    defense     INTEGER     NOT NULL
);

COMMENT ON TABLE wyldlands.entity_armor_defense IS 'Armor component - armor properties';
COMMENT ON COLUMN wyldlands.entity_armor_defense.entity_id IS 'Entity ID of Item';
COMMENT ON COLUMN wyldlands.entity_armor_defense.damage_kind IS 'Kind of Damage Defended Against';
COMMENT ON COLUMN wyldlands.entity_armor_defense.defense IS 'Defense Rating for this kind of Damage';

-- AI Components

--
-- Name: entity_ai_controller; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity which has a Goal Oriented Action Planning based AI
--

CREATE TABLE wyldlands.entity_ai_controller
(
    entity_id         UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    behavior_type     ai_behavior NOT NULL,
    current_goal      TEXT,
    state_type        ai_state    NOT NULL,
    state_target_id   UUID        REFERENCES wyldlands.entities (uuid) ON DELETE SET NULL,
    update_interval   REAL        NOT NULL DEFAULT 1.0,
    time_since_update REAL        NOT NULL DEFAULT 0.0
);

COMMENT ON TABLE wyldlands.entity_ai_controller IS 'AI Controller component - NPC behavior';
COMMENT ON COLUMN wyldlands.entity_ai_controller.entity_id IS 'Entity ID of Item';
COMMENT ON COLUMN wyldlands.entity_ai_controller.behavior_type IS 'Current Behavior Type';
COMMENT ON COLUMN wyldlands.entity_ai_controller.current_goal IS 'Description of Current Goal';
COMMENT ON COLUMN wyldlands.entity_ai_controller.state_type IS 'Current State';
COMMENT ON COLUMN wyldlands.entity_ai_controller.state_target_id IS 'Target of State';
COMMENT ON COLUMN wyldlands.entity_ai_controller.update_interval IS 'How often AI updates';
COMMENT ON COLUMN wyldlands.entity_ai_controller.time_since_update IS 'When was goal last updated';

--
-- Name: entity_personality; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Personality Profile information
--

CREATE TABLE wyldlands.entity_personality
(
    entity_id      UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    background     TEXT        NOT NULL DEFAULT '',
    speaking_style VARCHAR(50) NOT NULL DEFAULT 'neutral'
);

COMMENT ON TABLE wyldlands.entity_personality IS 'Personality component - NPC personality for LLM';
COMMENT ON COLUMN wyldlands.entity_personality.entity_id IS 'Entity ID ';
COMMENT ON COLUMN wyldlands.entity_personality.background IS 'Character Background Description';
COMMENT ON COLUMN wyldlands.entity_personality.speaking_style IS 'Speaking Style';

--
-- Name: entity_personality_bigfive; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity BigFive Personality Profile information
--

CREATE TABLE wyldlands.entity_personality_bigfive
(
    entity_id            UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,

    neuroticism          INT NOT NULL DEFAULT 60,
    anxiety              INT NOT NULL DEFAULT 10,
    anger                INT NOT NULL DEFAULT 10,
    depression           INT NOT NULL DEFAULT 10,
    self_consciousness   INT NOT NULL DEFAULT 10,
    immoderation         INT NOT NULL DEFAULT 10,
    vulnerability        INT NOT NULL DEFAULT 10,

    extroversion         INT NOT NULL DEFAULT 60,
    friendliness         INT NOT NULL DEFAULT 10,
    gregariousness       INT NOT NULL DEFAULT 10,
    assertiveness        INT NOT NULL DEFAULT 10,
    activity_level       INT NOT NULL DEFAULT 10,
    excitement_seeking   INT NOT NULL DEFAULT 10,
    cheerfulness         INT NOT NULL DEFAULT 10,

    openness             INT NOT NULL DEFAULT 60,
    imagination          INT NOT NULL DEFAULT 10,
    artistic_interest    INT NOT NULL DEFAULT 10,
    emotionality         INT NOT NULL DEFAULT 10,
    adventurousness      INT NOT NULL DEFAULT 10,
    intellect            INT NOT NULL DEFAULT 10,
    liberalism           INT NOT NULL DEFAULT 10,

    agreeableness        INT NOT NULL DEFAULT 60,
    trust                INT NOT NULL DEFAULT 10,
    morality             INT NOT NULL DEFAULT 10,
    altruism             INT NOT NULL DEFAULT 10,
    cooperation          INT NOT NULL DEFAULT 10,
    modesty              INT NOT NULL DEFAULT 10,
    sympathy             INT NOT NULL DEFAULT 10,

    conscientiousness    INT NOT NULL DEFAULT 60,
    self_efficacy        INT NOT NULL DEFAULT 10,
    orderliness          INT NOT NULL DEFAULT 10,
    dutifulness          INT NOT NULL DEFAULT 10,
    achievement_striving INT NOT NULL DEFAULT 10,
    self_discipline      INT NOT NULL DEFAULT 10,
    cautiousness         INT NOT NULL DEFAULT 10
);

COMMENT ON TABLE wyldlands.entity_personality_bigfive IS 'Personality component - NPC BigFive Personality Profile for LLM';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.entity_id IS 'Entity ID';

COMMENT ON COLUMN wyldlands.entity_personality_bigfive.neuroticism IS 'Overall emotional stability (0-120)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.anxiety IS 'Tendency to worry and feel anxious (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.anger IS 'Tendency to experience anger (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.depression IS 'Tendency toward sadness and hopelessness (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.self_consciousness IS 'Sensitivity to social judgment (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.immoderation IS 'Difficulty resisting temptation (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.vulnerability IS 'Susceptibility to stress (0-20)';

COMMENT ON COLUMN wyldlands.entity_personality_bigfive.extroversion IS 'Overall sociability and energy (0-120)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.friendliness IS 'Warmth toward others (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.gregariousness IS 'Preference for company (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.assertiveness IS 'Leadership and confidence (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.activity_level IS 'Energy and pace (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.excitement_seeking IS 'Desire for stimulation (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.cheerfulness IS 'Positive emotions (0-20)';

COMMENT ON COLUMN wyldlands.entity_personality_bigfive.openness IS 'Overall openness to experience (0-120)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.imagination IS 'Fantasy and creativity (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.artistic_interest IS 'Appreciation for art and beauty (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.emotionality IS 'Depth of emotional experience (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.adventurousness IS 'Willingness to try new things (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.intellect IS 'Intellectual curiosity (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.liberalism IS 'Openness to change and new ideas (0-20)';

COMMENT ON COLUMN wyldlands.entity_personality_bigfive.agreeableness IS 'Overall cooperativeness (0-120)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.trust IS 'Belief in others goodwill (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.morality IS 'Honesty and sincerity (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.altruism IS 'Concern for others welfare (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.cooperation IS 'Preference for harmony (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.modesty IS 'Humility (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.sympathy IS 'Compassion for others (0-20)';

COMMENT ON COLUMN wyldlands.entity_personality_bigfive.conscientiousness IS 'Overall self-discipline and organization (0-120)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.self_efficacy IS 'Confidence in ability (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.orderliness IS 'Preference for organization (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.dutifulness IS 'Sense of duty and obligation (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.achievement_striving IS 'Drive to succeed (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.self_discipline IS 'Ability to follow through (0-20)';
COMMENT ON COLUMN wyldlands.entity_personality_bigfive.cautiousness IS 'Tendency to think before acting (0-20)';

--
-- Name: entity_personality_traits; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Personality Trait Information
--

CREATE TABLE wyldlands.entity_personality_traits
(
    entity_id  UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    trait_name VARCHAR(50) NOT NULL,
    value      REAL        NOT NULL,
    PRIMARY KEY (entity_id, trait_name),
    CONSTRAINT valid_trait_value CHECK (-1.0 <= value AND value <= 1.0)
);

COMMENT ON TABLE wyldlands.entity_personality_traits IS 'Personality traits (key-value pairs)';
COMMENT ON COLUMN wyldlands.entity_personality_traits.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_personality_traits.trait_name IS 'Personality Trait Name';
COMMENT ON COLUMN wyldlands.entity_personality_traits.value IS 'Personality Trait Value (-1.0 - 1.0)';

--
-- Name: entity_personality_goals; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Personality Goal Information
--

CREATE TABLE wyldlands.entity_personality_goals
(
    entity_id  UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    goal       TEXT        NOT NULL,
    priority   INTEGER     NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_personality_goals_entity ON wyldlands.entity_personality_goals (entity_id);

COMMENT ON TABLE wyldlands.entity_personality_goals IS 'Personality goals';
COMMENT ON COLUMN wyldlands.entity_personality_goals.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_personality_goals.goal IS 'Goal Description';
COMMENT ON COLUMN wyldlands.entity_personality_goals.priority IS 'Goal Priority (higher is more important)';
COMMENT ON COLUMN wyldlands.entity_personality_goals.created_at IS 'When goal was created';

--
-- Name: entity_memory; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Memory
--

CREATE TABLE wyldlands.entity_memory
(
    entity_id    UUID    NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    memory_id    SERIAL,
    timestamp    BIGINT  NOT NULL,
    event        TEXT    NOT NULL,
    importance   REAL    NOT NULL,
    is_long_term BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (entity_id, memory_id)
);

CREATE INDEX idx_memory_entity ON wyldlands.entity_memory (entity_id);
CREATE INDEX idx_memory_importance ON wyldlands.entity_memory (entity_id, importance DESC);

COMMENT ON TABLE wyldlands.entity_memory IS 'Memory component - NPC memories';
COMMENT ON COLUMN wyldlands.entity_memory.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_memory.memory_id IS 'Sequential Memory ID';
COMMENT ON COLUMN wyldlands.entity_memory.timestamp IS 'When memory was created';
COMMENT ON COLUMN wyldlands.entity_memory.event IS 'Description of the event';
COMMENT ON COLUMN wyldlands.entity_memory.importance IS 'Importance of memory (0.0 - 1.0)';
COMMENT ON COLUMN wyldlands.entity_memory.is_long_term IS 'Is this a long-term memory';

--
-- Name: entity_memory_entities; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Memory Entities
--

CREATE TABLE wyldlands.entity_memory_entities
(
    entity_id          UUID    NOT NULL,
    memory_id          INTEGER NOT NULL,
    involved_entity_id UUID    NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    FOREIGN KEY (entity_id, memory_id) REFERENCES wyldlands.entity_memory (entity_id, memory_id) ON DELETE CASCADE
);

CREATE INDEX idx_memory_entities_memory ON wyldlands.entity_memory_entities (entity_id, memory_id);

COMMENT ON TABLE wyldlands.entity_memory_entities IS 'Entities involved in memories';
COMMENT ON COLUMN wyldlands.entity_memory_entities.entity_id IS 'Entity ID that has the memory';
COMMENT ON COLUMN wyldlands.entity_memory_entities.memory_id IS 'Memory ID';
COMMENT ON COLUMN wyldlands.entity_memory_entities.involved_entity_id IS 'Entity involved in the memory';

-- Interaction Components

--
-- Name: entity_commandable; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity Can Queue Commands
--

CREATE TABLE wyldlands.entity_commandable
(
    entity_id      UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    max_queue_size INTEGER NOT NULL DEFAULT 10
);

COMMENT ON TABLE wyldlands.entity_commandable IS 'Commandable component - can receive commands';
COMMENT ON COLUMN wyldlands.entity_commandable.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_commandable.max_queue_size IS 'Maximum number of queued commands';

--
-- Name: entity_command_queue; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Commands Entity Has Queued
--

CREATE TABLE wyldlands.entity_command_queue
(
    entity_id      UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    queue_position SERIAL,
    command        TEXT        NOT NULL,
    args           TEXT[]      NOT NULL DEFAULT '{}',
    priority       SMALLINT    NOT NULL DEFAULT 0,
    queued_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, queue_position)
);

CREATE INDEX idx_command_queue_entity ON wyldlands.entity_command_queue (entity_id);

COMMENT ON TABLE wyldlands.entity_command_queue IS 'Command queue for commandable entities';
COMMENT ON COLUMN wyldlands.entity_command_queue.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_command_queue.queue_position IS 'Position in queue';
COMMENT ON COLUMN wyldlands.entity_command_queue.command IS 'Command text';
COMMENT ON COLUMN wyldlands.entity_command_queue.args IS 'Command arguments';
COMMENT ON COLUMN wyldlands.entity_command_queue.priority IS 'Command priority';
COMMENT ON COLUMN wyldlands.entity_command_queue.queued_at IS 'When command was queued';

--
-- Name: entity_commandable; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Entity is Interactable
--

CREATE TABLE wyldlands.entity_interactable
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE
);

COMMENT ON TABLE wyldlands.entity_interactable IS 'Interactable component - can be interacted with';
COMMENT ON COLUMN wyldlands.entity_interactable.entity_id IS 'Entity ID';

--
-- Name: entity_interactions; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Interactions that can occur with an entity
--

CREATE TABLE wyldlands.entity_interactions
(
    entity_id     UUID        NOT NULL REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE,
    verb          VARCHAR(50) NOT NULL,
    description   TEXT        NOT NULL,
    requires_item VARCHAR(100),
    PRIMARY KEY (entity_id, verb)
);

CREATE INDEX idx_interactions_entity ON wyldlands.entity_interactions (entity_id);

COMMENT ON TABLE wyldlands.entity_interactions IS 'Available interactions for interactable entities';
COMMENT ON COLUMN wyldlands.entity_interactions.entity_id IS 'Entity ID';
COMMENT ON COLUMN wyldlands.entity_interactions.verb IS 'Interaction verb (e.g., pull, push, read)';
COMMENT ON COLUMN wyldlands.entity_interactions.description IS 'Description of what happens';
COMMENT ON COLUMN wyldlands.entity_interactions.requires_item IS 'Item required to perform interaction';

-- Persistence Markers

--
-- Name: entity_persistent; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Interactions that can occur with an entity
--

CREATE TABLE wyldlands.entity_persistent
(
    entity_id UUID PRIMARY KEY REFERENCES wyldlands.entities (uuid) ON DELETE CASCADE
);

COMMENT ON TABLE wyldlands.entity_persistent IS 'Persistent marker - entity should be saved to database';
COMMENT ON COLUMN wyldlands.entity_persistent.entity_id IS 'Entity ID';

--
-- Name: starting_locations; Type: TABLE; Schema: wyldlands; Owner: wyldlands
--

CREATE TABLE wyldlands.starting_locations
(
    id          VARCHAR(50) PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    room_id     UUID NOT NULL,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_starting_location_room
        FOREIGN KEY (room_id) REFERENCES wyldlands.entities(uuid)
            ON DELETE RESTRICT
);

COMMENT ON TABLE wyldlands.starting_locations IS 'Available starting locations for new characters';
COMMENT ON COLUMN wyldlands.starting_locations.id IS 'Unique identifier for the starting location';
COMMENT ON COLUMN wyldlands.starting_locations.name IS 'Display name of the starting location';
COMMENT ON COLUMN wyldlands.starting_locations.description IS 'Description shown to players during selection';
COMMENT ON COLUMN wyldlands.starting_locations.room_id IS 'UUID of the room entity where characters spawn';
COMMENT ON COLUMN wyldlands.starting_locations.enabled IS 'Whether this location is currently available for selection';
COMMENT ON COLUMN wyldlands.starting_locations.sort_order IS 'Display order (lower numbers first)';

--
-- Name: sessions; Type: TABLE; Schema: wyldlands; Owner: wyldlands
-- Matches: common/src/session.rs::Session
--

CREATE TABLE wyldlands.sessions
(
    id            UUID PRIMARY KEY,
    entity_id     UUID,
    created_at    TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    state         session_state    NOT NULL,
    protocol      session_protocol NOT NULL,
    client_addr   VARCHAR(100)     NOT NULL,
    metadata      JSONB            NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX idx_sessions_entity_id ON wyldlands.sessions (entity_id);
CREATE INDEX idx_sessions_state ON wyldlands.sessions (state);
CREATE INDEX idx_sessions_last_activity ON wyldlands.sessions (last_activity);

COMMENT ON TABLE wyldlands.sessions IS 'Active and historical client sessions';
COMMENT ON COLUMN wyldlands.sessions.id IS 'Unique session identifier';
COMMENT ON COLUMN wyldlands.sessions.entity_id IS 'Associated player entity ID';
COMMENT ON COLUMN wyldlands.sessions.created_at IS 'Session creation timestamp';
COMMENT ON COLUMN wyldlands.sessions.last_activity IS 'Last activity timestamp';
COMMENT ON COLUMN wyldlands.sessions.state IS 'Current session state';
COMMENT ON COLUMN wyldlands.sessions.protocol IS 'Connection protocol (Telnet/WebSocket)';
COMMENT ON COLUMN wyldlands.sessions.client_addr IS 'Client IP address and port';
COMMENT ON COLUMN wyldlands.sessions.metadata IS 'Session metadata (terminal type, window size, etc.)';

--
-- Name: session_command_queue; Type: TABLE; Schema: wyldlands; Owner: wyldlands
--

CREATE TABLE wyldlands.session_command_queue
(
    id         SERIAL PRIMARY KEY,
    session_id UUID        NOT NULL REFERENCES wyldlands.sessions (id) ON DELETE CASCADE,
    command    TEXT        NOT NULL,
    queued_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_command_queue_session ON wyldlands.session_command_queue (session_id);

COMMENT ON TABLE wyldlands.session_command_queue IS 'Queued commands for disconnected sessions';
COMMENT ON COLUMN wyldlands.session_command_queue.id IS 'Queue entry ID';
COMMENT ON COLUMN wyldlands.session_command_queue.session_id IS 'Associated session ID';
COMMENT ON COLUMN wyldlands.session_command_queue.command IS 'Queued command text';
COMMENT ON COLUMN wyldlands.session_command_queue.queued_at IS 'Time command was queued';

COMMIT;

-- Made with Bob
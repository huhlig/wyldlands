# Wyldlands MUD Development Plan

**Last Updated**: January 1, 2026
**Status**: Phase 2 Complete, Phase 3 Ready to Start

## Executive Summary

This document outlines the comprehensive development plan for Wyldlands MUD, a modern multi-user dungeon with ECS architecture, dual-protocol access (Telnet/WebSocket), connection persistence via proxy gateway, and AI-driven NPCs using GOAP and LLMs.

**Completed**: Phases 1-2 (40% of project)
**Remaining**: Phases 3-5 (60% of project)

## Current State (January 2026)

### Completed Infrastructure ✅
- ✅ **Phase 1 Complete**: Full ECS implementation with 25+ components, 5 systems, event bus
- ✅ **Phase 2 Complete**: Gateway with session management, connection pool, protocols, reconnection, RPC
- ✅ **Workspace Structure**: Multi-crate Rust workspace (gateway, common, server, world)
- ✅ **Gateway**: Full Axum-based HTTP/WebSocket/Telnet server
- ✅ **ECS Foundation**: Complete Hecs ECS with comprehensive component library
- ✅ **Protocol Layer**: Bidirectional tarpc-based RPC protocol
- ✅ **Database**: PostgreSQL with full schema and persistence
- ✅ **Session Management**: 6-state machine with database persistence
- ✅ **Connection Persistence**: Token-based reconnection with command replay
- ✅ **Authentication**: Account creation, character management
- ✅ **Docker Deployment**: Complete containerized infrastructure

### Remaining Work 📋
- 📋 **AI System**: GOAP planner and action library (Phase 3)
- 📋 **LLM Integration**: Provider implementations and hybrid AI (Phase 4)
- 📋 **Gameplay Systems**: Complete combat, items, quests (Phase 5)
- 📋 **Admin Tools**: World building and management tools (Phase 5)
- 📋 **Performance**: Load testing and optimization (Phase 5)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Clients                              │
│  ┌──────────────┐              ┌──────────────┐            │
│  │ Telnet Client│              │ Web Browser  │            │
│  └──────┬───────┘              └──────┬───────┘            │
└─────────┼──────────────────────────────┼──────────────────┘
          │                              │
          │ TCP:4000                     │ WS:8080
          │                              │
┌─────────▼──────────────────────────────▼──────────────────┐
│              Connection Gateway (Proxy)                    │
│  ┌──────────────────────────────────────────────────────┐ │
│  │ • Telnet Protocol Handler                            │ │
│  │ • WebSocket Handler                                  │ │
│  │ • Session Management & Persistence                   │ │
│  │ • Connection Pooling                                 │ │
│  │ • Protocol Translation                               │ │
│  └──────────────────────────────────────────────────────┘ │
└─────────┬──────────────────────────────────────────────────┘
          │ RPC (tarpc)
          │
┌─────────▼──────────────────────────────────────────────────┐
│                    World Server (ECS)                       │
│  ┌──────────────────────────────────────────────────────┐ │
│  │ ECS World (Hecs)                                     │ │
│  │  • Entities: Players, NPCs, Items, Rooms             │ │
│  │  • Components: Position, Inventory, Stats, AI, etc.  │ │
│  │  • Systems: Movement, Combat, AI, Commands           │ │
│  └──────────────────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────┐ │
│  │ AI Engine                                            │ │
│  │  • GOAP Planner (Goal-Oriented Action Planning)     │ │
│  │  • LLM Integration (OpenAI/Anthropic/Local)         │ │
│  │  • Behavior Trees                                    │ │
│  │  • Personality System                                │ │
│  └──────────────────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────┐ │
│  │ World State                                          │ │
│  │  • Area/Room Management                              │ │
│  │  • Event System                                      │ │
│  │  • Persistence Layer (PostgreSQL)                    │ │
│  └──────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Phase 1: Core ECS Implementation ✅ COMPLETE

**Duration**: Weeks 1-3 (3 weeks)  
**Status**: ✅ 100% Complete  
**Completion Date**: December 18, 2025

### Summary
Successfully implemented comprehensive ECS foundation with 25+ components, 5 complete systems, event bus, and 94% test coverage.

**Details**: See [archive/phase1/PHASE1_COMPLETE.md](archive/phase1/PHASE1_COMPLETE.md)

---

## Phase 1 Original Plan (Weeks 1-3) - ARCHIVED

### 1.1 ECS Component System
**Priority: Critical**

#### Components to Implement
```rust
// Core Identity
- EntityId (UUID-based unique identifier)
- Name (display name, keywords)
- Description (short, long, dynamic)

// Spatial
- Position (area_id, room_id, coordinates)
- Container (inventory, capacity, weight limits)
- Containable (size, weight, stackable)

// Character Stats
- Attributes (strength, dexterity, intelligence, etc.)
- Health (current, max, regeneration)
- Mana (current, max, regeneration)
- Experience (level, xp, progression)

// Interaction
- Commandable (can receive commands)
- Interactable (can be examined, used)
- Enterable (can be entered - rooms, vehicles)
- Lockable (locked state, key requirements)

// AI & Behavior
- AIController (behavior type, goals, state)
- Personality (traits for LLM context)
- Memory (recent events, relationships)
- Schedule (time-based behaviors)

// Combat
- Combatant (in combat, target, initiative)
- Equipment (worn items, weapon slots)
- Skills (abilities, cooldowns)

// Persistence
- Persistent (should be saved to DB)
- Dirty (needs database update)
```

#### Systems to Implement
```rust
// Core Systems
- CommandSystem (process player/NPC commands)
- MovementSystem (handle entity movement)
- CombatSystem (resolve combat actions)
- InventorySystem (item management)
- DescriptionSystem (generate dynamic descriptions)

// AI Systems (Phase 3)
- GOAPSystem (goal planning)
- LLMSystem (natural language processing)
- BehaviorSystem (execute behaviors)

// Maintenance Systems
- PersistenceSystem (save/load entities)
- CleanupSystem (remove dead entities)
- RegenerationSystem (health/mana regen)
```

### 1.2 World Structure
**Priority: Critical**

```rust
// Expand existing world module
- Area (collection of rooms, theme, level range)
- Room (description, exits, entities, flags)
- Exit (direction, destination, door state)
- Zone (multiple areas, shared properties)
```

### 1.3 Event System
**Priority: High**

```rust
// Event-driven architecture for loose coupling
pub enum GameEvent {
    EntityMoved { entity: EntityId, from: RoomId, to: RoomId },
    EntitySpawned { entity: EntityId, location: RoomId },
    EntityDied { entity: EntityId, killer: Option<EntityId> },
    CommandExecuted { entity: EntityId, command: String },
    CombatStarted { attacker: EntityId, defender: EntityId },
    ItemPickedUp { entity: EntityId, item: EntityId },
    // ... more events
}

// Event bus for system communication
pub struct EventBus {
    subscribers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}
```

**Deliverables:**
- [ ] Complete ECS component library
- [ ] Core system implementations
- [ ] Event system with pub/sub
- [ ] Unit tests for all components
- [ ] Integration tests for systems

---

## Phase 2: Gateway & Connection Persistence ✅ COMPLETE

**Duration**: Weeks 4-6 (3 weeks, completed in 12 days)  
**Status**: ✅ 100% Complete  
**Completion Date**: December 19, 2025

### Summary
Successfully implemented comprehensive gateway infrastructure with session management, connection pooling, protocol adapters, reconnection system, and bidirectional RPC.

**Details**: See [archive/phase2/PHASE2_COMPLETE.md](archive/phase2/PHASE2_COMPLETE.md)

---

## Phase 2 Original Plan (Weeks 4-6) - ARCHIVED

### 2.1 Session Management
**Priority: Critical**

```rust
pub struct Session {
    id: Uuid,
    entity_id: Option<EntityId>,  // Associated player entity
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    state: SessionState,
    protocol: ProtocolType,
}

pub enum SessionState {
    Connecting,
    Authenticating,
    CharacterSelection,
    Playing,
    Disconnected,
}

pub enum ProtocolType {
    Telnet,
    WebSocket,
}
```

### 2.2 Connection Pooling
**Priority: High**

```rust
pub struct ConnectionPool {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    world_client: WorldClient,  // RPC client to world server
}

pub enum Connection {
    Telnet(TelnetConnection),
    WebSocket(WebSocketConnection),
}
```

### 2.3 Telnet Implementation
**Priority: High**

**Dependencies to Add:**
```toml
# In gateway/Cargo.toml
libtelnet-rs = "0.2"  # or nectar if preferred
```

**Implementation:**
```rust
// gateway/src/telnet.rs
pub struct TelnetServer {
    listener: TcpListener,
    pool: Arc<ConnectionPool>,
}

pub struct TelnetConnection {
    stream: TcpStream,
    parser: TelnetParser,
    session_id: Uuid,
}

// Features to implement:
- MCCP (compression)
- MSDP (Mud Server Data Protocol) - already started
- GMCP (Generic Mud Communication Protocol)
- NAWS (window size negotiation)
- Color support (ANSI)
```

### 2.4 WebSocket Enhancement
**Priority: Medium**

```rust
// Enhance existing websocket.rs
- Binary message support
- Compression (permessage-deflate)
- Heartbeat/keepalive
- Reconnection handling
- Message queuing during disconnects
```

### 2.5 Protocol Translation Layer
**Priority: High**

```rust
pub trait ProtocolAdapter {
    fn encode_output(&self, message: &GameMessage) -> Vec<u8>;
    fn decode_input(&self, data: &[u8]) -> Result<GameCommand>;
}

pub struct TelnetAdapter {
    // ANSI color codes, telnet negotiation
}

pub struct WebSocketAdapter {
    // JSON or binary encoding
}
```

### 2.6 Persistence Strategy
**Priority: Critical**

```rust
// Session persistence across server restarts
pub struct SessionStore {
    db: PgPool,
}

impl SessionStore {
    async fn save_session(&self, session: &Session) -> Result<()>;
    async fn restore_session(&self, id: Uuid) -> Result<Session>;
    async fn cleanup_expired(&self) -> Result<()>;
}

// On world server restart:
// 1. Gateway maintains connections
// 2. Gateway queues commands
// 3. World server reconnects
// 4. Gateway replays queued commands
// 5. Seamless experience for players
```

**Deliverables:**
- [ ] Session management system
- [ ] Full telnet protocol support
- [ ] Enhanced WebSocket handler
- [ ] Protocol translation layer
- [ ] Connection persistence mechanism
- [ ] Reconnection handling
- [ ] Integration tests for both protocols

---

## Phase 3: GOAP AI System 📋 READY TO START

**Duration**: Weeks 7-9 (3 weeks, 15 working days)  
**Status**: 📋 Not Started  
**Prerequisites**: ✅ Phase 1 Complete, ✅ Phase 2 Complete

### Overview
Implement Goal-Oriented Action Planning (GOAP) system to enable intelligent NPC behavior. NPCs will autonomously select and execute actions based on their goals and the current world state.

---

## Phase 3 Detailed Plan (Weeks 7-9)

### 3.1 GOAP Architecture
**Priority: High**

```rust
// server/src/ai/goap/mod.rs

pub struct GOAPPlanner {
    actions: Vec<Box<dyn GOAPAction>>,
    max_depth: usize,
}

pub trait GOAPAction: Send + Sync {
    fn name(&self) -> &str;
    fn cost(&self) -> f32;
    fn preconditions(&self) -> &WorldState;
    fn effects(&self) -> &WorldState;
    fn can_execute(&self, world: &World, entity: Entity) -> bool;
    fn execute(&self, world: &mut World, entity: Entity) -> ActionResult;
}

pub struct WorldState {
    conditions: HashMap<String, bool>,
}

pub struct Goal {
    desired_state: WorldState,
    priority: f32,
}
```

### 3.2 Core GOAP Actions
**Priority: High**

```rust
// Implement basic NPC actions
pub struct WanderAction;      // Move randomly
pub struct SeekPlayerAction;  // Move toward player
pub struct FleeAction;        // Run away from threat
pub struct AttackAction;      // Engage in combat
pub struct HealAction;        // Use healing item
pub struct GuardAction;       // Stay in area
pub struct PatrolAction;      // Follow patrol route
pub struct FollowAction;      // Follow another entity
pub struct PickupItemAction;  // Collect items
pub struct UseItemAction;     // Use item from inventory
```

### 3.3 Goal System
**Priority: Medium**

```rust
pub struct GoalManager {
    goals: Vec<Goal>,
}

impl GoalManager {
    pub fn add_goal(&mut self, goal: Goal);
    pub fn get_highest_priority(&self) -> Option<&Goal>;
    pub fn update_priorities(&mut self, world: &World, entity: Entity);
}

// Example goals:
- StayAlive (high priority when health low)
- DefendTerritory (medium priority for guards)
- CollectResources (low priority for gatherers)
- SocialInteraction (variable priority)
```

### 3.4 A* Pathfinding
**Priority: High**

```rust
// server/src/ai/pathfinding.rs
pub struct Pathfinder {
    cache: HashMap<(RoomId, RoomId), Vec<RoomId>>,
}

impl Pathfinder {
    pub fn find_path(&mut self, from: RoomId, to: RoomId, world: &World) -> Option<Vec<RoomId>>;
    pub fn clear_cache(&mut self);
}
```

### 3.5 Behavior Trees (Optional Enhancement)
**Priority: Low**

```rust
// For more complex behaviors
pub enum BehaviorNode {
    Sequence(Vec<BehaviorNode>),
    Selector(Vec<BehaviorNode>),
    Action(Box<dyn BehaviorAction>),
    Condition(Box<dyn BehaviorCondition>),
}
```

**Deliverables:**
- [ ] GOAP planner implementation
- [ ] Core action library (10+ actions)
- [ ] Goal management system
- [ ] A* pathfinding
- [ ] Integration with ECS
- [ ] Performance benchmarks
- [ ] AI behavior tests

---

## Phase 4: LLM Integration 📋 PLANNED

**Duration**: Weeks 10-12 (3 weeks, 15 working days)  
**Status**: 📋 Not Started  
**Prerequisites**: ✅ Phase 1 Complete, 📋 Phase 3 Complete (for hybrid AI)

### Overview
Integrate Large Language Models for dynamic NPC dialogue and behavior. Combine with GOAP for hybrid AI system.

---

## Phase 4 Detailed Plan (Weeks 10-12)

### 4.1 LLM Service Layer
**Priority: High**

```rust
// server/src/ai/llm/mod.rs

pub trait LLMProvider: Send + Sync {
    async fn generate(&self, prompt: &str, context: &LLMContext) -> Result<String>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

pub struct LocalLLMProvider {
    // For llama.cpp, ollama, etc.
    endpoint: String,
}
```

### 4.2 Context Management
**Priority: Critical**

```rust
pub struct LLMContext {
    pub character_name: String,
    pub personality: Personality,
    pub recent_events: Vec<String>,
    pub current_location: String,
    pub nearby_entities: Vec<String>,
    pub conversation_history: Vec<Message>,
    pub world_knowledge: String,
}

pub struct Personality {
    pub traits: HashMap<String, f32>,  // e.g., "friendly": 0.8
    pub background: String,
    pub goals: Vec<String>,
    pub speaking_style: String,
}
```

### 4.3 Prompt Engineering
**Priority: High**

```rust
pub struct PromptBuilder {
    templates: HashMap<String, String>,
}

impl PromptBuilder {
    pub fn build_dialogue_prompt(&self, context: &LLMContext, input: &str) -> String;
    pub fn build_action_prompt(&self, context: &LLMContext, situation: &str) -> String;
    pub fn build_description_prompt(&self, entity: &EntityDescription) -> String;
}

// Example templates:
const DIALOGUE_TEMPLATE: &str = r#"
You are {character_name}, a {personality} character in a fantasy world.
Background: {background}
Current situation: {situation}
Recent events: {recent_events}

A player says: "{player_input}"

Respond in character, keeping your response under 200 words.
Response:
"#;
```

### 4.4 Response Processing
**Priority: High**

```rust
pub struct ResponseProcessor {
    parser: ResponseParser,
    validator: ResponseValidator,
}

impl ResponseProcessor {
    pub fn process(&self, raw_response: String) -> ProcessedResponse;
    pub fn extract_actions(&self, response: &str) -> Vec<GameAction>;
    pub fn sanitize(&self, response: &str) -> String;
}

pub struct ProcessedResponse {
    pub text: String,
    pub actions: Vec<GameAction>,
    pub emotions: Vec<Emotion>,
}
```

### 4.5 Caching & Rate Limiting
**Priority: High**

```rust
pub struct LLMCache {
    cache: Arc<RwLock<HashMap<String, CachedResponse>>>,
    ttl: Duration,
}

pub struct RateLimiter {
    requests_per_minute: u32,
    tokens_per_minute: u32,
    current_usage: Arc<RwLock<UsageStats>>,
}
```

### 4.6 Hybrid AI System
**Priority: Critical**

```rust
pub struct HybridAI {
    goap: GOAPPlanner,
    llm: Box<dyn LLMProvider>,
    mode: AIMode,
}

pub enum AIMode {
    GOAPOnly,           // Fast, deterministic
    LLMOnly,            // Slow, creative
    Hybrid,             // GOAP for actions, LLM for dialogue
    Adaptive,           // Switch based on situation
}

impl HybridAI {
    pub async fn decide_action(&mut self, world: &World, entity: Entity) -> Action {
        match self.mode {
            AIMode::GOAPOnly => self.goap.plan(world, entity),
            AIMode::LLMOnly => self.llm_decide(world, entity).await,
            AIMode::Hybrid => {
                // Use GOAP for movement/combat
                // Use LLM for dialogue/complex decisions
                if self.needs_dialogue(world, entity) {
                    self.llm_decide(world, entity).await
                } else {
                    self.goap.plan(world, entity)
                }
            }
            AIMode::Adaptive => self.adaptive_decide(world, entity).await,
        }
    }
}
```

### 4.7 Memory System
**Priority: Medium**

```rust
pub struct NPCMemory {
    short_term: VecDeque<MemoryEntry>,  // Last 10-20 events
    long_term: Vec<MemoryEntry>,         // Important events
    relationships: HashMap<EntityId, Relationship>,
}

pub struct MemoryEntry {
    timestamp: DateTime<Utc>,
    event: String,
    importance: f32,
    embedding: Option<Vec<f32>>,  // For semantic search
}

pub struct Relationship {
    entity_id: EntityId,
    affinity: f32,  // -1.0 to 1.0
    trust: f32,
    history: Vec<Interaction>,
}
```

**Deliverables:**
- [ ] LLM provider implementations (OpenAI, Anthropic, Local)
- [ ] Context management system
- [ ] Prompt templates and builder
- [ ] Response processing pipeline
- [ ] Caching and rate limiting
- [ ] Hybrid AI system
- [ ] Memory and relationship system
- [ ] Cost monitoring and optimization
- [ ] Integration tests with mock LLM

---

## Phase 5: Integration & Polish 📋 PLANNED

**Duration**: Weeks 13-15 (3 weeks, 15 working days)  
**Status**: 📋 Not Started  
**Prerequisites**: All previous phases complete

### Overview
Complete gameplay systems, polish user experience, optimize performance, and prepare for production deployment.

---

## Phase 5 Detailed Plan (Weeks 13-15)

### 5.1 Command System Enhancement
**Priority: High**

```rust
// Expand existing command system
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Command>>,
    aliases: HashMap<String, String>,
}

pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str];
    fn help(&self) -> &str;
    fn execute(&self, world: &mut World, entity: Entity, args: &[&str]) -> CommandResult;
    fn can_execute(&self, world: &World, entity: Entity) -> bool;
}

// Implement commands:
- Movement: north, south, east, west, up, down, go
- Interaction: look, examine, get, drop, give, use
- Communication: say, tell, emote, shout
- Combat: attack, flee, cast, defend
- Social: follow, group, trade
- Admin: teleport, spawn, modify
```

### 5.2 Combat System
**Priority: High**

```rust
pub struct CombatSystem {
    initiative_order: Vec<Entity>,
    current_round: u32,
}

impl CombatSystem {
    pub fn start_combat(&mut self, participants: Vec<Entity>);
    pub fn process_round(&mut self, world: &mut World);
    pub fn resolve_attack(&self, attacker: Entity, defender: Entity, world: &mut World) -> AttackResult;
    pub fn end_combat(&mut self, world: &mut World);
}

pub struct AttackResult {
    hit: bool,
    damage: i32,
    critical: bool,
    effects: Vec<CombatEffect>,
}
```

### 5.3 Item System
**Priority: Medium**

```rust
pub struct Item {
    // Use ECS components
}

pub enum ItemType {
    Weapon { damage: i32, speed: f32 },
    Armor { defense: i32, slot: EquipSlot },
    Consumable { effect: Effect },
    Container { capacity: u32 },
    Quest { quest_id: Uuid },
    Misc,
}

pub enum EquipSlot {
    Head, Chest, Legs, Feet, Hands,
    MainHand, OffHand, TwoHand,
    Ring, Neck, Back,
}
```

### 5.4 Quest System
**Priority: Low**

```rust
pub struct Quest {
    id: Uuid,
    name: String,
    description: String,
    objectives: Vec<Objective>,
    rewards: Vec<Reward>,
    prerequisites: Vec<Uuid>,
}

pub enum Objective {
    Kill { target: String, count: u32 },
    Collect { item: String, count: u32 },
    Deliver { item: String, npc: String },
    Explore { location: RoomId },
    Talk { npc: String },
}
```

### 5.5 Admin Tools
**Priority: Medium**

```rust
pub struct AdminConsole {
    // Web-based admin interface
}

// Features:
- Live world monitoring
- Entity inspection and modification
- Player management
- AI behavior debugging
- Performance metrics
- Log viewing
```

### 5.6 Testing & Documentation
**Priority: Critical**

```rust
// Comprehensive test suite
- Unit tests for all components
- Integration tests for systems
- End-to-end tests for gameplay
- Load testing for gateway
- AI behavior tests
- Protocol compliance tests

// Documentation
- API documentation (rustdoc)
- Architecture guide
- Deployment guide
- Admin manual
- Player guide
- AI configuration guide
```

**Deliverables:**
- [ ] Complete command system
- [ ] Combat system
- [ ] Item and equipment system
- [ ] Quest system (basic)
- [ ] Admin tools
- [ ] Comprehensive test suite
- [ ] Complete documentation
- [ ] Performance optimization
- [ ] Security audit

---

## Technology Stack

### Core Dependencies
```toml
[workspace.dependencies]
# Existing
hecs = "0.10"                    # ECS
axum = "0.8"                     # Web framework
tokio = "1"                      # Async runtime
sqlx = "0.8"                     # Database
tarpc = "0.37"                   # RPC

# To Add
libtelnet-rs = "0.2"             # Telnet protocol
async-openai = "0.24"            # OpenAI API
anthropic-sdk = "0.2"            # Anthropic API
reqwest = "0.12"                 # HTTP client
serde_json = "1"                 # JSON
dashmap = "6"                    # Concurrent HashMap
parking_lot = "0.12"             # Better locks
rayon = "1.10"                   # Parallel processing
pathfinding = "4"                # A* pathfinding
chrono = "0.4"                   # Date/time
uuid = "1"                       # UUIDs
thiserror = "1"                  # Error handling
anyhow = "1"                     # Error handling
tracing = "0.1"                  # Logging
tracing-subscriber = "0.3"       # Logging
metrics = "0.23"                 # Metrics
```

### Optional Dependencies
```toml
# For local LLM
llama-cpp-rs = "0.1"             # Local LLM inference
candle = "0.6"                   # ML framework

# For advanced features
redis = "0.24"                   # Caching
kafka = "0.9"                    # Event streaming
prometheus = "0.13"              # Metrics
```

---

## Database Schema

### Core Tables
```sql
-- Players
CREATE TABLE players (
    id UUID PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    last_login TIMESTAMP
);

-- Characters (player entities)
CREATE TABLE characters (
    id UUID PRIMARY KEY,
    player_id UUID REFERENCES players(id),
    name VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,  -- ECS component data
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

-- Sessions
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    player_id UUID REFERENCES players(id),
    character_id UUID REFERENCES characters(id),
    created_at TIMESTAMP NOT NULL,
    last_activity TIMESTAMP NOT NULL,
    state VARCHAR(50) NOT NULL,
    data JSONB
);

-- World entities (NPCs, items, rooms)
CREATE TABLE entities (
    id UUID PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

-- AI memory
CREATE TABLE npc_memories (
    id UUID PRIMARY KEY,
    npc_id UUID REFERENCES entities(id),
    timestamp TIMESTAMP NOT NULL,
    event TEXT NOT NULL,
    importance FLOAT NOT NULL,
    embedding VECTOR(1536)  -- For semantic search
);

-- Relationships
CREATE TABLE relationships (
    id UUID PRIMARY KEY,
    entity_a UUID REFERENCES entities(id),
    entity_b UUID REFERENCES entities(id),
    affinity FLOAT NOT NULL,
    trust FLOAT NOT NULL,
    data JSONB,
    UNIQUE(entity_a, entity_b)
);
```

---

## Performance Targets

### Gateway
- **Concurrent Connections**: 1,000+ simultaneous players
- **Latency**: <50ms for command processing
- **Throughput**: 10,000+ messages/second
- **Uptime**: 99.9% availability

### World Server
- **Tick Rate**: 10 ticks/second (100ms per tick)
- **Entity Count**: 100,000+ entities
- **System Processing**: <10ms per system per tick
- **Memory**: <2GB for 10,000 active entities

### AI System
- **GOAP Planning**: <5ms per decision
- **LLM Response**: <2s for dialogue generation
- **Cache Hit Rate**: >80% for common interactions
- **Cost**: <$0.01 per player hour

---

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Load Balancer                         │
│                   (nginx/haproxy)                        │
└────────┬────────────────────────────────┬───────────────┘
         │                                │
    ┌────▼─────┐                    ┌────▼─────┐
    │ Gateway  │                    │ Gateway  │
    │ Instance │                    │ Instance │
    │    #1    │                    │    #2    │
    └────┬─────┘                    └────┬─────┘
         │                                │
         └────────────┬───────────────────┘
                      │
              ┌───────▼────────┐
              │  World Server  │
              │   (Primary)    │
              └───────┬────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
    ┌────▼─────┐ ┌───▼────┐ ┌────▼─────┐
    │PostgreSQL│ │ Redis  │ │   LLM    │
    │          │ │ Cache  │ │ Service  │
    └──────────┘ └────────┘ └──────────┘
```

---

## Risk Mitigation

### Technical Risks
1. **LLM Cost Overruns**
   - Mitigation: Aggressive caching, rate limiting, local LLM fallback
   
2. **Performance Bottlenecks**
   - Mitigation: Profiling, benchmarking, horizontal scaling
   
3. **Connection Persistence Complexity**
   - Mitigation: Thorough testing, gradual rollout, fallback mechanisms

4. **AI Behavior Quality**
   - Mitigation: Extensive testing, human oversight, feedback loops

### Operational Risks
1. **Database Scaling**
   - Mitigation: Read replicas, connection pooling, caching
   
2. **LLM API Availability**
   - Mitigation: Multiple providers, local fallback, graceful degradation

---

## Success Metrics

### Technical Metrics
- [ ] 99.9% uptime
- [ ] <100ms average latency
- [ ] 1,000+ concurrent players supported
- [ ] <$100/month LLM costs for 100 active players
- [ ] Zero data loss during server restarts

### Gameplay Metrics
- [ ] NPCs pass basic Turing test (player can't easily distinguish)
- [ ] 90%+ player satisfaction with AI interactions
- [ ] <5% of AI responses require moderation
- [ ] NPCs exhibit consistent personalities

---

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| Phase 1: Core ECS | 3 weeks | Complete ECS implementation, event system |
| Phase 2: Gateway | 3 weeks | Telnet support, connection persistence |
| Phase 3: GOAP AI | 3 weeks | GOAP planner, pathfinding, basic behaviors |
| Phase 4: LLM Integration | 3 weeks | LLM providers, hybrid AI, memory system |
| Phase 5: Integration | 3 weeks | Combat, items, testing, documentation |
| **Total** | **15 weeks** | **Production-ready MUD** |

---

## Next Steps

1. **Immediate Actions**
   - Review and approve this plan
   - Set up project tracking (GitHub Projects/Jira)
   - Create detailed task breakdown for Phase 1
   - Set up CI/CD pipeline
   - Configure development environment

2. **Week 1 Tasks**
   - Implement core ECS components
   - Set up event system
   - Create component tests
   - Begin system implementations

3. **Dependencies to Add**
   - Update Cargo.toml with new dependencies
   - Set up LLM API keys (development)
   - Configure database migrations

---

## Appendix A: Code Examples

### Example: GOAP Action Implementation
```rust
pub struct AttackAction;

impl GOAPAction for AttackAction {
    fn name(&self) -> &str { "attack" }
    
    fn cost(&self) -> f32 { 1.0 }
    
    fn preconditions(&self) -> &WorldState {
        // Must have a target and be in range
        &WorldState::new()
            .with("has_target", true)
            .with("in_range", true)
            .with("has_weapon", true)
    }
    
    fn effects(&self) -> &WorldState {
        // Target takes damage
        &WorldState::new()
            .with("target_damaged", true)
    }
    
    fn can_execute(&self, world: &World, entity: Entity) -> bool {
        // Check if entity can actually attack
        world.get::<Combatant>(entity).is_ok()
    }
    
    fn execute(&self, world: &mut World, entity: Entity) -> ActionResult {
        // Perform attack logic
        // ...
        ActionResult::Success
    }
}
```

### Example: LLM Dialogue Generation
```rust
pub async fn generate_npc_dialogue(
    npc: Entity,
    player_input: &str,
    world: &World,
    llm: &dyn LLMProvider,
) -> Result<String> {
    let context = build_context(npc, world)?;
    
    let prompt = format!(
        "You are {}, {}. Current location: {}. Recent events: {}. \
         Player says: '{}'. Respond in character:",
        context.character_name,
        context.personality.background,
        context.current_location,
        context.recent_events.join(", "),
        player_input
    );
    
    let response = llm.generate(&prompt, &context).await?;
    let processed = sanitize_response(&response);
    
    Ok(processed)
}
```

---

## Appendix B: Configuration Examples

### config.yaml
```yaml
server:
  tick_rate: 10  # ticks per second
  max_entities: 100000
  
gateway:
  telnet:
    addr: "0.0.0.0"
    port: 4000
  websocket:
    addr: "0.0.0.0"
    port: 8080
  max_connections: 1000
  session_timeout: 3600  # seconds

ai:
  mode: "hybrid"  # goap_only, llm_only, hybrid, adaptive
  goap:
    max_planning_depth: 10
    planning_timeout: 5  # ms
  llm:
    provider: "openai"  # openai, anthropic, local
    model: "gpt-4o-mini"
    max_tokens: 200
    temperature: 0.7
    cache_ttl: 3600  # seconds
    rate_limit:
      requests_per_minute: 60
      tokens_per_minute: 100000
  memory:
    short_term_size: 20
    long_term_threshold: 0.7  # importance threshold

database:
  url: "postgresql://user:pass@localhost/wyldlands"
  max_connections: 20
  min_connections: 5
```

---

## Conclusion

This development plan provides a comprehensive roadmap for building a modern MUD with cutting-edge features. The phased approach ensures steady progress while maintaining flexibility to adapt based on learnings and feedback.

Key success factors:
- **Modular architecture** enables parallel development
- **ECS foundation** provides scalability and flexibility
- **Hybrid AI** balances performance and creativity
- **Connection persistence** ensures excellent UX
- **Comprehensive testing** ensures reliability

The 15-week timeline is aggressive but achievable with focused effort. Regular reviews and adjustments will be necessary to stay on track.
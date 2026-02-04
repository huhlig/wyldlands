# Wyldlands Developer's Guide

**Version**: 1.0  
**Last Updated**: January 28, 2026  
**Target Audience**: Developers extending the Wyldlands MUD system

---

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [Development Environment Setup](#development-environment-setup)
4. [Core Extension Points](#core-extension-points)
5. [Adding New Components](#adding-new-components)
6. [Adding New Systems](#adding-new-systems)
7. [Adding New Commands](#adding-new-commands)
8. [Adding New GOAP Actions](#adding-new-goap-actions)
9. [Extending the Gateway Protocol](#extending-the-gateway-protocol)
10. [Bidirectional RPC Communication](#bidirectional-rpc-communication)
11. [Database Schema Extensions](#database-schema-extensions)
12. [Testing Your Extensions](#testing-your-extensions)
13. [Best Practices](#best-practices)
14. [Common Patterns](#common-patterns)
15. [Troubleshooting](#troubleshooting)

---

## Introduction

Wyldlands is a modern MUD (Multi-User Dimension) built with Rust, featuring:
- **Entity Component System (ECS)** architecture using the `hecs` library
- **Distributed architecture** with separate Gateway and World Server
- **Advanced AI** with GOAP (Goal-Oriented Action Planning) and LLM integration
- **PostgreSQL persistence** for world state and player data
- **RPC communication** between gateway and server using `gRPC` (tonic)

This guide will help you extend the system by adding new features, components, systems, and commands.

---

## Architecture Overview

### High-Level Architecture

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
│              Connection Gateway                            │
│  • Session Management                                      │
│  • Protocol Adapters (WebSocket, Telnet)                  │
│  • Authentication & Account Management                     │
└─────────┬──────────────────────────────────────────────────┘
          │ RPC (gRPC)
          │
┌─────────▼──────────────────────────────────────────────────┐
│                    World Server (ECS)                       │
│  • Entity Component System (hecs)                          │
│  • Game Logic Systems                                      │
│  • GOAP AI Engine                                          │
│  • LLM Integration                                         │
└─────────┬──────────────────────────────────────────────────┘
          │ SQL
          │
┌─────────▼──────────────────────────────────────────────────┐
│                    PostgreSQL Database                      │
│  • World State & Entities                                  │
│  • Player Accounts & Characters                            │
└─────────────────────────────────────────────────────────────┘
```

### Key Directories

```
wyldlands/
├── common/              # Shared types and RPC protocol definitions
│   └── src/
│       ├── gateway.rs   # RPC service traits
│       ├── account.rs   # Account types
│       └── character.rs # Character types
├── gateway/             # Connection gateway server
│   └── src/
│       ├── session/     # Session management
│       ├── protocol/    # Protocol adapters
│       └── rpc_client.rs # RPC client
├── server/              # World server (game logic)
│   └── src/
│       ├── ecs/         # Entity Component System
│       │   ├── components/  # Component definitions
│       │   ├── systems/     # System implementations
│       │   ├── events/      # Event system
│       │   ├── context.rs   # WorldContext API
│       │   └── registry.rs  # Entity-UUID mapping
│       ├── llm/         # LLM integration
│       └── persistence.rs # Database persistence
└── migrations/          # Database migrations
```

---

## Development Environment Setup

### Prerequisites

```bash
# Required
- Rust 1.75 or later
- PostgreSQL 15 or later
- Git

# Optional (for Docker development)
- Docker
- Docker Compose
```

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/huhlig/wyldlands.git
cd wyldlands

# Set up PostgreSQL database
createdb wyldlands
psql wyldlands < migrations/001_table_setup.sql
psql wyldlands < migrations/002_settings_data.sql
psql wyldlands < migrations/003_world_data.sql
psql wyldlands < migrations/004_help_data.sql

# Configure environment
cp gateway/.env.example gateway/.env
cp server/.env.example server/.env
# Edit .env files with your database credentials

# Build the project
cargo build

# Run tests
cargo test
```

### Running the Development Environment

```bash
# Terminal 1: Start the world server
cargo run --bin server

# Terminal 2: Start the gateway
cargo run --bin gateway

# Or use Docker Compose
docker-compose up --build
```

---

## Core Extension Points

Wyldlands provides several well-defined extension points:

### 1. **Components** (`server/src/ecs/components/`)
Add new data structures that can be attached to entities.

### 2. **Systems** (`server/src/ecs/systems/`)
Add new game logic that operates on components.

### 3. **Commands** (`server/src/ecs/systems/command/`)
Add new player-facing commands.

### 4. **GOAP Actions** (`server/src/ecs/systems/actions.rs`)
Add new AI behaviors for NPCs.

### 5. **RPC Protocol** (`common/src/gateway.rs`)
Extend gateway-server communication.

### 6. **Database Schema** (`migrations/`)
Add new tables or columns for persistence.

---

## Adding New Components

Components are pure data structures that can be attached to entities. They should be serializable for persistence.

### Step 1: Create Component File

Create a new file in `server/src/ecs/components/` or add to an existing category file:

```rust
// server/src/ecs/components/magic.rs

use serde::{Deserialize, Serialize};

/// Component for entities that can cast spells
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCaster {
    /// Current mana points
    pub mana: i32,
    /// Maximum mana points
    pub max_mana: i32,
    /// Known spells
    pub known_spells: Vec<String>,
    /// Spell cooldowns (spell_name -> remaining_turns)
    pub cooldowns: std::collections::HashMap<String, u32>,
}

impl SpellCaster {
    pub fn new(max_mana: i32) -> Self {
        Self {
            mana: max_mana,
            max_mana,
            known_spells: Vec::new(),
            cooldowns: std::collections::HashMap::new(),
        }
    }
    
    pub fn can_cast(&self, spell: &str, cost: i32) -> bool {
        self.mana >= cost && 
        self.known_spells.contains(&spell.to_string()) &&
        !self.cooldowns.contains_key(spell)
    }
    
    pub fn cast_spell(&mut self, spell: &str, cost: i32) -> Result<(), String> {
        if !self.can_cast(spell, cost) {
            return Err("Cannot cast spell".to_string());
        }
        self.mana -= cost;
        Ok(())
    }
}

/// Component for spell effects on entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellEffect {
    pub effect_type: SpellEffectType,
    pub duration: u32,
    pub power: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpellEffectType {
    Buff,
    Debuff,
    DamageOverTime,
    HealOverTime,
}
```

### Step 2: Register Component

Add to `server/src/ecs/components.rs`:

```rust
mod magic;  // Add this line

// In the re-export section
pub use magic::*;  // Add this line
```

### Step 3: Add Persistence Support

If the component should be saved to the database, update `server/src/persistence.rs`:

```rust
// In save_entity function, add serialization:
if let Ok(spell_caster) = world.get::<&SpellCaster>(entity) {
    let data = serde_json::to_value(spell_caster)
        .map_err(|e| format!("Failed to serialize SpellCaster: {}", e))?;
    sqlx::query(
        "INSERT INTO components (entity_id, component_type, data) 
         VALUES ($1, $2, $3)
         ON CONFLICT (entity_id, component_type) 
         DO UPDATE SET data = $3"
    )
    .bind(uuid)
    .bind("SpellCaster")
    .bind(data)
    .execute(&self.pool)
    .await
    .map_err(|e| format!("Failed to save SpellCaster: {}", e))?;
}

// In load_entity function, add deserialization:
"SpellCaster" => {
    let component: SpellCaster = serde_json::from_value(row.data)
        .map_err(|e| format!("Failed to deserialize SpellCaster: {}", e))?;
    builder.add(component);
}
```

### Step 4: Use in Systems

```rust
// In any system, query for your component
let world = context.entities().read().await;
for (entity, (spell_caster, health)) in world.query::<(&mut SpellCaster, &Health)>().iter() {
    // Your logic here
}
```

---

## Adding New Systems

Systems contain game logic that operates on components. They run periodically or in response to events.

### Step 1: Create System File

Create `server/src/ecs/systems/magic.rs`:

```rust
use crate::ecs::components::{SpellCaster, SpellEffect, Health};
use crate::ecs::context::WorldContext;
use crate::ecs::events::{EventBus, GameEvent};
use std::sync::Arc;

/// System for processing spell effects and mana regeneration
pub struct MagicSystem {
    event_bus: EventBus,
}

impl MagicSystem {
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }
    
    /// Process spell effects and mana regeneration
    pub async fn update(&self, context: Arc<WorldContext>) {
        self.process_spell_effects(context.clone()).await;
        self.regenerate_mana(context).await;
    }
    
    async fn process_spell_effects(&self, context: Arc<WorldContext>) {
        let mut world = context.entities().write().await;
        
        for (entity, (effect, health)) in world.query::<(&mut SpellEffect, &mut Health)>().iter() {
            match effect.effect_type {
                SpellEffectType::DamageOverTime => {
                    health.current = health.current.saturating_sub(effect.power);
                    // Emit damage event
                    self.event_bus.publish(GameEvent::Damage {
                        entity,
                        amount: effect.power,
                        source: "spell".to_string(),
                    }).await;
                }
                SpellEffectType::HealOverTime => {
                    health.current = (health.current + effect.power).min(health.maximum);
                }
                _ => {}
            }
            
            // Decrease duration
            effect.duration = effect.duration.saturating_sub(1);
        }
        
        // Remove expired effects
        let expired: Vec<_> = world
            .query::<&SpellEffect>()
            .iter()
            .filter(|(_, effect)| effect.duration == 0)
            .map(|(entity, _)| entity)
            .collect();
            
        for entity in expired {
            let _ = world.remove_one::<SpellEffect>(entity);
        }
    }
    
    async fn regenerate_mana(&self, context: Arc<WorldContext>) {
        let mut world = context.entities().write().await;
        
        for (entity, spell_caster) in world.query::<&mut SpellCaster>().iter() {
            // Regenerate 5% of max mana per tick
            let regen = (spell_caster.max_mana as f32 * 0.05) as i32;
            spell_caster.mana = (spell_caster.mana + regen).min(spell_caster.max_mana);
            
            // Decrease cooldowns
            spell_caster.cooldowns.retain(|_, cooldown| {
                *cooldown = cooldown.saturating_sub(1);
                *cooldown > 0
            });
        }
    }
}
```

### Step 2: Register System

Add to `server/src/ecs/systems.rs`:

```rust
mod magic;  // Add this line

pub use magic::*;  // Add this line
```

### Step 3: Initialize and Run System

In `server/src/main.rs` or wherever systems are initialized:

```rust
let magic_system = MagicSystem::new(event_bus.clone());

// In your game loop
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        magic_system.update(context.clone()).await;
    }
});
```

---

## Adding New Commands

Commands are the primary way players interact with the game. The command system supports aliases, help text, and role-based permissions.

### Step 1: Create Command Module

Create `server/src/ecs/systems/command/magic.rs`:

```rust
use crate::ecs::components::{SpellCaster, Location, Name};
use crate::ecs::context::WorldContext;
use crate::ecs::systems::command::CommandResult;
use crate::ecs::EcsEntity;
use std::sync::Arc;

/// Cast a spell command
pub async fn cast_spell(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _command: String,
    args: Vec<String>,
) -> CommandResult {
    if args.is_empty() {
        return CommandResult::Invalid("Usage: cast <spell_name> [target]".to_string());
    }
    
    let spell_name = &args[0];
    let target_name = args.get(1).map(|s| s.as_str());
    
    // Get caster's spell caster component
    let world = context.entities().read().await;
    let spell_caster = match world.get::<&SpellCaster>(entity) {
        Ok(sc) => sc.clone(),
        Err(_) => {
            return CommandResult::Failure("You cannot cast spells!".to_string());
        }
    };
    
    // Check if spell is known
    if !spell_caster.known_spells.contains(&spell_name.to_string()) {
        return CommandResult::Failure(format!("You don't know the spell '{}'", spell_name));
    }
    
    // Get spell cost (in a real implementation, this would come from a spell database)
    let spell_cost = 20;
    
    // Check if can cast
    if !spell_caster.can_cast(spell_name, spell_cost) {
        return CommandResult::Failure("You don't have enough mana or the spell is on cooldown".to_string());
    }
    
    // Find target if specified
    let target_entity = if let Some(target_name) = target_name {
        find_target_in_room(&world, entity, target_name).await
    } else {
        None
    };
    
    drop(world);
    
    // Cast the spell
    let mut world = context.entities().write().await;
    if let Ok(mut spell_caster) = world.get::<&mut SpellCaster>(entity) {
        if let Err(e) = spell_caster.cast_spell(spell_name, spell_cost) {
            return CommandResult::Failure(e);
        }
    }
    
    // Apply spell effects to target
    if let Some(target) = target_entity {
        // Add spell effect component to target
        let effect = SpellEffect {
            effect_type: SpellEffectType::DamageOverTime,
            duration: 5,
            power: 10,
        };
        let _ = world.insert_one(target, effect);
    }
    
    CommandResult::Success(format!("You cast {}!", spell_name))
}

/// List known spells
pub async fn list_spells(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _command: String,
    _args: Vec<String>,
) -> CommandResult {
    let world = context.entities().read().await;
    
    let spell_caster = match world.get::<&SpellCaster>(entity) {
        Ok(sc) => sc,
        Err(_) => {
            return CommandResult::Failure("You cannot cast spells!".to_string());
        }
    };
    
    let mut output = format!("Mana: {}/{}\n\nKnown Spells:\n", 
        spell_caster.mana, spell_caster.max_mana);
    
    for spell in &spell_caster.known_spells {
        let cooldown = spell_caster.cooldowns.get(spell);
        if let Some(cd) = cooldown {
            output.push_str(&format!("  {} (cooldown: {} turns)\n", spell, cd));
        } else {
            output.push_str(&format!("  {}\n", spell));
        }
    }
    
    CommandResult::Success(output)
}

// Helper function to find target in same room
async fn find_target_in_room(
    world: &hecs::World,
    caster: EcsEntity,
    target_name: &str,
) -> Option<EcsEntity> {
    // Get caster's location
    let caster_location = world.get::<&Location>(caster).ok()?.room_id;
    
    // Find entities in same room with matching name
    for (entity, (name, location)) in world.query::<(&Name, &Location)>().iter() {
        if location.room_id == caster_location && 
           name.value.to_lowercase().contains(&target_name.to_lowercase()) {
            return Some(entity);
        }
    }
    
    None
}
```

### Step 2: Register Commands

Add to `server/src/ecs/systems/command.rs`:

```rust
mod magic;  // Add this line

// In register_default_commands() method:
fn register_default_commands(&mut self) {
    // ... existing commands ...
    
    // Magic commands
    self.register_command(
        "cast".to_string(),
        vec!["c".to_string()],
        "Cast a spell. Usage: cast <spell_name> [target]".to_string(),
        magic::cast_spell,
    );
    
    self.register_command(
        "spells".to_string(),
        vec!["sp".to_string()],
        "List your known spells and mana".to_string(),
        magic::list_spells,
    );
}
```

### Step 3: Add Help Documentation

Add entries to the database via migration or admin command:

```sql
INSERT INTO help_topics (keyword, category, title, content, syntax, examples, related_topics)
VALUES (
    'cast',
    'Command',
    'Cast Spell',
    'Cast a spell on yourself or a target. Requires mana and the spell must be known.',
    'cast <spell_name> [target]',
    'cast fireball goblin
cast heal
cast shield',
    ARRAY['spells', 'mana', 'magic']
);
```

---

## Adding New GOAP Actions

GOAP (Goal-Oriented Action Planning) actions define behaviors that NPCs can perform to achieve their goals.

### Step 1: Define Action in Action Library

Add to `server/src/ecs/systems/actions.rs`:

```rust
/// Action for NPCs to cast healing spells on themselves or allies
pub fn create_heal_action() -> GoapAction {
    let mut preconditions = HashMap::new();
    preconditions.insert("has_mana".to_string(), true);
    preconditions.insert("is_injured".to_string(), true);
    
    let mut effects = HashMap::new();
    effects.insert("is_injured".to_string(), false);
    effects.insert("has_mana".to_string(), false);  // Consumes mana
    
    GoapAction {
        name: "heal".to_string(),
        cost: 5.0,
        preconditions,
        effects,
    }
}

/// Action for NPCs to cast offensive spells
pub fn create_cast_attack_spell_action() -> GoapAction {
    let mut preconditions = HashMap::new();
    preconditions.insert("has_mana".to_string(), true);
    preconditions.insert("has_enemy".to_string(), true);
    preconditions.insert("in_combat".to_string(), true);
    
    let mut effects = HashMap::new();
    effects.insert("enemy_damaged".to_string(), true);
    effects.insert("has_mana".to_string(), false);
    
    GoapAction {
        name: "cast_attack_spell".to_string(),
        cost: 8.0,
        preconditions,
        effects,
    }
}
```

### Step 2: Register in Action Library

Update the `ActionLibrary::new()` method:

```rust
impl ActionLibrary {
    pub fn new() -> Self {
        let mut actions = HashMap::new();
        
        // Existing actions...
        actions.insert("wander".to_string(), create_wander_action());
        actions.insert("follow".to_string(), create_follow_action());
        
        // New magic actions
        actions.insert("heal".to_string(), create_heal_action());
        actions.insert("cast_attack_spell".to_string(), create_cast_attack_spell_action());
        
        Self { actions }
    }
}
```

### Step 3: Implement Action Execution

In `server/src/ecs/systems/npc_ai.rs`, add execution logic:

```rust
async fn execute_action(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    action_name: &str,
) -> Result<(), String> {
    match action_name {
        "heal" => {
            // Cast healing spell on self
            let world = context.entities().read().await;
            if let Ok(spell_caster) = world.get::<&SpellCaster>(entity) {
                if spell_caster.can_cast("heal", 15) {
                    drop(world);
                    let mut world = context.entities().write().await;
                    if let Ok(mut sc) = world.get::<&mut SpellCaster>(entity) {
                        sc.cast_spell("heal", 15)?;
                    }
                    if let Ok(mut health) = world.get::<&mut Health>(entity) {
                        health.current = (health.current + 30).min(health.maximum);
                    }
                }
            }
            Ok(())
        }
        "cast_attack_spell" => {
            // Find enemy and cast attack spell
            // Implementation similar to heal but targeting enemy
            Ok(())
        }
        // ... other actions ...
        _ => Err(format!("Unknown action: {}", action_name))
    }
}
```

### Step 4: Configure NPC to Use Actions

```rust
// Via command or programmatically
npc_goap_addaction(context, npc_entity, "heal".to_string()).await;
npc_goap_addaction(context, npc_entity, "cast_attack_spell".to_string()).await;

// Set goals
npc_goap_addgoal(context, npc_entity, "stay_healthy".to_string(), 10).await;
```

---

## Extending the Gateway Protocol

The RPC protocol defines communication between the gateway and world server.

### Step 1: Add RPC Method to Trait

Edit `common/src/gateway.rs`:

```rust
#[tonic::async_trait]
pub trait GatewayServer {
    // ... existing methods ...
    
    /// Get player's spell list
    async fn get_spell_list(
        session_id: SessionId,
    ) -> Result<SpellListResponse, String>;
    
    /// Learn a new spell
    async fn learn_spell(
        session_id: SessionId,
        spell_name: String,
    ) -> Result<(), String>;
}

/// Response containing spell information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellListResponse {
    pub known_spells: Vec<SpellInfo>,
    pub current_mana: i32,
    pub max_mana: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellInfo {
    pub name: String,
    pub cost: i32,
    pub cooldown: Option<u32>,
    pub description: String,
}
```

### Step 2: Implement Server-Side Handler

In `server/src/listener.rs`:

```rust
impl GatewayServer for ServerListener {
    async fn get_spell_list(
        self,
        request: tonic::Request<AuthenticateRequest>,
        session_id: SessionId,
    ) -> Result<SpellListResponse, String> {
        // Get entity from session
        let entity = self.get_entity_from_session(&session_id).await?;
        
        // Get spell caster component
        let world = self.context.entities().read().await;
        let spell_caster = world.get::<&SpellCaster>(entity)
            .map_err(|_| "Not a spell caster".to_string())?;
        
        let spells = spell_caster.known_spells.iter().map(|name| {
            SpellInfo {
                name: name.clone(),
                cost: 20, // Would come from spell database
                cooldown: spell_caster.cooldowns.get(name).copied(),
                description: "A powerful spell".to_string(),
            }
        }).collect();
        
        Ok(SpellListResponse {
            known_spells: spells,
            current_mana: spell_caster.mana,
            max_mana: spell_caster.max_mana,
        })
    }
    
    async fn learn_spell(
        self,
        request: tonic::Request<AuthenticateRequest>,
        session_id: SessionId,
        spell_name: String,
    ) -> Result<(), String> {
        let entity = self.get_entity_from_session(&session_id).await?;
        
        let mut world = self.context.entities().write().await;
        let mut spell_caster = world.get::<&mut SpellCaster>(entity)
            .map_err(|_| "Not a spell caster".to_string())?;
        
        if !spell_caster.known_spells.contains(&spell_name) {
            spell_caster.known_spells.push(spell_name);
            Ok(())
        } else {
            Err("Spell already known".to_string())
        }
    }
}
```

### Step 3: Use from Gateway

In `gateway/src/rpc_client.rs` or wherever you call RPC methods:

```rust
pub async fn get_player_spells(&self, session_id: &str) -> Result<SpellListResponse, String> {
    let client = self.client.lock().await;
    client.get_spell_list(session_id.to_string()).await
        .map_err(|e| format!("RPC error: {}", e))?
}
```

---

## Bidirectional RPC Communication

The gateway and server communicate bidirectionally using gRPC. Understanding this architecture is crucial for extending the system.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Gateway                              │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │ GatewayRpcServer (implements WorldToSession)       │    │
│  │ - Receives calls FROM server                       │    │
│  │ - Routes output to connected clients               │    │
│  └────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │ RpcClientManager (contains SessionToWorldClient)   │    │
│  │ - Sends calls TO server                            │    │
│  │ - Handles commands from clients                    │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                            ↕ gRPC
┌─────────────────────────────────────────────────────────────┐
│                         Server                               │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │ ServerRpcHandler                                   │    │
│  │ - Implements SessionToWorld & GatewayManagement    │    │
│  │ - Receives calls FROM gateway                      │    │
│  │ - Contains WorldToSessionClient                    │    │
│  │ - Sends calls TO gateway                           │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Communication Flows

#### 1. Gateway → Server (SessionToWorld)
```rust
// Gateway sends command to server
let response = session_client.send_input(SendInputRequest {
    session_id: session_id.to_string(),
    command: "look".to_string(),
}).await?;
```

#### 2. Server → Gateway (WorldToSession)
```rust
// Server sends output to gateway
let output = vec![GameOutput {
    output_type: Some(OutputType::Text(TextOutput {
        content: "You see a room.".to_string()
    })),
}];
self.send_output_to_session(&session_id, output).await?;
```

### Key Components

#### ServerRpcHandler (server/src/listener.rs)

The `ServerRpcHandler` handles both directions of communication:

```rust
pub struct ServerRpcHandler {
    // ... session management fields ...
    
    /// Client for sending messages back to gateway
    gateway_client: Arc<RwLock<Option<WorldToSessionClient>>>,
    
    /// Gateway address for connection
    gateway_addr: String,
}

impl ServerRpcHandler {
    /// Connect to the gateway server
    pub async fn connect_to_gateway(&self) -> Result<(), String> {
        let channel = Channel::from_shared(format!("http://{}", self.gateway_addr))?
            .connect()
            .await?;
        
        let client = WorldToSessionClient::new(channel);
        *self.gateway_client.write().await = Some(client);
        Ok(())
    }
    
    /// Send output to a session via the gateway
    async fn send_output_to_session(
        &self,
        session_id: &str,
        output: Vec<GameOutput>,
    ) -> Result<(), String> {
        let gateway_client = self.gateway_client.read().await;
        
        if let Some(client) = gateway_client.as_ref() {
            let mut client = client.clone();
            drop(gateway_client);
            
            client.send_output(SendOutputRequest {
                session_id: session_id.to_string(),
                output,
                error: None,
            }).await?;
            
            Ok(())
        } else {
            Err("Gateway client not connected".to_string())
        }
    }
}
```

#### GatewayRpcServer (gateway/src/grpc/server.rs)

The `GatewayRpcServer` receives output from the server and routes it to clients:

```rust
#[tonic::async_trait]
impl WorldToSession for GatewayRpcServer {
    async fn send_output(
        &self,
        request: Request<SendOutputRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let session_id = uuid::Uuid::parse_str(&req.session_id)?;
        
        // Route each output message to the client
        for output in req.output {
            self.connection_pool.send(session_id, output.into_bytes()).await?;
        }
        
        Ok(Response::new(Empty {}))
    }
}
```

### Best Practices for Bidirectional RPC

#### 1. Always Send Output Directly

**Don't** pack output in RPC responses:
```rust
// ❌ Bad - packing output in response
Ok(Response::new(SendInputResponse {
    success: true,
    output: vec![game_output],  // Don't do this
    error: None,
}))
```

**Do** send output via `send_output_to_session`:
```rust
// ✅ Good - send output directly
self.send_output_to_session(&session_id, vec![game_output]).await?;

Ok(Response::new(SendInputResponse {
    success: true,
    output: vec![],  // Empty - already sent
    error: None,
}))
```

#### 2. Handle Connection Failures Gracefully

```rust
// Try to send, but don't fail the command if sending fails
if let Err(e) = self.send_output_to_session(&session_id, output).await {
    tracing::warn!("Failed to send output to session {}: {}", session_id, e);
    // Command still succeeded, just couldn't send output
}
```

#### 3. Initialize Connection on Startup

In `server/src/main.rs`:
```rust
let handler = ServerRpcHandler::new(
    config.listener.auth_key.as_str(),
    world_context,
    &config.listener.gateway_addr.to_string(),
);

// Connect to gateway for sending messages back
if let Err(e) = handler.connect_to_gateway().await {
    tracing::warn!("Failed to connect to gateway: {}", e);
}
```

#### 4. Configure Gateway Address

In `server/config.yaml`:
```yaml
listener:
  addr: ${WYLDLANDS_LISTENER_ADDR:-0.0.0.0:6006}
  auth_key: ${WYLDLANDS_AUTH_KEY:-secret-key}
  gateway_addr: ${WYLDLANDS_GATEWAY_ADDR:-localhost:6005}
```

### Adding New Server→Gateway Messages

To add a new message type that the server sends to the gateway:

1. **Add to proto definition** (`common/proto/gateway.proto`):
```protobuf
service WorldToSession {
  // ... existing methods ...
  
  rpc SendNotification(NotificationRequest) returns (Empty);
}

message NotificationRequest {
  string session_id = 1;
  string notification_type = 2;
  string message = 3;
}
```

2. **Implement in GatewayRpcServer** (`gateway/src/grpc/server.rs`):
```rust
async fn send_notification(
    &self,
    request: Request<NotificationRequest>,
) -> Result<Response<Empty>, Status> {
    let req = request.into_inner();
    let session_id = uuid::Uuid::parse_str(&req.session_id)?;
    
    // Format and send notification
    let notification = format!("[{}] {}\r\n", req.notification_type, req.message);
    self.connection_pool.send(session_id, notification.into_bytes()).await?;
    
    Ok(Response::new(Empty {}))
}
```

3. **Use from ServerRpcHandler** (`server/src/listener.rs`):
```rust
async fn send_notification(
    &self,
    session_id: &str,
    notification_type: &str,
    message: &str,
) -> Result<(), String> {
    let gateway_client = self.gateway_client.read().await;
    
    if let Some(client) = gateway_client.as_ref() {
        let mut client = client.clone();
        drop(gateway_client);
        
        client.send_notification(NotificationRequest {
            session_id: session_id.to_string(),
            notification_type: notification_type.to_string(),
            message: message.to_string(),
        }).await?;
        
        Ok(())
    } else {
        Err("Gateway client not connected".to_string())
    }
}
```

---

## Database Schema Extensions

### Step 1: Create Migration File

Create `migrations/005_magic_system.sql`:

```sql
-- Add spell-related tables

CREATE TABLE spells (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT NOT NULL,
    mana_cost INTEGER NOT NULL,
    cooldown INTEGER NOT NULL DEFAULT 0,
    effect_type VARCHAR(50) NOT NULL,
    effect_power INTEGER NOT NULL,
    effect_duration INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE entity_spells (
    entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    spell_id UUID NOT NULL REFERENCES spells(id) ON DELETE CASCADE,
    learned_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, spell_id)
);

CREATE INDEX idx_entity_spells_entity ON entity_spells(entity_id);

-- Insert some default spells
INSERT INTO spells (name, description, mana_cost, cooldown, effect_type, effect_power, effect_duration)
VALUES 
    ('fireball', 'Hurls a ball of fire at the target', 20, 3, 'damage', 30, 0),
    ('heal', 'Restores health to the target', 15, 2, 'heal', 25, 0),
    ('shield', 'Creates a protective barrier', 25, 5, 'buff', 10, 3),
    ('poison', 'Inflicts poison damage over time', 18, 4, 'dot', 5, 5);
```

### Step 2: Apply Migration

```bash
psql wyldlands < migrations/005_magic_system.sql
```

### Step 3: Update Persistence Layer

Update `server/src/persistence.rs` to load/save spell data:

```rust
// In load_entity, after loading components:
let spells: Vec<String> = sqlx::query_scalar(
    "SELECT s.name FROM entity_spells es 
     JOIN spells s ON es.spell_id = s.id 
     WHERE es.entity_id = $1"
)
.bind(entity_id)
.fetch_all(&self.pool)
.await
.map_err(|e| format!("Failed to load spells: {}", e))?;

if !spells.is_empty() {
    if let Ok(mut spell_caster) = world.get::<&mut SpellCaster>(entity) {
        spell_caster.known_spells = spells;
    }
}
```

---

## Testing Your Extensions

### Unit Tests

Create tests alongside your code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_spell_caster_creation() {
        let caster = SpellCaster::new(100);
        assert_eq!(caster.mana, 100);
        assert_eq!(caster.max_mana, 100);
        assert!(caster.known_spells.is_empty());
    }
    
    #[test]
    fn test_can_cast_spell() {
        let mut caster = SpellCaster::new(100);
        caster.known_spells.push("fireball".to_string());
        
        assert!(caster.can_cast("fireball", 20));
        assert!(!caster.can_cast("fireball", 150));
        assert!(!caster.can_cast("unknown", 20));
    }
    
    #[tokio::test]
    async fn test_cast_spell_command() {
        let context = create_test_context().await;
        let entity = context.spawn((
            SpellCaster::new(100),
            Name::new("Test Wizard"),
        )).await;
        
        let result = cast_spell(
            context.clone(),
            entity,
            "cast".to_string(),
            vec!["fireball".to_string()],
        ).await;
        
        assert!(matches!(result, CommandResult::Success(_)));
    }
}
```

### Integration Tests

Create `server/tests/magic_integration_tests.rs`:

```rust
use wyldlands_server::ecs::*;
use wyldlands_server::persistence::PersistenceManager;

#[tokio::test]
async fn test_spell_casting_flow() {
    // Set up test database
    let pool = create_test_pool().await;
    let persistence = Arc::new(PersistenceManager::new(pool));
    let context = Arc::new(WorldContext::new(persistence));
    
    // Create wizard entity
    let wizard = context.spawn((
        SpellCaster::new(100),
        Health::new(50, 100),
        Name::new("Test Wizard"),
    )).await;
    
    // Create target entity
    let target = context.spawn((
        Health::new(100, 100),
        Name::new("Target"),
    )).await;
    
    // Cast spell
    let result = cast_spell(
        context.clone(),
        wizard,
        "cast".to_string(),
        vec!["fireball".to_string(), "target".to_string()],
    ).await;
    
    assert!(matches!(result, CommandResult::Success(_)));
    
    // Verify mana was consumed
    let world = context.entities().read().await;
    let spell_caster = world.get::<&SpellCaster>(wizard).unwrap();
    assert_eq!(spell_caster.mana, 80);
    
    // Verify target has spell effect
    assert!(world.get::<&SpellEffect>(target).is_ok());
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test magic_integration_tests

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_spell_casting_flow
```

---

## Best Practices

### 1. Component Design

- **Keep components simple**: Components should be pure data structures
- **Use composition**: Prefer multiple small components over large monolithic ones
- **Make serializable**: Always derive `Serialize` and `Deserialize` for persistence
- **Document fields**: Add doc comments explaining what each field represents

```rust
/// Component for entities that can cast spells
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCaster {
    /// Current mana points available for casting
    pub mana: i32,
    /// Maximum mana capacity
    pub max_mana: i32,
    /// List of spell names this entity knows
    pub known_spells: Vec<String>,
}
```

### 2. System Design

- **Single responsibility**: Each system should handle one aspect of game logic
- **Use WorldContext**: Always use the `WorldContext` API for safe access
- **Handle errors gracefully**: Don't panic, return `Result` types
- **Emit events**: Use the event bus to notify other systems of changes

```rust
pub async fn update(&self, context: Arc<WorldContext>) -> Result<(), String> {
    // Process logic
    // Emit events
    self.event_bus.publish(GameEvent::SpellCast { ... }).await;
    Ok(())
}
```

### 3. Command Design

- **Validate input**: Always check arguments before processing
- **Provide helpful errors**: Give users clear feedback on what went wrong
- **Use CommandResult**: Return appropriate result types
- **Check permissions**: Verify entity has required components/permissions

```rust
pub async fn my_command(
    context: Arc<WorldContext>,
    entity: EcsEntity,
    _command: String,
    args: Vec<String>,
) -> CommandResult {
    // Validate
    if args.is_empty() {
        return CommandResult::Invalid("Usage: mycommand <arg>".to_string());
    }
    
    // Check permissions
    let world = context.entities().read().await;
    if world.get::<&RequiredComponent>(entity).is_err() {
        return CommandResult::Failure("You cannot do that!".to_string());
    }
    
    // Execute
    // ...
    
    CommandResult::Success("Done!".to_string())
}
```

### 4. Database Design

- **Use migrations**: Never modify the schema directly
- **Add indexes**: Index foreign keys and frequently queried columns
- **Use constraints**: Enforce data integrity at the database level
- **Document schema**: Add comments to tables and columns

```sql
-- Good: Clear table with constraints and indexes
CREATE TABLE entity_spells (
    entity_id UUID NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    spell_id UUID NOT NULL REFERENCES spells(id) ON DELETE CASCADE,
    learned_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, spell_id)
);

CREATE INDEX idx_entity_spells_entity ON entity_spells(entity_id);
COMMENT ON TABLE entity_spells IS 'Tracks which spells each entity knows';
```

### 5. Error Handling

- **Use Result types**: Don't panic in production code
- **Provide context**: Include helpful error messages
- **Log errors**: Use `tracing` for debugging
- **Handle async errors**: Properly propagate errors in async code

```rust
pub async fn risky_operation(context: Arc<WorldContext>) -> Result<(), String> {
    let entity = context.get_entity_by_uuid(uuid).await
        .ok_or_else(|| format!("Entity {} not found", uuid))?;
    
    let world = context.entities().read().await;
    let component = world.get::<&MyComponent>(entity)
        .map_err(|e| format!("Failed to get component: {}", e))?;
    
    Ok(())
}
```

---

## Common Patterns

### Pattern 1: Entity Lookup by UUID

```rust
// Get entity from UUID
let entity = context.get_entity_by_uuid(uuid).await
    .ok_or_else(|| "Entity not found".to_string())?;

// Get UUID from entity
let uuid = context.get_uuid_by_entity(entity).await
    .ok_or_else(|| "Entity not registered".to_string())?;
```

### Pattern 2: Component Query

```rust
// Read-only query
let world = context.entities().read().await;
for (entity, (name, health)) in world.query::<(&Name, &Health)>().iter() {
    println!("{}: {}/{}", name.value, health.current, health.maximum);
}

// Mutable query
let mut world = context.entities().write().await;
for (entity, health) in world.query::<&mut Health>().iter() {
    health.current = health.maximum; // Full heal
}
```

### Pattern 3: Safe Component Access

```rust
// Check if component exists
let world = context.entities().read().await;
if let Ok(spell_caster) = world.get::<&SpellCaster>(entity) {
    // Use spell_caster
} else {
    return CommandResult::Failure("You cannot cast spells!".to_string());
}
```

### Pattern 4: Marking Entities Dirty

```rust
// After modifying an entity, mark it dirty for persistence
context.mark_entity_dirty(entity).await;

// Or by UUID
context.mark_dirty(uuid).await;
```

### Pattern 5: Event Publishing

```rust
// Publish an event
self.event_bus.publish(GameEvent::SpellCast {
    caster: entity,
    spell_name: "fireball".to_string(),
    target: Some(target_entity),
}).await;
```

### Pattern 6: Finding Entities in Room

```rust
async fn find_in_room(
    world: &hecs::World,
    searcher: EcsEntity,
    name: &str,
) -> Option<EcsEntity> {
    let searcher_location = world.get::<&Location>(searcher).ok()?.room_id;
    
    for (entity, (entity_name, location)) in world.query::<(&Name, &Location)>().iter() {
        if location.room_id == searcher_location && 
           entity_name.value.to_lowercase().contains(&name.to_lowercase()) {
            return Some(entity);
        }
    }
    None
}
```

---

## Troubleshooting

### Common Issues

#### 1. "Entity not found" errors

**Problem**: Trying to access an entity that doesn't exist or hasn't been registered.

**Solution**:
```rust
// Always check if entity exists
if !context.contains(entity).await {
    return Err("Entity does not exist".to_string());
}

// Or check registry
if context.get_uuid_by_entity(entity).await.is_none() {
    return Err("Entity not registered".to_string());
}
```

#### 2. Component not found

**Problem**: Querying for a component that the entity doesn't have.

**Solution**:
```rust
// Use pattern matching
let world = context.entities().read().await;
match world.get::<&MyComponent>(entity) {
    Ok(component) => { /* use component */ },
    Err(_) => return CommandResult::Failure("Missing required component".to_string()),
}
```

#### 3. Deadlocks

**Problem**: Holding multiple locks or acquiring locks in wrong order.

**Solution**:
```rust
// BAD: Holding lock while calling async function
let world = context.entities().read().await;
some_async_function().await; // Deadlock risk!

// GOOD: Drop lock before async call
let data = {
    let world = context.entities().read().await;
    world.get::<&MyComponent>(entity).ok().cloned()
};
some_async_function().await;
```

#### 4. Serialization errors

**Problem**: Component can't be serialized to database.

**Solution**:
```rust
// Ensure all fields are serializable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyComponent {
    // Use types that implement Serialize/Deserialize
    pub value: i32,
    pub name: String,
    pub data: HashMap<String, String>, // OK
    // pub callback: Box<dyn Fn()>, // NOT OK - can't serialize
}
```

#### 5. RPC connection failures

**Problem**: Gateway can't connect to world server.

**Solution**:
- Check server is running: `cargo run --bin server`
- Verify port configuration in `config.yaml`
- Check firewall settings
- Review logs: `RUST_LOG=debug cargo run --bin server`

### Debugging Tips

1. **Enable debug logging**:
```bash
RUST_LOG=debug cargo run --bin server
```

2. **Use tracing in your code**:
```rust
use tracing::{debug, info, warn, error};

debug!("Processing entity {:?}", entity);
info!("Spell cast: {}", spell_name);
warn!("Low mana: {}/{}", current, max);
error!("Failed to save entity: {}", err);
```

3. **Add test utilities**:
```rust
#[cfg(test)]
pub fn create_test_context() -> Arc<WorldContext> {
    let pool = create_test_pool();
    let persistence = Arc::new(PersistenceManager::new(pool));
    Arc::new(WorldContext::new(persistence))
}
```

4. **Use cargo check for fast feedback**:
```bash
cargo check  # Faster than full build
cargo clippy # Lint checks
```

---

## Additional Resources

### Documentation
- [Project Status](development/PROJECT_STATUS.md) - Current implementation status
- [Development Plan](development/DEVELOPMENT_PLAN.md) - Roadmap and phases
- [NPC System](NPC_SYSTEM.md) - NPC AI and GOAP documentation
- [Builder Commands](BUILDER_COMMANDS.md) - World building reference
- [LLM Generation](LLM_GENERATION.md) - AI content generation

### External Resources
- [Hecs ECS Documentation](https://docs.rs/hecs) - Entity Component System
- [Tokio Documentation](https://tokio.rs) - Async runtime
- [SQLx Documentation](https://docs.rs/sqlx) - Database library
- [tonic Documentation](https://docs.rs/tonic) - gRPC framework
- [prost Documentation](https://docs.rs/prost) - Protocol Buffers

### Community
- GitHub Issues: https://github.com/huhlig/wyldlands/issues
- Discussions: https://github.com/huhlig/wyldlands/discussions

---

## Conclusion

This guide has covered the main extension points in Wyldlands:

1. ✅ **Components** - Add new data structures
2. ✅ **Systems** - Add new game logic
3. ✅ **Commands** - Add player interactions
4. ✅ **GOAP Actions** - Add NPC behaviors
5. ✅ **RPC Protocol** - Extend gateway communication
6. ✅ **Database Schema** - Add persistence

The modular architecture makes it straightforward to extend the system while maintaining code quality and consistency. Follow the patterns and best practices outlined here, and you'll be able to add rich new features to the game.

Happy coding! 🎮

---

**Document Version**: 1.0  
**Last Updated**: January 28, 2026  
**Maintainer**: Wyldlands Development Team
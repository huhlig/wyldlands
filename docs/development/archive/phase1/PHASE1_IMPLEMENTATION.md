
# Phase 1: Core ECS Implementation - Detailed Implementation Plan

**Duration**: 3 weeks (15 working days)  
**Goal**: Build a solid ECS foundation with components, systems, and event infrastructure

---

## Week 1: Component Library & Core Infrastructure (Days 1-5)

### Day 1: Project Setup & Core Types

#### Tasks
1. **Create core module structure**
   ```
   server/src/
   ├── ecs/
   │   ├── mod.rs
   │   ├── components/
   │   │   ├── mod.rs
   │   │   ├── identity.rs
   │   │   ├── spatial.rs
   │   │   ├── character.rs
   │   │   ├── interaction.rs
   │   │   ├── ai.rs
   │   │   ├── combat.rs
   │   │   └── persistence.rs
   │   ├── systems/
   │   │   └── mod.rs
   │   └── events/
   │       ├── mod.rs
   │       ├── bus.rs
   │       └── types.rs
   ```

2. **Implement core types**
   ```rust
   // server/src/ecs/mod.rs
   pub use hecs::{Entity, World, Query, QueryBorrow};
   
   pub type EntityId = Entity;
   pub type GameWorld = World;
   
   // Re-exports
   pub mod components;
   pub mod systems;
   pub mod events;
   ```

3. **Set up testing infrastructure**
   ```rust
   // server/src/ecs/test_utils.rs
   pub fn create_test_world() -> GameWorld {
       World::new()
   }
   
   pub fn spawn_test_entity(world: &mut GameWorld) -> Entity {
       world.spawn((
           components::Name::new("Test Entity"),
           components::Position::default(),
       ))
   }
   ```

**Deliverables:**
- [ ] Module structure created
- [ ] Core type aliases defined
- [ ] Test utilities implemented
- [ ] Basic CI pipeline configured

---

### Day 2: Identity Components

#### Implementation

```rust
// server/src/ecs/components/identity.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for entities that need persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityUuid(pub Uuid);

impl EntityUuid {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EntityUuid {
    fn default() -> Self {
        Self::new()
    }
}

/// Display name and keywords for entity identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name {
    /// Primary display name
    pub display: String,
    /// Keywords for matching (e.g., "sword", "rusty", "blade")
    pub keywords: Vec<String>,
}

impl Name {
    pub fn new(display: impl Into<String>) -> Self {
        let display = display.into();
        let keywords = vec![display.to_lowercase()];
        Self { display, keywords }
    }
    
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords.into_iter()
            .map(|k| k.to_lowercase())
            .collect();
        self
    }
    
    pub fn matches(&self, keyword: &str) -> bool {
        let keyword = keyword.to_lowercase();
        self.keywords.iter().any(|k| k.starts_with(&keyword))
    }
}

/// Entity descriptions at various detail levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    /// Brief description (one line)
    pub short: String,
    /// Detailed description (multiple paragraphs)
    pub long: String,
    /// Dynamic description generator (optional)
    #[serde(skip)]
    pub generator: Option<Box<dyn DescriptionGenerator>>,
}

pub trait DescriptionGenerator: Send + Sync {
    fn generate(&self, world: &crate::ecs::GameWorld, entity: Entity) -> String;
}

impl Description {
    pub fn new(short: impl Into<String>, long: impl Into<String>) -> Self {
        Self {
            short: short.into(),
            long: long.into(),
            generator: None,
        }
    }
    
    pub fn get_long(&self, world: &crate::ecs::GameWorld, entity: Entity) -> String {
        if let Some(gen) = &self.generator {
            gen.generate(world, entity)
        } else {
            self.long.clone()
        }
    }
}

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    NPC,
    Item,
    Room,
    Exit,
    Container,
    Vehicle,
    Projectile,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_name_matching() {
        let name = Name::new("Rusty Sword")
            .with_keywords(vec!["rusty".into(), "sword".into(), "blade".into()]);
        
        assert!(name.matches("rus"));
        assert!(name.matches("sword"));
        assert!(name.matches("bla"));
        assert!(!name.matches("axe"));
    }
    
    #[test]
    fn test_entity_uuid_uniqueness() {
        let id1 = EntityUuid::new();
        let id2 = EntityUuid::new();
        assert_ne!(id1, id2);
    }
}
```

**Deliverables:**
- [ ] EntityUuid component
- [ ] Name component with keyword matching
- [ ] Description component with dynamic generation
- [ ] EntityType enum
- [ ] Unit tests (>90% coverage)

---

### Day 3: Spatial Components

#### Implementation

```rust
// server/src/ecs/components/spatial.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::ecs::EntityId;

/// 3D position in the world
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub area_id: u32,
    pub room_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn new(area_id: u32, room_id: u32) -> Self {
        Self {
            area_id,
            room_id,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
    
    pub fn with_coords(mut self, x: f32, y: f32, z: f32) -> Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }
    
    pub fn distance_to(&self, other: &Position) -> f32 {
        if self.area_id != other.area_id || self.room_id != other.room_id {
            return f32::INFINITY;
        }
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Container for holding other entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    /// Entities contained within
    pub contents: Vec<EntityId>,
    /// Maximum number of items
    pub capacity: Option<usize>,
    /// Maximum total weight
    pub max_weight: Option<f32>,
    /// Current total weight
    pub current_weight: f32,
    /// Container flags
    pub flags: ContainerFlags,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ContainerFlags {
    pub closeable: bool,
    pub closed: bool,
    pub lockable: bool,
    pub locked: bool,
    pub transparent: bool,
}

impl Container {
    pub fn new(capacity: Option<usize>) -> Self {
        Self {
            contents: Vec::new(),
            capacity,
            max_weight: None,
            current_weight: 0.0,
            flags: ContainerFlags::default(),
        }
    }
    
    pub fn can_add(&self, weight: f32) -> bool {
        if let Some(cap) = self.capacity {
            if self.contents.len() >= cap {
                return false;
            }
        }
        if let Some(max) = self.max_weight {
            if self.current_weight + weight > max {
                return false;
            }
        }
        !self.flags.closed
    }
    
    pub fn add(&mut self, entity: EntityId, weight: f32) -> Result<(), ContainerError> {
        if !self.can_add(weight) {
            return Err(ContainerError::Full);
        }
        self.contents.push(entity);
        self.current_weight += weight;
        Ok(())
    }
    
    pub fn remove(&mut self, entity: EntityId, weight: f32) -> Result<(), ContainerError> {
        if let Some(pos) = self.contents.iter().position(|&e| e == entity) {
            self.contents.remove(pos);
            self.current_weight -= weight;
            Ok(())
        } else {
            Err(ContainerError::NotFound)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerError {
    Full,
    NotFound,
    Closed,
    Locked,
}

/// Properties of containable entities
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Containable {
    pub weight: f32,
    pub size: Size,
    pub stackable: bool,
    pub stack_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
}

impl Containable {
    pub fn new(weight: f32) -> Self {
        Self {
            weight,
            size: Size::Medium,
            stackable: false,
            stack_size: 1,
        }
    }
}

/// Marks entities that can be entered (rooms, vehicles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enterable {
    pub capacity: Option<usize>,
    pub occupants: Vec<EntityId>,
}

impl Enterable {
    pub fn new() -> Self {
        Self {
            capacity: None,
            occupants: Vec::new(),
        }
    }
    
    pub fn can_enter(&self) -> bool {
        if let Some(cap) = self.capacity {
            self.occupants.len() < cap
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_distance() {
        let pos1 = Position::new(1, 1).with_coords(0.0, 0.0, 0.0);
        let pos2 = Position::new(1, 1).with_coords(3.0, 4.0, 0.0);
        assert_eq!(pos1.distance_to(&pos2), 5.0);
        
        let pos3 = Position::new(2, 1);
        assert_eq!(pos1.distance_to(&pos3), f32::INFINITY);
    }
    
    #[test]
    fn test_container_capacity() {
        let mut container = Container::new(Some(2));
        let entity1 = EntityId::from_bits(1).unwrap();
        let entity2 = EntityId::from_bits(2).unwrap();
        let entity3 = EntityId::from_bits(3).unwrap();
        
        assert!(container.add(entity1, 1.0).is_ok());
        assert!(container.add(entity2, 1.0).is_ok());
        assert!(container.add(entity3, 1.0).is_err());
    }
}
```

**Deliverables:**
- [ ] Position component with distance calculation
- [ ] Container component with capacity management
- [ ] Containable component
- [ ] Enterable component
- [ ] Unit tests

---

### Day 4: Character Components

#### Implementation

```rust
// server/src/ecs/components/character.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core character attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attributes {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

impl Attributes {
    pub fn new() -> Self {
        Self {
            strength: 10,
            dexterity: 10,
            constitution: 10,
            intelligence: 10,
            wisdom: 10,
            charisma: 10,
        }
    }
    
    pub fn modifier(&self, attr: AttributeType) -> i32 {
        let value = match attr {
            AttributeType::Strength => self.strength,
            AttributeType::Dexterity => self.dexterity,
            AttributeType::Constitution => self.constitution,
            AttributeType::Intelligence => self.intelligence,
            AttributeType::Wisdom => self.wisdom,
            AttributeType::Charisma => self.charisma,
        };
        (value - 10) / 2
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    Strength,
    Dexterity,
    Constitution,
    Intelligence,
    Wisdom,
    Charisma,
}

/// Health points
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Health {
    pub current: i32,
    pub max: i32,
    pub regeneration: f32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self {
            current: max,
            max,
            regeneration: 1.0,
        }
    }
    
    pub fn damage(&mut self, amount: i32) {
        self.current = (self.current - amount).max(0);
    }
    
    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
    
    pub fn is_alive(&self) -> bool {
        self.current > 0
    }
    
    pub fn percentage(&self) -> f32 {
        self.current as f32 / self.max as f32
    }
}

/// Mana/energy points
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
    pub regeneration: f32,
}

impl Mana {
    pub fn new(max: i32) -> Self {
        Self {
            current: max,
            max,
            regeneration: 1.0,
        }
    }
    
    pub fn spend(&mut self, amount: i32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }
    
    pub fn restore(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// Experience and leveling
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Experience {
    pub level: u32,
    pub current_xp: u64,
    pub xp_to_next: u64,
}

impl Experience {
    pub fn new() -> Self {
        Self {
            level: 1,
            current_xp: 0,
            xp_to_next: 1000,
        }
    }
    
    pub fn add_xp(&mut self, amount: u64) -> bool {
        self.current_xp += amount;
        if self.current_xp >= self.xp_to_next {
            self.level_up();
            true
        } else {
            false
        }
    }
    
    fn level_up(&mut self) {
        self.level += 1;
        self.current_xp -= self.xp_to_next;
        self.xp_to_next = self.calculate_xp_for_level(self.level + 1);
    }
    
    fn calculate_xp_for_level(&self, level: u32) -> u64 {
        // Simple exponential curve
        (1000.0 * (level as f64).powf(1.5)) as u64
    }
}

impl Default for Experience {
    fn default() -> Self {
        Self::new()
    }
}

/// Character skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skills {
    pub skills: HashMap<String, Skill>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Skill {
    pub level: u32,
    pub experience: u32,
}

impl Skills {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }
    
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }
    
    pub fn set(&mut self, name: String, skill: Skill) {
        self.skills.insert(name, skill);
    }
    
    pub fn improve(&mut self, name: &str, amount: u32) {
        if let Some(skill) = self.skills.get_mut(name) {
            skill.experience += amount;
            // Level up logic
            while skill.experience >= skill.level * 100 {
                skill.experience -= skill.level * 100;
                skill.level += 1;
            }
        }
    }
}

impl Default for Skills {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attribute_modifiers() {
        let mut attrs = Attributes::new();
        attrs.strength = 16;
        assert_eq!(attrs.modifier(AttributeType::Strength), 3);
        
        attrs.dexterity = 8;
        assert_eq!(attrs.modifier(AttributeType::Dexterity), -1);
    }
    
    #[test]
    fn test_health_damage() {
        let mut health = Health::new(100);
        health.damage(30);
        assert_eq!(health.current, 70);
        assert!(health.is_alive());
        
        health.damage(100);
        assert_eq!(health.current, 0);
        assert!(!health.is_alive());
    }
    
    #[test]
    fn test_experience_leveling() {
        let mut xp = Experience::new();
        let leveled = xp.add_xp(1000);
        assert!(leveled);
        assert_eq!(xp.level, 2);
    }
}
```

**Deliverables:**
- [ ] Attributes component
- [ ] Health component
- [ ] Mana component
- [ ] Experience component
- [ ] Skills component
- [ ] Unit tests

---

### Day 5: Interaction & AI Components

#### Implementation

```rust
// server/src/ecs/components/interaction.rs

use serde::{Deserialize, Serialize};
use crate::ecs::EntityId;

/// Marks entities that can receive and execute commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commandable {
    pub command_queue: Vec<QueuedCommand>,
    pub max_queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub priority: u8,
}

impl Commandable {
    pub fn new() -> Self {
        Self {
            command_queue: Vec::new(),
            max_queue_size: 10,
        }
    }
    
    pub fn queue_command(&mut self, command: String, args: Vec<String>) -> bool {
        if self.command_queue.len() >= self.max_queue_size {
            return false;
        }
        self.command_queue.push(QueuedCommand {
            command,
            args,
            priority: 0,
        });
        true
    }
    
    pub fn next_command(&mut self) -> Option<QueuedCommand> {
        if self.command_queue.is_empty() {
            None
        } else {
            Some(self.command_queue.remove(0))
        }
    }
}

impl Default for Commandable {
    fn default() -> Self {
        Self::new()
    }
}

/// Marks entities that can be interacted with
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interactable {
    pub interactions: Vec<Interaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub verb: String,
    pub description: String,
    pub requires_item: Option<String>,
}

impl Interactable {
    pub fn new() -> Self {
        Self {
            interactions: Vec::new(),
        }
    }
    
    pub fn add_interaction(&mut self, verb: String, description: String) {
        self.interactions.push(Interaction {
            verb,
            description,
            requires_item: None,
        });
    }
}

impl Default for Interactable {
    fn default() -> Self {
        Self::new()
    }
}

// server/src/ecs/components/ai.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI controller component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIController {
    pub behavior_type: BehaviorType,
    pub current_goal: Option<String>,
    pub state: AIState,
    pub update_interval: f32,
    pub time_since_update: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorType {
    Passive,      // Does nothing
    Wandering,    // Moves randomly
    Aggressive,   // Attacks on sight
    Defensive,    // Attacks when attacked
    Friendly,     // Helps players
    Merchant,     // Trades items
    Quest,        // Gives quests
    Custom,       // Custom behavior
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIState {
    Idle,
    Moving { target: EntityId },
    Combat { target: EntityId },
    Fleeing { from: EntityId },
    Following { target: EntityId },
    Dialogue { with: EntityId },
}

impl AIController {
    pub fn new(behavior_type: BehaviorType) -> Self {
        Self {
            behavior_type,
            current_goal: None,
            state: AIState::Idle,
            update_interval: 1.0,
            time_since_update: 0.0,
        }
    }
    
    pub fn should_update(&self, delta_time: f32) -> bool {
        self.time_since_update >= self.update_interval
    }
    
    pub fn mark_updated(&mut self) {
        self.time_since_update = 0.0;
    }
}

/// Personality traits for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub traits: HashMap<String, f32>,
    pub background: String,
    pub goals: Vec<String>,
    pub speaking_style: String,
}

impl Personality {
    pub fn new() -> Self {
        Self {
            traits: HashMap::new(),
            background: String::new(),
            goals: Vec::new(),
            speaking_style: "neutral".to_string(),
        }
    }
    
    pub fn set_trait(&mut self, trait_name: String, value: f32) {
        self.traits.insert(trait_name, value.clamp(-1.0, 1.0));
    }
    
    pub fn get_trait(&self, trait_name: &str) -> f32 {
        self.traits.get(trait_name).copied().unwrap_or(0.0)
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory system for NPCs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub short_term: Vec<MemoryEntry>,
    pub long_term: Vec<MemoryEntry>,
    pub max_short_term: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub timestamp: u64,
    pub event: String,
    pub importance: f32,
    pub entities_involved: Vec<EntityId>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            short_term: Vec::new(),
            long_term: Vec::new(),
            max_short_term: 20,
        }
    }
    
    pub fn add_memory(&mut self, event: String, importance: f32, entities: Vec<EntityId>) {
        let entry = MemoryEntry {
            timestamp: 0, // TODO: Use actual timestamp
            event,
            importance,
            entities_involved: entities,
        };
        
        self.short_term.push(entry.clone());
        
        // Move to long-term if important
        if importance > 0.7 {
            self.long_term.push(entry);
        }
        
        // Trim short-term memory
        if self.short_term.len() > self.max_short_term {
            self.short_term.remove(0);
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_commandable_queue() {
        let mut cmd = Commandable::new();
        assert!(cmd.queue_command("move".into(), vec!["north".into()]));
        assert!(cmd.next_command().is_some());
        assert!(cmd.next_command().is_none());
    }
    
    #[test]
    fn test_personality_traits() {
        let mut personality = Personality::new();
        personality.set_trait("friendly".into(), 0.8);
        assert_eq!(personality.get_trait("friendly"), 0.8);
        assert_eq!(personality.get_trait("aggressive"), 0.0);
    }
}
```

**Deliverables:**
- [ ] Commandable component
- [ ] Interactable component
- [ ] AIController component
- [ ] Personality component
- [ ] Memory component
- [ ] Unit tests

---

## Week 2: Systems & Event Infrastructure (Days 6-10)

### Day 6: Event System

#### Implementation

```rust
// server/src/ecs/events/types.rs

use serde::{Deserialize, Serialize};
use crate::ecs::EntityId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // Entity lifecycle
    EntitySpawned {
        entity: EntityId,
        entity_type: String,
        location: Option<(u32, u32)>,
    },
    EntityDespawned {
        entity: EntityId,
    },
    
    // Movement
    EntityMoved {
        entity: EntityId,
        from: (u32, u32),
        to: (u32, u32),
    },
    EntityEnteredRoom {
        entity: EntityId,
        room: u32,
    },
    EntityLeftRoom {
        entity: EntityId,
        room: u32,
    },
    
    // Combat
    CombatStarted {
        attacker: EntityId,
        defender: EntityId,
    },
    CombatEnded {
        participants: Vec<EntityId>,
    },
    EntityAttacked {
        attacker: EntityId,
        defender: EntityId,
        damage: i32,
    },
    EntityDied {
        entity: EntityId,
        killer: Option<EntityId>,
    },
    
    // Items
    ItemPickedUp {
        entity: EntityId,
        item: EntityId,
    },
    ItemDropped {
        entity: EntityId,
        item: EntityId,
    },
    ItemUsed {
        entity: EntityId,
        item: EntityId,
    },
    
    // Commands
    CommandExecuted {
        entity: EntityId,
        command: String,
        success: bool,
    },
    
    // Communication
    MessageSent {
        sender: EntityId,
        recipients: Vec<EntityId>,
        message: String,
        channel: MessageChannel,
    },
    
    // Custom events
    Custom {
        event_type: String,
        data: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageChannel {
    Say,
    Tell,
    Shout,
    Emote,
    System,
}

// server/src/ecs/events/bus.rs

use super::types::GameEvent;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::any::{Any, TypeId};

pub type EventHandler = Box<dyn Fn(&GameEvent) + Send + Sync>;

pub struct EventBus {
    handlers: Arc<RwLock<Vec<EventHandler>>>,
    event_queue: Arc<RwLock<Vec<GameEvent>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(Vec::new())),
            event_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn subscribe<F>(&self, handler: F)
    where
        F: Fn(&GameEvent) + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push(Box::new(handler));
    }
    
    pub fn publish(&self, event: GameEvent) {
        let mut queue = self.event_queue.write().unwrap();
        queue.push(event);
    }
    
    pub fn process_events(&self) {
        let mut queue = self.event_queue.write().unwrap();
        let events: Vec<_> = queue.drain(..).collect();
        drop(queue);
        
        let handlers = self.handlers.read().unwrap();
        for event in events {
            for handler in handlers.iter() {
                handler(&event);
            }
        }
    }
    
    pub fn clear(&self) {
        let mut queue = self.event_queue.write().unwrap();
        queue.clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            event_queue: Arc::clone(&self.event_queue),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_event_bus() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        
        bus.subscribe(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        bus.publish(GameEvent::Custom {
            event_type: "test".into(),
            data: "data".into(),
        });
        
        bus.process_events();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
```

**Deliverables:**
- [ ] GameEvent enum with all event types
- [ ] EventBus implementation
- [ ] Subscribe/publish mechanism
- [ ] Event processing
- [ ] Unit tests

---

### Day 7: Movement System

#### Implementation

```rust
// server/src/ecs/systems/movement.rs

use crate::ecs::{GameWorld, EntityId};
use crate::ecs::components::{Position, Commandable, Health};
use crate::ecs::events::{EventBus, GameEvent};

pub struct MovementSystem {
    event_bus: EventBus,
}

impl MovementSystem {
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }
    
    pub fn update(&mut self, world: &mut GameWorld, delta_time: f32) {
        // Process movement commands
        for (entity, (commandable, position)) in world.query_mut::<(&mut Commandable, &mut Position)>() {
            if let Some(cmd) = commandable.next_command() {
                if cmd.command == "move" {
                    if let Some(direction) = cmd.args.first() {
                        self.move_entity(world, entity, position, direction);
                    }
                }
            }
        }
    }
    
    fn move_entity(&mut self, world: &GameWorld, entity: EntityId, position: &mut Position, direction: &str) {
        let old_pos = *position;
        
        // Simple movement logic (will be enhanced with pathfinding later)
        match direction {
            "north" => position.y += 1.0,
            "south" => position.y -= 1.0,
            "east" => position.x += 1.0,
            "west" => position.x -= 1.0,
            "up" => position.z += 1.0,
            "down" => position.z -= 1.0,
            _ => return,
        }
        
        // Publish movement event
        self.event_bus.publish(GameEvent::EntityMoved {
            entity,
            from: (old_pos.area_id, old_pos.room_id),
            to: (position.area_id, position.room_id),
        });
    }
    
    pub fn teleport(&mut self, world: &mut GameWorld, entity: EntityId, target: Position) -> Result<(), String> {
        if let Ok(mut pos) = world.get::<&mut Position>(entity) {
            let old_pos = *pos;
            *pos = target;
            
            self.event_bus.publish(GameEvent::EntityMoved {
                entity,
                from: (old_pos.area_id, old_pos.room_id),
                to: (target.area_id, target.room_id),
            });
            
            Ok(())
        } else {
            Err("Entity has no position component".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Name;
    
    #[test]
    fn test_movement() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = MovementSystem::new(event_bus.clone());
        
        let entity = world.spawn((
            Name::new("Test"),
            Position::new(1, 1),
            Commandable::new(),
        ));
        
        // Queue movement command
        {
            let mut cmd = world.get::<&mut Commandable>(entity).unwrap();
            cmd.queue_command("move".into(), vec!["north".into()]);
        }
        
        system.update(&mut world, 0.1);
        
        let pos = world.get::<&Position>(entity).unwrap();
        assert_eq!(pos.y, 1.0);
    }
}
```

**Deliverables:**
- [ ] MovementSystem implementation
- [ ] Direction parsing
- [ ] Movement validation
- [ ] Event publishing
- [ ] Unit tests

---

### Day 8: Command System

#### Implementation

```rust
// server/src/ecs/systems/command.rs

use crate::ecs::{GameWorld, EntityId};
use crate::ecs::components::{Commandable, Name, Position, Container};
use crate::ecs::events::{EventBus, GameEvent};
use std::collections::HashMap;

pub type CommandFn = Box<dyn Fn(&mut GameWorld, EntityId, &[String]) -> CommandResult + Send + Sync>;

#[derive(Debug, Clone)]
pub enum CommandResult {
    Success(String),
    Failure(String),
    Invalid(String),
}

pub struct CommandSystem {
    commands: HashMap<String, CommandFn>,
    aliases: HashMap<String, String>,
    event_bus: EventBus,
}

impl CommandSystem {
    pub fn new(event_bus: EventBus) -> Self {
        let mut system = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
            event_bus,
        };
        
        system.register_default_commands();
        system
    }
    
    pub fn register_command<F>(&mut self, name: String, aliases: Vec<String>, handler: F)
    where
        F: Fn(&mut GameWorld, EntityId, &[String]) -> CommandResult + Send + Sync + 'static,
    {
        self.commands.insert(name.clone(), Box::new(handler));
        for alias in aliases {
            self.aliases.insert(alias, name.clone());
        }
    }
    
    pub fn execute(&mut self, world: &mut GameWorld, entity: EntityId, command: &str, args: &[String]) -> CommandResult {
        let cmd_name = command.to_lowercase();
        let cmd_name = self.aliases.get(&cmd_name).unwrap_or(&cmd_name).clone();
        
        if let Some(handler) = self.commands.get(&cmd_name) {
            let result = handler(world, entity, args);
            
            self.event_bus.publish(GameEvent::CommandExecuted {
                entity,
                command: command.to_string(),
                success: matches!(result, CommandResult::Success(_)),
            });
            
            result
        } else {
            CommandResult::Invalid(format!("Unknown command: {}", command))
        }
    }
    
    fn register_default_commands(&mut self) {
        // Look command
        self.register_command(
            "look".to_string(),
            vec!["l".to_string()],
            |world, entity, args| {
                if let Ok(pos) = world.get::<&Position>(entity) {
                    CommandResult::Success(format!(
                        "You are at area {}, room {}",
                        pos.area_id, pos.room_id
                    ))
                } else {
                    CommandResult::Failure("You have no position".to_string())
                }
            },
        );
        
        // Inventory command
        self.register_command(
            "inventory".to_string(),
            vec!["i".to_string(), "inv".to_string()],
            |world, entity, _args| {
                if let Ok(container) = world.get::<&Container>(entity) {
                    if container.contents.is_empty() {
                        CommandResult::Success("You are carrying nothing.".to_string())
                    } else {
                        CommandResult::Success(format!(
                            "You are carrying {} items.",
                            container.contents.len()
                        ))
                    }
                } else {
                    CommandResult::Failure("You have no inventory".to_string())
                }
            },
        );
        
        // Say command
        self.register_command(
            "say".to_string(),
            vec!["'".to_string()],
            |world, entity, args| {
                if args.is_empty() {
                    return CommandResult::Invalid("Say what?".to_string());
                }
                
                let message = args.join(" ");
                if let Ok(name) = world.get::<&Name>(entity) {
                    CommandResult::Success(format!("You say: '{}'", message))
                } else {
                    CommandResult::Failure("You cannot speak".to_string())
                }
            },
        );
    }
    
    pub fn update(&mut self, world: &mut GameWorld) {
        let mut commands_to_execute = Vec::new();
        
        // Collect commands from all commandable entities
        for (entity, commandable) in world.query_mut::<&mut Commandable>() {
            if let Some(cmd) = commandable.next_command() {
                commands_to_execute.push((entity, cmd.command, cmd.args));
            }
        }
        
        // Execute collected commands
        for (entity, command, args) in commands_to_execute {
            self.execute(world, entity, &command, &args);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_execution() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = CommandSystem::new(event_bus);
        
        let entity = world.spawn((
            Name::new("Test"),
            Position::new(1, 1),
        ));
        
        let result = system.execute(&mut world, entity, "look", &[]);
        assert!(matches!(result, CommandResult::Success(_)));
    }
}
```

**Deliverables:**
- [ ] CommandSystem with registry
- [ ] Default commands (look, inventory, say)
- [ ] Command aliases
- [ ] Command execution
- [ ] Unit tests

---

### Day 9: Inventory System

#### Implementation

```rust
// server/src/ecs/systems/inventory.rs

use crate::ecs::{GameWorld, EntityId};
use crate::ecs::components::{Container, Containable, Position};
use crate::ecs::events::{EventBus, GameEvent};

pub struct InventorySystem {
    event_bus: EventBus,
}

impl InventorySystem {
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }
    
    pub fn pickup_item(
        &mut self,
        world: &mut GameWorld,
        entity: EntityId,
        item: EntityId,
    ) -> Result<(), String> {
        // Get item weight
        let weight = world.get::<&Containable>(item)
            .map(|c| c.weight)
            .unwrap_or(0.0);
        
        // Add to container
        if let Ok(mut container) = world.get::<&mut Container>(entity) {
            container.add(item, weight)
                .map_err(|e| format!("Cannot pick up item: {:?}", e))?;
            
            // Remove item from world position
            if let Ok(mut pos) = world.get::<&mut Position>(item) {
                pos.area_id = 0;
                pos.room_id = 0;
            }
            
            self.event_bus.publish(GameEvent::ItemPickedUp { entity, item });
            Ok(())
        } else {
            Err("Entity has no inventory".to_string())
        }
    }
    
    pub fn drop_item(
        &mut self,
        world: &mut GameWorld,
        entity: EntityId,
        item: EntityId,
    ) -> Result<(), String> {
        // Get item weight
        let weight = world.get::<&Containable>(item)
            .map(|c| c.weight)
            .unwrap_or(0.0);
        
        // Remove from container
        if let Ok(mut container) = world.get::<&mut Container>(entity) {
            container.remove(item, weight)
                .map_err(|e| format!("Cannot drop item: {:?}", e))?;
            
            // Set item position to entity's position
            if let Ok(entity_pos) = world.get::<&Position>(entity) {
                if let Ok(mut item_pos) = world.get::<&mut Position>(item) {
                    *item_pos = *entity_pos;
                }
            }
            
            self.event_bus.publish(GameEvent::ItemDropped { entity, item });
            Ok(())
        } else {
            Err("Entity has no inventory".to_string())
        }
    }
    
    pub fn transfer_item(
        &mut self,
        world: &mut GameWorld,
        from: EntityId,
        to: EntityId,
        item: EntityId,
    ) -> Result<(), String> {
        self.drop_item(world, from, item)?;
        self.pickup_item(world, to, item)?;
        Ok(())
    }
    
    pub fn get_items_in_container(&self, world: &GameWorld, entity: EntityId) -> Vec<EntityId> {
        world.get::<&Container>(entity)
            .map(|c| c.contents.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Name;
    
    #[test]
    fn test_pickup_drop() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = InventorySystem::new(event_bus);
        
        let player = world.spawn((
            Name::new("Player"),
            Container::new(Some(10)),
            Position::new(1, 1),
        ));
        
        let item = world.spawn((
            Name::new("Sword"),
            Containable::new(5.0),
            Position::new(1, 1),
        ));
        
        assert!(system.pickup_item(&mut world, player, item).is_ok());
        assert!(system.drop_item(&mut world, player, item).is_ok());
    }
}
```

**Deliverables:**
- [ ] InventorySystem implementation
- [ ] Pickup/drop mechanics
- [ ] Item transfer
- [ ] Weight/capacity validation
- [ ] Unit tests

---

### Day 10: Persistence System

#### Implementation

```rust
// server/src/ecs/systems/persistence.rs

use crate::ecs::{GameWorld, EntityId};
use crate::ecs::components::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEntity {
    pub uuid: EntityUuid,
    pub components: HashMap<String, serde_json::Value>,
}

pub struct PersistenceSystem {
    db_pool: PgPool,
    dirty_entities: Vec<EntityId>,
}

impl PersistenceSystem {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            dirty_entities: Vec::new(),
        }
    }
    
    pub fn mark_dirty(&mut self, entity: EntityId) {
        if !self.dirty_entities.contains(&entity) {
            self.dirty_entities.push(entity);
        }
    }
    
    pub async fn save_entity(&self, world: &GameWorld, entity: EntityId) -> Result<(), sqlx::Error> {
        let mut components = HashMap::new();
        
        // Serialize all components
        if let Ok(uuid) = world.get::<&EntityUuid>(entity) {
            if let Ok(name) = world.get::<&Name>(entity) {
                components.insert("name".to_string(), serde_json::to_value(name).unwrap());
            }
            if let Ok(pos) = world.get::<&Position>(entity) {
                components.insert("position".to_string(), serde_json::to_value(pos).unwrap());
            }
            if let Ok(health) = world.get::<&Health>(entity) {
                components.insert("health".to_string(), serde_json::to_value(health).unwrap());
            }
            // Add more components as needed
            
            let serialized = SerializedEntity {
                uuid: *uuid,
                components,
            };
            
            // Save to database
            sqlx::query!(
                "INSERT INTO entities (id, data, updated_at) 
                 VALUES ($1, $2, NOW())
                 ON CONFLICT (id) DO UPDATE SET data = $2, updated_at = NOW()",
                uuid.0,
                serde_json::to_value(&serialized).unwrap()
            )
            .execute(&self.db_pool)
            .await?;
        }
        
        Ok(())
    }
    
    pub async fn load_entity(&self, world: &mut GameWorld, uuid: EntityUuid) -> Result<EntityId, sqlx::Error> {
        let row = sqlx::query!(
            "SELECT data FROM entities WHERE id = $1",
            uuid.0
        )
        .fetch_one(&self.db_pool)
        .await?;
        
        let serialized: SerializedEntity = serde_json::from_value(row.data).unwrap();
        
        // Create entity and deserialize components
        let entity = world.spawn((uuid,));
        
        for (component_name, value) in serialized.components {
            match component_name.as_str() {
                "name" => {
                    let name: Name = serde_json::from_value(value).unwrap();
                    world.insert_one(entity, name).ok();
                }
                "position" => {
                    let pos: Position = serde_json::from_value(value).unwrap();
                    world.insert_one(entity, pos).ok();
                }
                "health" => {
                    let health: Health = serde_json::from_value(value).unwrap();
                    world.insert_one(entity, health).ok();
                }
                // Add more components as needed
                _ => {}
            }
        }
        
        Ok(entity)
    }
    
    pub async fn save_all_dirty(&mut self, world: &GameWorld) -> Result<(), sqlx::Error> {
        for entity in self.dirty_entities.drain(..) {
            self.save_entity(world, entity).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_serialization() {
        let mut world = GameWorld::new();
        
        let entity = world.spawn((
            EntityUuid::new(),
            Name::new("Test"),
            Position::new(1, 1),
            Health::new(100),
        ));
        
        // Test would require database connection
        // This is a placeholder for integration tests
    }
}
```

**Deliverables:**
- [ ] PersistenceSystem implementation
- [ ] Entity serialization
- [ ] Entity deserialization
- [ ] Dirty tracking
- [ ] Database integration
- [ ] Integration tests

---

## Week 3: Integration & Testing (Days 11-15)

### Day 11: Combat System (Basic)

#### Implementation

```rust
// server/src/ecs/components/combat.rs

use serde::{Deserialize, Serialize};
use crate::ecs::EntityId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    pub in_combat: bool,
    pub target: Option<EntityId>,
    pub initiative: i32,
    pub attack_cooldown: f32,
    pub time_since_attack: f32,
}

impl Combatant {
    pub fn new() -> Self {
        Self {
            in_combat: false,
            target: None,
            initiative: 0,
            attack_cooldown: 1.0,
            time_since_attack: 0.0,
        }
    }
    
    pub fn can_attack(&self) -> bool {
        self.time_since_attack >= self.attack_cooldown
    }
}

impl Default for Combatant {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub slots: std::collections::HashMap<EquipSlot, EntityId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Head,
    Chest,
    Legs,
    Feet,
    Hands,
    MainHand,
    OffHand,
    Ring1,
    Ring2,
    Neck,
    Back,
}

impl Equipment {
    pub fn new() -> Self {
        Self {
            slots: std::collections::HashMap::new(),
        }
    }
    
    pub fn equip(&mut self, slot: EquipSlot, item: EntityId) -> Option<EntityId> {
        self.slots.insert(slot, item)
    }
    
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<EntityId> {
        self.slots.remove(&slot)
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Self::new()
    }
}

// server/src/ecs/systems/combat.rs

use crate::ecs::{GameWorld, EntityId};
use crate::ecs::components::{Combatant, Health, Attributes, AttributeType};
use crate::ecs::events::{EventBus, GameEvent};

pub struct CombatSystem {
    event_bus: EventBus,
}

impl CombatSystem {
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }
    
    pub fn start_combat(&mut self, world: &mut GameWorld, attacker: EntityId, defender: EntityId) {
        if let Ok(mut combatant) = world.get::<&mut Combatant>(attacker) {
            combatant.in_combat = true;
            combatant.target = Some(defender);
        }
        
        if let Ok(mut combatant) = world.get::<&mut Combatant>(defender) {
            combatant.in_combat = true;
            combatant.target = Some(attacker);
        }
        
        self.event_bus.publish(GameEvent::CombatStarted { attacker, defender });
    }
    
    pub fn attack(&mut self, world: &mut GameWorld, attacker: EntityId, defender: EntityId) -> Option<i32> {
        // Calculate damage
        let damage = if let Ok(attrs) = world.get::<&Attributes>(attacker) {
            let base_damage = 10;
            let str_mod = attrs.modifier(AttributeType::Strength);
            base_damage + str_mod
        } else {
            10
        };
        
        // Apply damage
        if let Ok(mut health) = world.get::<&mut Health>(defender) {
            health.damage(damage);
            
            self.event_bus.publish(GameEvent::EntityAttacked {
                attacker,
                defender,
                damage,
            });
            
            if !health.is_alive() {
                self.event_bus.publish(GameEvent::EntityDied {
                    entity: defender,
                    killer: Some(attacker),
                });
            }
            
            Some(damage)
        } else {
            None
        }
    }
    
    pub fn update(&mut self, world: &mut GameWorld, delta_time: f32) {
        let mut attacks = Vec::new();
        
        // Find entities ready to attack
        for (entity, combatant) in world.query_mut::<&mut Combatant>() {
            if combatant.in_combat {
                combatant.time_since_attack += delta_time;
                
                if combatant.can_attack() {
                    if let Some(target) = combatant.target {
                        attacks.push((entity, target));
                        combatant.time_since_attack = 0.0;
                    }
                }
            }
        }
        
        // Execute attacks
        for (attacker, defender) in attacks {
            self.attack(world, attacker, defender);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::Name;
    
    #[test]
    fn test_combat() {
        let mut world = GameWorld::new();
        let event_bus = EventBus::new();
        let mut system = CombatSystem::new(event_bus);
        
        let attacker = world.spawn((
            Name::new("Attacker"),
            Combatant::new(),
            Attributes::new(),
        ));
        
        let defender = world.spawn((
            Name::new("Defender"),
            Combatant::new(),
            Health::new(100),
        ));
        
        system.start_combat(&mut world, attacker, defender);
        let damage = system.attack(&mut world, attacker, defender);
        
        assert!(damage.is_some());
        assert!(damage.unwrap() > 0);
    }
}
```

**Deliverables:**
- [ ] Combatant component
- [ ] Equipment component
- [ ] CombatSystem implementation
- [ ] Attack mechanics
- [ ] Damage calculation
- [ ] Unit tests

---

### Days 12-13: Integration Testing

#### Test Suite Structure

```rust
// server/tests/integration_tests.rs

use wyldlands_worldserver::ecs::*;

#[test]
fn test_full_gameplay_loop() {
    // Create world
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    
    // Create systems
    let mut command_system = systems::CommandSystem::new(event_bus.clone());
    let mut movement_system = systems::MovementSystem::new(event_bus.clone());
    let mut inventory_system = systems::InventorySystem::new(event_bus.clone());
    let mut combat_system = systems::CombatSystem::new(event_bus.clone());
    
    // Spawn player
    let player = world.spawn((
        components::EntityUuid::new(),
        components::Name::new("Player"),
        components::Position::new(1, 1),
        components::Commandable::new(),
        components::Container::new(Some(20)),
        components::Health::new(100),
        components::Mana::new(50),
        components::Attributes::new(),
        components::Combatant::new(),
    ));
    
    // Spawn NPC
    let npc = world.spawn((
        components::EntityUuid::new(),
        components::Name::new("Goblin"),
        components::Position::new(1, 1),
        components::Health::new(50),
        components::Combatant::new(),
        components::AIController::new(components::BehaviorType::Aggressive),
    ));
    
    // Spawn item
    let sword = world.spawn((
        components::EntityUuid::new(),
        components::Name::new("Sword"),
        components::Position::new(1, 1),
        components::Containable::new(5.0),
    ));
    
    // Test pickup
    inventory_system.pickup_item(&mut world, player, sword).unwrap();
    
    // Test combat
    combat_system.start_combat(&mut world, player, npc);
    combat_system.update(&mut world, 1.0);
    
    // Verify NPC took damage
    let npc_health = world.get::<&components::Health>(npc).unwrap();
    assert!(npc_health.current < 50);
    
    // Process events
    event_bus.process_events();
}

#[test]
fn test_command_processing() {
    let mut world = GameWorld::new();
    let event_bus = EventBus::new();
    let mut command_system = systems::CommandSystem::new(event_bus);
    
    let player = world.spawn((
        components::Name::new("Player"),
        components::Position::new(1, 1),
        components::Commandable::new(),
    ));
    
    // Test look command
    let result = command_system.execute(&mut world, player, "look", &[]);
    assert!(matches!(result, systems::CommandResult::Success(_)));
    
    // Test invalid command
    let result = command_system.execute(&mut world, player, "invalid", &[]);
    assert!(matches!(result, systems::CommandResult::Invalid(_)));
}

#[test]
fn test_event_system() {
    let event_bus = EventBus::new();
    let mut event_count = 0;
    
    event_bus.subscribe(move |event| {
        event_count += 1;
    });
    
    event_bus.publish(GameEvent::Custom {
        event_type: "test".into(),
        data: "data".into(),
    });
    
    event_bus.process_events();
}
```

**Test Categories:**
- [ ] Component tests
- [ ] System tests
- [ ] Integration tests
- [ ] Event system tests
- [ ] Serialization tests
- [ ] Performance tests

---

### Days 14-15: Documentation & Polish

#### Documentation Tasks

1. **API Documentation**
   ```bash
   cargo doc --no-deps --open
   ```
   - Add doc comments to all public items
   - Include examples in doc comments
   - Document component relationships

2
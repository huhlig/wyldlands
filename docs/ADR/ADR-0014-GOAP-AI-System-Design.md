---
parent: ADR
nav_order: 0014
title: GOAP AI System Design
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0014: GOAP AI System Design

## Context and Problem Statement

NPCs need intelligent, goal-driven behavior that feels natural and responds to changing game conditions. The AI system must:
- Enable NPCs to make autonomous decisions
- Support complex multi-step plans
- React to dynamic world state changes
- Be extensible with new actions and goals
- Perform efficiently for many NPCs
- Integrate with LLM dialogue system
- Be debuggable and tunable

How should we design the NPC AI system to provide intelligent, flexible behavior?

## Decision Drivers

* **Intelligent Behavior**: NPCs should make sensible decisions
* **Goal-Oriented**: NPCs work toward specific objectives
* **Flexibility**: Easy to add new behaviors without code changes
* **Performance**: Efficient planning for many NPCs
* **Debuggability**: Clear visibility into AI decision-making
* **Extensibility**: Support for custom actions and goals
* **Integration**: Works with LLM dialogue and ECS architecture

## Considered Options

* GOAP (Goal-Oriented Action Planning) with A* Pathfinding
* Behavior Trees
* Finite State Machines
* Utility-Based AI

## Decision Outcome

Chosen option: "GOAP with A* Pathfinding", because it provides the best balance of flexibility, intelligence, and performance while enabling data-driven behavior configuration.

### GOAP Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    GoapPlanner                           │
│  • Actions: Available behaviors                          │
│  • Goals: Desired world states                           │
│  • World State: Current conditions                       │
│  • A* Planning: Find optimal action sequence             │
└─────────────────┬───────────────────────────────────────┘
                  │
                  │ Plan: [Action1, Action2, Action3]
                  ▼
┌─────────────────────────────────────────────────────────┐
│                  ActionLibrary                           │
│  Pre-built actions:                                      │
│  • WanderAction                                          │
│  • FollowAction                                          │
│  • AttackAction                                          │
│  • FleeAction                                            │
│  • PatrolAction                                          │
│  • GuardAction                                           │
│  • RestAction                                            │
│  • InteractAction                                        │
└─────────────────────────────────────────────────────────┘
```

### Core Components

**GoapAction:**
```rust
pub struct GoapAction {
    pub id: String,
    pub name: String,
    pub preconditions: WorldState,  // HashMap<String, bool>
    pub effects: WorldState,
    pub cost: f32,
    pub enabled: bool,
}
```

**GoapGoal:**
```rust
pub struct GoapGoal {
    pub id: String,
    pub name: String,
    pub priority: i32,
    pub conditions: WorldState,
    pub enabled: bool,
}
```

**GoapPlanner:**
```rust
pub struct GoapPlanner {
    pub actions: Vec<GoapAction>,
    pub goals: Vec<GoapGoal>,
    pub current_world_state: WorldState,
    pub current_plan: Option<Vec<String>>,
    pub current_goal: Option<String>,
}
```

### A* Planning Algorithm

The planner uses A* pathfinding to find the optimal sequence of actions:

1. **Start State**: Current world state
2. **Goal State**: Desired world state from highest priority goal
3. **Actions**: State transitions with costs
4. **Heuristic**: Number of unsatisfied goal conditions
5. **Result**: Sequence of actions to reach goal

**Planning Steps:**
```
1. Select highest priority achievable goal
2. Build A* search space:
   - Nodes: World states
   - Edges: Actions (with preconditions and effects)
   - Costs: Action costs
3. Find path from current state to goal state
4. Return action sequence
```

### Positive Consequences

* **Intelligent Planning**: NPCs find optimal paths to goals
* **Flexible Behavior**: Add new actions without code changes
* **Data-Driven**: Configure behavior via commands
* **Debuggable**: Can inspect plans and world state
* **Efficient**: A* finds optimal plans quickly
* **Extensible**: Easy to add new actions and goals
* **Reactive**: Replans when world state changes

### Negative Consequences

* **Complexity**: More complex than simple state machines
* **Planning Cost**: A* search has computational cost
* **Configuration**: Requires careful action/goal design
* **Debugging**: Complex plans can be hard to understand

## Pros and Cons of the Options

### GOAP with A* Pathfinding

* Good, because finds optimal action sequences
* Good, because data-driven configuration
* Good, because flexible and extensible
* Good, because handles complex multi-step plans
* Good, because reacts to changing conditions
* Neutral, because requires careful cost tuning
* Bad, because more complex than alternatives
* Bad, because planning has computational cost

### Behavior Trees

* Good, because visual and intuitive
* Good, because modular and reusable
* Good, because well-understood pattern
* Neutral, because requires tree editor
* Bad, because less flexible than GOAP
* Bad, because harder to add new behaviors
* Bad, because can become complex for advanced AI

### Finite State Machines

* Good, because simple and fast
* Good, because easy to understand
* Good, because predictable behavior
* Neutral, because requires state design
* Bad, because inflexible
* Bad, because state explosion for complex behavior
* Bad, because hard to extend

### Utility-Based AI

* Good, because handles multiple competing goals
* Good, because smooth priority transitions
* Neutral, because requires utility functions
* Bad, because harder to predict behavior
* Bad, because harder to debug
* Bad, because requires careful tuning

## Implementation Details

### Pre-Built Actions

**Location:** `server/src/ecs/systems/actions.rs`

1. **WanderAction**: Random movement
   - Preconditions: `is_idle = true`
   - Effects: `has_moved = true`
   - Cost: 2.0

2. **FollowAction**: Follow target entity
   - Preconditions: `has_target = true`
   - Effects: `near_target = true`
   - Cost: 3.0

3. **AttackAction**: Combat engagement
   - Preconditions: `has_enemy = true`, `in_range = true`
   - Effects: `enemy_damaged = true`
   - Cost: 5.0

4. **FleeAction**: Escape from threats
   - Preconditions: `in_danger = true`
   - Effects: `is_safe = true`
   - Cost: 4.0

5. **PatrolAction**: Waypoint-based patrol
   - Preconditions: `has_patrol_route = true`
   - Effects: `at_waypoint = true`
   - Cost: 2.0

6. **GuardAction**: Location guarding
   - Preconditions: `has_guard_post = true`
   - Effects: `at_guard_post = true`
   - Cost: 1.0

7. **RestAction**: Health/mana recovery
   - Preconditions: `is_injured = true`
   - Effects: `is_healed = true`
   - Cost: 3.0

8. **InteractAction**: Object interaction
   - Preconditions: `near_object = true`
   - Effects: `object_used = true`
   - Cost: 2.0

### NPC Commands

**GOAP Configuration:**
```
npc goap <uuid> addgoal <id> <name> <priority>
npc goap <uuid> addaction <id> <name> <cost>
npc goap <uuid> setstate <key> <value>
npc goap <uuid> show
```

**Example:**
```
# Add goal
npc goap 123e4567 addgoal patrol "Patrol Area" 10

# Add actions
npc goap 123e4567 addaction move_north "Move North" 1.0
npc goap 123e4567 addaction move_south "Move South" 1.0

# Set world state
npc goap 123e4567 setstate at_north false
npc goap 123e4567 setstate at_south false

# View configuration
npc goap 123e4567 show
```

### Integration with NPC AI Loop

**Location:** `server/src/ecs/systems/npc_ai.rs`

```rust
// 1. Update world state based on current conditions
planner.update_world_state("is_idle", !in_combat);
planner.update_world_state("has_enemy", enemy_nearby);

// 2. Plan if no current plan or goal changed
if planner.current_plan.is_none() || goal_changed {
    planner.plan();
}

// 3. Execute next action in plan
if let Some(action_id) = planner.get_next_action() {
    execute_action(action_id, entity).await?;
    planner.advance_plan();
}

// 4. Replan if action failed or world state changed significantly
if action_failed || significant_change {
    planner.replan();
}
```

### Hybrid AI: GOAP + LLM

GOAP handles tactical decisions, LLM handles dialogue:

```
┌─────────────────────────────────────────┐
│         NPC AI System                    │
│                                          │
│  ┌────────────────┐  ┌────────────────┐│
│  │  GOAP Planner  │  │  LLM Dialogue  ││
│  │  (Actions)     │  │  (Speech)      ││
│  └────────────────┘  └────────────────┘│
│         │                    │          │
│         │                    │          │
│         ▼                    ▼          │
│    Movement/Combat      Conversation    │
└─────────────────────────────────────────┘
```

## Validation

The GOAP AI system is validated by:

1. **Unit Tests**: Action/goal creation, planning algorithm
2. **Integration Tests**: Full NPC AI execution
3. **Planning Tests**: Multi-step plans, cost optimization
4. **Performance Tests**: Planning time for various scenarios
5. **Behavior Tests**: NPCs achieve goals correctly

## More Information

### Performance Characteristics

- **Planning Time**: <10ms for typical scenarios (5-10 actions, 3-5 goals)
- **Memory**: ~1KB per NPC planner
- **Scalability**: Supports 100+ NPCs with GOAP
- **Replan Frequency**: Only when world state changes significantly

### Debugging Tools

**Commands:**
```
npc goap <uuid> show          # View current configuration
npc goap <uuid> plan          # Force replanning
npc goap <uuid> state         # View world state
npc goap <uuid> actions       # List available actions
npc goap <uuid> goals         # List goals with priorities
```

### Future Enhancements

1. **Dynamic Action Generation**: LLM generates new actions
2. **Learning**: NPCs learn from successful/failed plans
3. **Cooperation**: Multi-NPC coordinated plans
4. **Emotional State**: Emotions affect action costs
5. **Memory Integration**: Past experiences influence planning
6. **Hierarchical Planning**: High-level and low-level goals

### Related Decisions

- [ADR-0004](ADR-0004-Use-Entity-Component-System.md) - ECS enables GOAP components
- [ADR-0013](ADR-0013-LLM-Integration-Architecture.md) - LLM complements GOAP for dialogue

### References

- GOAP Components: [server/src/ecs/components/ai/goap.rs](../../server/src/ecs/components/ai/goap.rs)
- Action Library: [server/src/ecs/systems/actions.rs](../../server/src/ecs/systems/actions.rs)
- NPC AI System: [server/src/ecs/systems/npc_ai.rs](../../server/src/ecs/systems/npc_ai.rs)
- NPC Commands: [server/src/ecs/systems/command/npc.rs](../../server/src/ecs/systems/command/npc.rs)
- NPC System Guide: [docs/NPC_SYSTEM.md](../NPC_SYSTEM.md)
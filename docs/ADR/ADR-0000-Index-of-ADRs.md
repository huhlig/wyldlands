---
parent: ADR
nav_order: 0000
title: Index of ADRs
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0000: Index of ADRs

This is a complete list of Architectural Decision Records for the Wyldlands project.

| ADR                                                                     | Title                                                  | Status   |
|:------------------------------------------------------------------------|:-------------------------------------------------------|:---------|
| [ADR-0000](ADR-0000-Index-of-ADRs.md)                                   | Index of ADRs                                          | N/A      |
| [ADR-0001](ADR-0001-ADR-Template.md)                                    | ADR Template                                           | N/A      |
| [ADR-0002](ADR-0002-Use-Markdown-Architectural-Decision-Records.md)     | Use Markdown Architectural Decision Records            | Accepted |
| [ADR-0003](ADR-0003-Use-Rust-Programming-Language.md)                   | Use Rust Programming Language                          | Accepted |
| [ADR-0004](ADR-0004-Use-Entity-Component-System.md)                     | Use Entity Component System Architecture               | Accepted |
| [ADR-0005](ADR-0005-Gateway-Server-Separation.md)                       | Gateway-Server Separation Architecture                 | Accepted |
| [ADR-0006](ADR-0006-Layered-State-Machine-Architecture.md)              | Layered State Machine Architecture                     | Accepted |
| [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md)        | Use gRPC for Inter-Service Communication               | Accepted |
| [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md)                  | Use PostgreSQL for Persistence                         | Accepted |
| [ADR-0009](ADR-0009-Protocol-Independence-Design.md)                    | Protocol Independence Design                           | Accepted |
| [ADR-0010](ADR-0010-Cargo-Workspace-Structure.md)                       | Cargo Workspace Structure                              | Accepted |
| [ADR-0011](ADR-0011-Character-Creation-System-Architecture.md)          | Character Creation System Architecture                 | Accepted |
| [ADR-0012](ADR-0012-Session-State-Management-Strategy.md)               | Session State Management Strategy                      | Accepted |
| [ADR-0013](ADR-0013-LLM-Integration-Architecture.md)                    | LLM Integration Architecture                           | Accepted |
| [ADR-0014](ADR-0014-GOAP-AI-System-Design.md)                           | GOAP AI System Design                                  | Accepted |
| [ADR-0015](ADR-0015-Database-Schema-Evolution-Strategy.md)              | Database Schema Evolution Strategy                     | Accepted |
| [ADR-0016](ADR-0016-Testing-Strategy-and-Coverage-Requirements.md)      | Testing Strategy and Coverage Requirements             | Accepted |
| [ADR-0017](ADR-0017-Side-Channel-Protocol-Support.md)                   | Side Channel Protocol Support                          | Accepted |
| [ADR-0018](ADR-0018-Input-Mode-Architecture.md)                         | Input Mode Architecture                                | Accepted |
| [ADR-0019](ADR-0019-Error-Handling-and-Recovery-Strategy.md)            | Error Handling and Recovery Strategy                   | Accepted |
| [ADR-0020](ADR-0020-Configuration-Management-Approach.md)               | Configuration Management Approach                      | Accepted |
| [ADR-0021](ADR-0021-Docker-Deployment-Architecture.md)                  | Docker Deployment Architecture                         | Accepted |
| [ADR-0022](ADR-0022-Termionix-Integration-for-Telnet-Support.md)        | Termionix Integration for Telnet Support               | Accepted |

## ADR Categories

### Language and Tooling
- **ADR-0003**: Rust Programming Language - Core language choice
- **ADR-0010**: Cargo Workspace Structure - Project organization
- **ADR-0022**: Termionix Integration - Telnet protocol library

### Architecture
- **ADR-0004**: Entity Component System - Game world architecture
- **ADR-0005**: Gateway-Server Separation - Distributed component design
- **ADR-0006**: Layered State Machine - Session state management
- **ADR-0009**: Protocol Independence - Multi-protocol support
- **ADR-0012**: Session State Management - Layered state machines

### Communication and Data
- **ADR-0007**: gRPC Communication - Inter-service protocol
- **ADR-0008**: PostgreSQL Persistence - Database choice
- **ADR-0015**: Database Schema Evolution - Migration strategy
- **ADR-0017**: Side Channel Protocols - MSDP, GMCP, WebSocket JSON

### Game Systems
- **ADR-0011**: Character Creation System - Point-buy character builder
- **ADR-0013**: LLM Integration - Multi-provider LLM support
- **ADR-0014**: GOAP AI System - Goal-oriented action planning
- **ADR-0018**: Input Mode Architecture - Playing vs Editing modes

### Operations
- **ADR-0016**: Testing Strategy - Unit, integration, and benchmark tests
- **ADR-0019**: Error Handling - Result-based error handling
- **ADR-0020**: Configuration Management - YAML with environment overrides
- **ADR-0021**: Docker Deployment - Container orchestration

### Documentation
- **ADR-0002**: Markdown ADRs - Documentation format

## Decision Dependencies

```
ADR-0003 (Rust)
    ├─► ADR-0004 (ECS) - Rust enables type-safe ECS
    ├─► ADR-0007 (gRPC) - Rust enables efficient gRPC with Tonic
    ├─► ADR-0008 (PostgreSQL) - Rust enables SQLx compile-time checking
    ├─► ADR-0010 (Workspace) - Rust enables workspace structure
    ├─► ADR-0016 (Testing) - Rust enables comprehensive testing
    └─► ADR-0019 (Error Handling) - Rust enables Result-based errors

ADR-0005 (Gateway-Server)
    ├─► ADR-0006 (State Machines) - Separation enables layered states
    ├─► ADR-0007 (gRPC) - Separation requires inter-service protocol
    ├─► ADR-0009 (Protocol Independence) - Gateway layer enables protocol adapters
    ├─► ADR-0012 (Session State) - Layered state machines
    └─► ADR-0021 (Docker) - Separate services enable containerization

ADR-0004 (ECS)
    ├─► ADR-0008 (PostgreSQL) - ECS components serialized to database
    ├─► ADR-0011 (Character Creation) - Character components in ECS
    └─► ADR-0014 (GOAP AI) - AI components in ECS

ADR-0008 (PostgreSQL)
    ├─► ADR-0011 (Character Creation) - Character data persistence
    ├─► ADR-0012 (Session State) - Session state persistence
    └─► ADR-0015 (Schema Evolution) - Database migration strategy

ADR-0009 (Protocol Independence)
    ├─► ADR-0017 (Side Channels) - Protocol-independent side channels
    ├─► ADR-0018 (Input Modes) - Protocol-independent input handling
    └─► ADR-0022 (Termionix) - Telnet protocol implementation

ADR-0012 (Session State)
    ├─► ADR-0011 (Character Creation) - Character creation is a session state
    └─► ADR-0018 (Input Modes) - Input modes are session substates

ADR-0013 (LLM Integration)
    └─► ADR-0014 (GOAP AI) - LLM complements GOAP for dialogue

ADR-0020 (Configuration)
    ├─► ADR-0013 (LLM Integration) - LLM provider configuration
    └─► ADR-0021 (Docker) - Configuration in containers
```

## Key Architectural Decisions

### 1. Modern MUD Server Design
The project uses a modern, distributed architecture with:
- Rust for performance and safety (ADR-0003)
- ECS for flexible game logic (ADR-0004)
- Separated gateway and server (ADR-0005)
- Protocol-independent design (ADR-0009)
- Layered state machines (ADR-0006, ADR-0012)

### 2. Scalability and Performance
Architectural choices support high performance:
- Zero-cost abstractions in Rust
- Efficient ECS iteration
- gRPC for low-latency RPC
- PostgreSQL for reliable persistence
- Connection pooling and caching
- Docker containerization (ADR-0021)

### 3. Maintainability and Extensibility
Design enables easy evolution:
- Clear component boundaries
- Protocol adapters for new protocols
- Layered state machines
- Workspace structure for code organization
- Compile-time safety guarantees
- Comprehensive testing (ADR-0016)

### 4. AI and Content Generation
Advanced AI capabilities:
- GOAP for tactical NPC behavior (ADR-0014)
- LLM for dialogue and content generation (ADR-0013)
- Hybrid AI system combining both approaches
- Multi-provider LLM support

### 5. Player Experience
Rich player features:
- Point-buy character creation (ADR-0011)
- Multiple input modes (ADR-0018)
- Side-channel protocols (ADR-0017)
- Telnet and WebSocket support (ADR-0022)
- Session persistence and reconnection

## Related Documentation

- [Project Status](../development/PROJECT_STATUS.md) - Current implementation status
- [Session State Engine](../development/SESSION_STATE_ENGINE.md) - State machine details
- [Gateway Protocol](../development/GATEWAY_PROTOCOL.md) - RPC protocol reference
- [Developer Guide](../DEVELOPER_GUIDE.md) - Development guidelines
- [NPC System](../NPC_SYSTEM.md) - NPC AI and dialogue
- [LLM Generation](../LLM_GENERATION.md) - Content generation
- [Builder Commands](../BUILDER_COMMANDS.md) - World building
- [Help System](../HELP_SYSTEM.md) - In-game help
- [Configuration](../CONFIGURATION.md) - Configuration guide
- [Docker Guide](../../DOCKER.md) - Docker deployment

## Adding New ADRs

When creating a new ADR:

1. Use the next sequential number (ADR-0023, ADR-0024, etc.)
2. Follow the template in [ADR-0001](ADR-0001-ADR-Template.md)
3. Add entry to this index
4. Update decision dependencies if applicable
5. Link to related ADRs in the new document
6. Update category sections above

## ADR Status Definitions

- **Proposed**: Under consideration
- **Accepted**: Decision made and implemented
- **Deprecated**: No longer applicable
- **Superseded**: Replaced by another ADR
- **Rejected**: Considered but not chosen

## Statistics

- **Total ADRs**: 22 (excluding index and template)
- **Accepted**: 20
- **Categories**: 6 (Language/Tooling, Architecture, Communication/Data, Game Systems, Operations, Documentation)
- **Last Updated**: 2026-02-01

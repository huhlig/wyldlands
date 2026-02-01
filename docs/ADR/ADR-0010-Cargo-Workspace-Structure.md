---
parent: ADR
nav_order: 0010
title: Cargo Workspace Structure
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0010: Cargo Workspace Structure

## Context and Problem Statement

We need to organize the Rust codebase for a multi-component system (gateway, server, common libraries) with:
- Shared dependencies and version management
- Independent compilation and testing
- Code reuse between components
- Clear module boundaries
- Efficient build times
- Easy dependency management

How should we structure the Rust project to support multiple related components?

## Decision Drivers

* **Code Organization**: Clear separation between components
* **Dependency Management**: Shared dependencies with consistent versions
* **Build Efficiency**: Incremental compilation and caching
* **Code Reuse**: Shared code between gateway and server
* **Testing**: Independent testing of each component
* **Deployment**: Separate binaries for gateway and server
* **Maintainability**: Easy to navigate and understand

## Considered Options

* Cargo Workspace with Multiple Crates
* Monolithic Single Crate
* Separate Repositories
* Git Submodules

## Decision Outcome

Chosen option: "Cargo Workspace with Multiple Crates", because it provides the best balance of code organization, dependency management, and build efficiency while maintaining clear boundaries between components.

### Workspace Structure

```
wyldlands/
├── Cargo.toml              # Workspace root
├── common/                 # Shared library
│   ├── Cargo.toml
│   ├── build.rs           # Protocol buffer generation
│   ├── proto/
│   │   └── gateway.proto  # gRPC protocol definition
│   └── src/
│       ├── lib.rs
│       ├── gateway.rs     # Generated gRPC code
│       └── utility.rs     # Shared utilities
├── gateway/                # Gateway server
│   ├── Cargo.toml
│   ├── config.yaml
│   ├── src/
│   │   ├── main.rs        # Gateway binary
│   │   ├── lib.rs         # Gateway library
│   │   ├── auth.rs
│   │   ├── session.rs
│   │   ├── pool.rs
│   │   ├── grpc/          # gRPC client
│   │   ├── protocol/      # Protocol adapters
│   │   └── server/        # Protocol servers
│   ├── tests/             # Integration tests
│   └── benches/           # Benchmarks
├── server/                 # World server
│   ├── Cargo.toml
│   ├── config.yaml
│   ├── src/
│   │   ├── main.rs        # Server binary
│   │   ├── lib.rs         # Server library
│   │   ├── listener.rs    # gRPC server
│   │   ├── persistence.rs
│   │   ├── ecs/           # ECS implementation
│   │   └── models/        # LLM integration
│   ├── tests/             # Integration tests
│   └── benches/           # Benchmarks
├── migrations/             # Database migrations
├── docs/                   # Documentation
└── docker-compose.yml      # Docker deployment
```

### Positive Consequences

* **Shared Dependencies**: Single version management in workspace Cargo.toml
* **Code Reuse**: Common library shared between gateway and server
* **Independent Builds**: Each crate can be built/tested independently
* **Incremental Compilation**: Cargo caches unchanged crates
* **Clear Boundaries**: Physical separation enforces architectural boundaries
* **Easy Testing**: Each component has its own test suite
* **Flexible Deployment**: Separate binaries for gateway and server
* **Unified Tooling**: Single `cargo test`, `cargo build` for entire workspace

### Negative Consequences

* **Build Complexity**: More complex than single crate
* **Dependency Coordination**: Must keep versions synchronized
* **Initial Setup**: More configuration required

## Pros and Cons of the Options

### Cargo Workspace with Multiple Crates

* Good, because shared dependency management
* Good, because clear component boundaries
* Good, because independent compilation
* Good, because code reuse via common crate
* Good, because incremental builds
* Good, because separate binaries
* Neutral, because requires workspace configuration
* Bad, because slightly more complex than monolith

### Monolithic Single Crate

```
wyldlands/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── gateway/
    ├── server/
    └── common/
```

* Good, because simple structure
* Good, because single Cargo.toml
* Neutral, because all code in one crate
* Bad, because no clear boundaries
* Bad, because cannot build components independently
* Bad, because single binary or complex build scripts
* Bad, because harder to maintain separation

### Separate Repositories

```
wyldlands-common/
wyldlands-gateway/
wyldlands-server/
```

* Good, because complete independence
* Good, because separate versioning
* Neutral, because requires publishing common crate
* Bad, because harder to coordinate changes
* Bad, because more complex dependency management
* Bad, because harder to make cross-cutting changes
* Bad, because overkill for tightly coupled components

### Git Submodules

```
wyldlands/
├── common/        (submodule)
├── gateway/       (submodule)
└── server/        (submodule)
```

* Good, because independent repositories
* Neutral, because Git submodule management
* Bad, because complex workflow
* Bad, because harder to coordinate changes
* Bad, because submodule synchronization issues
* Bad, because unnecessary complexity

## Implementation Details

### Workspace Cargo.toml

**Location:** `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "common",
    "gateway",
    "server",
]

[workspace.package]
version = "0.0.1"
edition = "2024"
publish = false
authors = ["Hans W. Uhlig <huhlig@gmail.com>"]
license = "APACHE2 or MIT"
repository = "https://github.com/huhlig/wyldlands"

[workspace.dependencies]
# Shared dependencies with consistent versions
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
uuid = { version = "1", features = ["serde", "v4"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres"] }

# gRPC
tonic = "0.14"
prost = "0.14"

# ECS
hecs = { version = "0.11", features = ["serde"] }

# Web
axum = { version = "0.8", features = ["ws"] }

# Internal crates
wyldlands-common = { path = "common" }
wyldlands-gateway = { path = "gateway" }
wyldlands-server = { path = "server" }
```

### Common Crate

**Purpose:** Shared code between gateway and server

**Location:** `common/Cargo.toml`

```toml
[package]
name = "wyldlands-common"
version.workspace = true
edition.workspace = true

[dependencies]
# Use workspace dependencies
tonic.workspace = true
prost.workspace = true
serde.workspace = true
uuid.workspace = true

[build-dependencies]
tonic-build = "0.14"
```

**Exports:**
- gRPC protocol definitions (generated from .proto)
- Shared data structures
- Utility functions
- Common types

### Gateway Crate

**Purpose:** Connection gateway server

**Location:** `gateway/Cargo.toml`

```toml
[package]
name = "wyldlands-gateway"
version.workspace = true
edition.workspace = true

[[bin]]
name = "gateway"
path = "src/main.rs"

[lib]
name = "wyldlands_gateway"
path = "src/lib.rs"

[dependencies]
# Workspace dependencies
wyldlands-common.workspace = true
tokio.workspace = true
axum.workspace = true
tonic.workspace = true
sqlx.workspace = true

# Gateway-specific dependencies
termionix-service = { git = "https://github.com/huhlig/termionix.git" }
```

**Exports:**
- Gateway library (for testing)
- Gateway binary

### Server Crate

**Purpose:** World server with game logic

**Location:** `server/Cargo.toml`

```toml
[package]
name = "wyldlands-server"
version.workspace = true
edition.workspace = true

[[bin]]
name = "server"
path = "src/main.rs"

[lib]
name = "wyldlands_server"
path = "src/lib.rs"

[dependencies]
# Workspace dependencies
wyldlands-common.workspace = true
tokio.workspace = true
tonic.workspace = true
sqlx.workspace = true
hecs.workspace = true

# Server-specific dependencies
mistralrs = "0.7"
```

**Exports:**
- Server library (for testing)
- Server binary

### Build Commands

**Build entire workspace:**
```bash
cargo build --release
```

**Build specific crate:**
```bash
cargo build -p wyldlands-gateway --release
cargo build -p wyldlands-server --release
```

**Test entire workspace:**
```bash
cargo test
```

**Test specific crate:**
```bash
cargo test -p wyldlands-gateway
cargo test -p wyldlands-server
```

**Run specific binary:**
```bash
cargo run --bin gateway
cargo run --bin server
```

### Dependency Management

**Adding a dependency to workspace:**

1. Add to `[workspace.dependencies]` in root Cargo.toml
2. Reference in crate with `.workspace = true`

```toml
# Root Cargo.toml
[workspace.dependencies]
new-crate = "1.0"

# gateway/Cargo.toml
[dependencies]
new-crate.workspace = true
```

**Adding a crate-specific dependency:**

```toml
# gateway/Cargo.toml
[dependencies]
gateway-only-crate = "1.0"
```

### Code Organization Benefits

**Clear Module Boundaries:**
```rust
// Gateway can use common
use wyldlands_common::gateway::{SendInputRequest, SendInputResponse};

// Gateway cannot use server (not in dependencies)
// use wyldlands_server::ecs::World; // ❌ Compile error

// Server can use common
use wyldlands_common::gateway::gateway_server_server::GatewayServer;

// Server cannot use gateway (not in dependencies)
// use wyldlands_gateway::session::Session; // ❌ Compile error
```

**Enforced Separation:**
- Gateway and server cannot directly depend on each other
- All communication through common crate (gRPC protocol)
- Prevents tight coupling
- Enforces architectural boundaries

## Validation

The workspace structure is validated by:

1. **Build Success:**
   - All crates compile successfully
   - No circular dependencies
   - Clean dependency graph

2. **Test Independence:**
   - Gateway tests: 70+ tests
   - Server tests: 145+ tests
   - Common tests: Protocol validation
   - All can run independently

3. **Code Organization:**
   - Clear boundaries between components
   - No cross-component dependencies (except common)
   - Easy to navigate codebase

4. **Build Performance:**
   - Incremental compilation works well
   - Unchanged crates not rebuilt
   - Parallel compilation of independent crates

## More Information

### Workspace Features

**Unified Commands:**
```bash
# Format all code
cargo fmt

# Lint all code
cargo clippy

# Generate documentation
cargo doc --no-deps --open

# Run benchmarks
cargo bench
```

**Dependency Resolution:**
- Cargo resolves dependencies once for entire workspace
- Shared dependencies use same version
- Reduces compilation time
- Smaller target directory

**Feature Flags:**
```toml
[workspace.dependencies]
tokio = { version = "1", features = ["rt-multi-thread"] }

# Gateway adds more features
[dependencies]
tokio = { workspace = true, features = ["net", "signal"] }
```

### Best Practices

**Workspace Organization:**
- Keep common crate minimal (only shared code)
- Each crate should be independently useful
- Avoid circular dependencies
- Use workspace dependencies for consistency

**Version Management:**
- Use workspace.version for all crates
- Bump version in one place
- Consistent versioning across components

**Testing:**
- Each crate has its own tests
- Integration tests in each crate
- Workspace-level tests if needed

### Future Enhancements

1. **Additional Crates:**
   - `wyldlands-cli` - Command-line tools
   - `wyldlands-web` - Web dashboard
   - `wyldlands-api` - REST API

2. **Feature Organization:**
   - Optional features per crate
   - Feature-gated functionality
   - Conditional compilation

3. **Optimization:**
   - Profile-guided optimization
   - Link-time optimization
   - Binary size reduction

### Related Decisions

- [ADR-0003](ADR-0003-Use-Rust-Programming-Language.md) - Rust enables workspace structure
- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Separation reflected in workspace
- [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md) - Common crate contains protocol

### References

- [Cargo Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Cargo Reference](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- Workspace Root: [Cargo.toml](../../Cargo.toml)
- Common Crate: [common/Cargo.toml](../../common/Cargo.toml)
- Gateway Crate: [gateway/Cargo.toml](../../gateway/Cargo.toml)
- Server Crate: [server/Cargo.toml](../../server/Cargo.toml)
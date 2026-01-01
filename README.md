# Wyldlands Multi-Player Dimension 

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Actions Status](https://github.com/huhlig/wyldlands/workflows/rust/badge.svg)](https://github.com/huhlig/wyldlands/actions)

([API Docs])

> Wyldlands Multi-User Dimension (WyldMUD) is an experimental vibe coded multiplayer adventure simulation inspired by 
> classic Multi-User Dimensions like DikuMUD and TinyMuck. It is an experiment in both the usage of agentic application
> creation and usage of LLMs in a fantasy virtual world simulation. Its core design is modular with a reasonable 
> framework for extension. The default AI is a mixture of a Goal Oriented Action Planning statechart with an LLM used 
> to make decisions.

## Quick Start

### Using Docker (Recommended)

The easiest way to run Wyldlands is using Docker Compose:

```bash
# Start all services (PostgreSQL, world server, gateway)
docker-compose up --build
```

Once running, connect to:
- **Web Client**: http://localhost:8080
- **Telnet**: `telnet localhost 4000`

For detailed Docker instructions, see [DOCKER.md](DOCKER.md).

### Manual Setup

#### Prerequisites

- Rust 1.75 or later
- PostgreSQL 15 or later
- Git

#### Installation

1. Clone the repository:
```bash
git clone https://github.com/huhlig/wyldlands.git
cd wyldlands
```

2. Set up PostgreSQL database:
```bash
# Create database and user
psql -U postgres -f 001_table_setup.sql

# Or manually:
createdb wyldlands
psql wyldlands < 001_table_setup.sql
```

3. Configure environment:
```bash
# Copy example config
cp gateway/gateway.env.example gateway/gateway.env

# Edit gateway/gateway.env with your database credentials
```

4. Build and run:
```bash
# Build all components
cargo build --release

# Run world server (terminal 1)
cargo run --release --bin server

# Run gateway (terminal 2)
cargo run --release --bin gateway
```

5. Connect:
- Web client: http://localhost:8080
- Telnet: `telnet localhost 4000`

## Project Structure

* `.github` - GitHub Actions Workflows and Issue Templates
* `assets` - Project Assets
* `docs` - Project Documentation
  * `development` - Development plans and status
  * `worldbuilding` - World design documents
* `gateway` - Gateway server (handles client connections)
* `protocol` - Shared protocol definitions
* `server` - World server (game logic and ECS)
* `world` - World data library

## Architecture

Wyldlands uses a distributed architecture:

- **Gateway**: Handles client connections (WebSocket, Telnet), session management, and protocol translation
- **World Server**: Runs game logic using an Entity Component System (ECS)
- **PostgreSQL**: Stores persistent session and world data

Communication between gateway and world server uses tarpc RPC framework.

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --package gateway
cargo test --package server

# Run with database (required for integration tests)
docker-compose up -d postgres
cargo test
```

### Building Documentation

```bash
# Generate API documentation
cargo doc --no-deps --open

# View development documentation
cat docs/development/PROJECT_STATUS.md
```

### Development Status

See [docs/development/PROJECT_STATUS.md](docs/development/PROJECT_STATUS.md) for detailed progress.

**Phase 1 (✅ Complete)**: Core ECS Implementation - 25+ components, 5 systems, event bus
**Phase 2 (✅ Complete)**: Gateway & Connection Persistence - Session mgmt, protocols, RPC
**Phase 3 (📋 Planned)**: GOAP AI System - Goal-oriented action planning
**Phase 4 (📋 Planned)**: LLM Integration - Dynamic NPC dialogue
**Phase 5 (📋 Planned)**: Integration & Polish - Complete gameplay systems

**Overall Progress**: 40% (2 of 5 phases complete)

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

See [docs/development/DEVELOPMENT_PLAN.md](docs/development/DEVELOPMENT_PLAN.md) for roadmap.

## License

This project is licensed under [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as 
defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.

[API Docs]: https://huhlig.github.io/wyldlands/
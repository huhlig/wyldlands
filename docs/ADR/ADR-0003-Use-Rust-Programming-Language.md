---
parent: ADR
nav_order: 0003
title: Use Rust Programming Language
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0003: Use Rust Programming Language

## Context and Problem Statement

We need to select a programming language for building a modern MUD (Multi-User Dimension) server that can handle:
- High concurrency (1000+ simultaneous connections)
- Real-time game logic processing
- Complex state management
- Memory-safe operations
- Low latency requirements
- Long-running server processes

Which programming language should we use for the Wyldlands MUD server?

## Decision Drivers

* **Performance**: Need low-latency command processing and efficient resource usage
* **Concurrency**: Must handle thousands of concurrent player connections
* **Memory Safety**: Long-running server must be stable and free from memory leaks
* **Type Safety**: Complex game logic requires strong compile-time guarantees
* **Ecosystem**: Need mature libraries for networking, databases, and async operations
* **Developer Experience**: Must support rapid development and refactoring
* **Deployment**: Should produce efficient, self-contained binaries

## Considered Options

* Rust
* Go
* C++
* Java/Kotlin
* Python
* Node.js/TypeScript

## Decision Outcome

Chosen option: "Rust", because it provides the best combination of performance, safety, and modern language features for building a high-performance, concurrent game server.

### Positive Consequences

* **Memory Safety**: Ownership system prevents memory leaks, data races, and null pointer errors at compile time
* **Zero-Cost Abstractions**: High-level code compiles to efficient machine code without runtime overhead
* **Fearless Concurrency**: Type system prevents data races, making concurrent code safe by default
* **Rich Ecosystem**: Excellent libraries for async I/O (Tokio), web frameworks (Axum), databases (SQLx), and gRPC (Tonic)
* **Strong Type System**: Enums, pattern matching, and traits enable expressive, type-safe code
* **No Garbage Collection**: Predictable performance without GC pauses
* **Modern Tooling**: Cargo provides excellent dependency management, testing, and documentation
* **Cross-Platform**: Single codebase compiles to Windows, Linux, and macOS

### Negative Consequences

* **Learning Curve**: Ownership and borrowing concepts require time to master
* **Compilation Time**: Rust's compile times are longer than interpreted languages
* **Smaller Talent Pool**: Fewer developers know Rust compared to mainstream languages
* **Async Ecosystem Complexity**: Multiple async runtimes and some ecosystem fragmentation

## Pros and Cons of the Options

### Rust

* Good, because ownership system prevents entire classes of bugs at compile time
* Good, because zero-cost abstractions provide C++-level performance with high-level ergonomics
* Good, because Tokio provides excellent async runtime for handling thousands of connections
* Good, because strong type system catches errors early in development
* Good, because no garbage collector means predictable latency
* Good, because Cargo makes dependency management and testing straightforward
* Neutral, because compilation times can be slow for large projects
* Bad, because ownership concepts have a steep learning curve
* Bad, because smaller ecosystem compared to more established languages

### Go

* Good, because simple language with fast compilation
* Good, because built-in concurrency with goroutines
* Good, because large standard library
* Neutral, because garbage collection provides memory safety but with unpredictable pauses
* Bad, because less expressive type system (no generics until recently)
* Bad, because garbage collection can cause latency spikes in real-time systems
* Bad, because less control over memory layout and performance

### C++

* Good, because maximum performance and control
* Good, because mature ecosystem and large community
* Good, because no garbage collection
* Neutral, because modern C++ (C++17/20) has improved significantly
* Bad, because manual memory management leads to bugs (use-after-free, memory leaks)
* Bad, because undefined behavior is common and hard to debug
* Bad, because complex build systems and dependency management
* Bad, because no built-in async/await or modern concurrency primitives

### Java/Kotlin

* Good, because mature ecosystem and large community
* Good, because strong type system and good tooling
* Good, because excellent libraries for enterprise applications
* Neutral, because JVM provides cross-platform compatibility
* Bad, because garbage collection causes unpredictable latency
* Bad, because higher memory overhead
* Bad, because slower startup times
* Bad, because less control over low-level performance

### Python

* Good, because rapid development and prototyping
* Good, because extensive library ecosystem
* Good, because easy to learn and read
* Bad, because interpreted language with poor performance
* Bad, because Global Interpreter Lock (GIL) limits concurrency
* Bad, because dynamic typing leads to runtime errors
* Bad, because not suitable for high-performance, low-latency systems

### Node.js/TypeScript

* Good, because JavaScript/TypeScript has large developer community
* Good, because async I/O is built into the platform
* Good, because TypeScript adds static typing
* Neutral, because single-threaded event loop model
* Bad, because V8 garbage collection causes latency spikes
* Bad, because callback-heavy code can be hard to maintain
* Bad, because less suitable for CPU-intensive game logic
* Bad, because weaker type system compared to Rust

## Validation

The decision is validated by:
1. **Performance Benchmarks**: Rust consistently achieves C++-level performance in game server benchmarks
2. **Production Use**: Major game companies (e.g., Embark Studios) use Rust for game servers
3. **Ecosystem Maturity**: All required libraries (Tokio, Axum, SQLx, Tonic, Hecs) are production-ready
4. **Project Success**: Wyldlands has successfully implemented 85% of planned features with excellent stability

## More Information

### Key Dependencies

The Rust ecosystem provides excellent libraries for all project requirements:

- **Async Runtime**: Tokio - Industry-standard async runtime with excellent performance
- **Web Framework**: Axum - Modern, ergonomic web framework built on Tokio
- **Database**: SQLx - Compile-time checked SQL queries with async support
- **gRPC**: Tonic - Full-featured gRPC implementation
- **ECS**: Hecs - Fast, flexible Entity Component System
- **Serialization**: Serde - Zero-cost serialization framework

### Performance Characteristics

Achieved performance metrics:
- Session creation: <1ms
- Message routing: <0.1ms
- ECS system updates: <1ms for 1,000 entities
- Concurrent capacity: 10,000+ connections
- Memory usage: ~200-500 bytes per entity

### Related Decisions

- [ADR-0004](ADR-0004-Use-Entity-Component-System.md) - ECS architecture choice
- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Distributed architecture
- [ADR-0007](ADR-0007-Use-gRPC-for-Inter-Service-Communication.md) - gRPC protocol choice

### References

- [Rust Programming Language](https://www.rust-lang.org/)
- [Tokio Async Runtime](https://tokio.rs/)
- [Rust in Production: Game Servers](https://embark.dev/)
- Project Status: [docs/development/PROJECT_STATUS.md](../development/PROJECT_STATUS.md)
---
parent: ADR
nav_order: 0016
title: Testing Strategy and Coverage Requirements
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0016: Testing Strategy and Coverage Requirements

## Context and Problem Statement

A complex MUD server requires comprehensive testing to ensure reliability. We need:
- Unit tests for individual components
- Integration tests for system interactions
- Performance benchmarks
- Clear coverage targets
- Fast test execution
- Maintainable test code

How should we structure our testing strategy to ensure quality while maintaining development velocity?

## Decision Drivers

* **Quality Assurance**: Catch bugs before production
* **Development Speed**: Fast test execution
* **Maintainability**: Tests should be easy to update
* **Coverage**: High code coverage without diminishing returns
* **Confidence**: Tests should give confidence in changes
* **Documentation**: Tests document expected behavior

## Considered Options

* Comprehensive Testing Strategy (Unit + Integration + Benchmarks)
* Unit Tests Only
* Integration Tests Only
* Property-Based Testing Focus

## Decision Outcome

Chosen option: "Comprehensive Testing Strategy", because it provides the best balance of coverage, confidence, and maintainability.

### Testing Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Test Pyramid                           │
│                                                          │
│                    ▲                                     │
│                   ╱ ╲                                    │
│                  ╱   ╲  E2E Tests (Manual)              │
│                 ╱─────╲                                  │
│                ╱       ╲                                 │
│               ╱         ╲ Integration Tests (60+)       │
│              ╱───────────╲                               │
│             ╱             ╲                              │
│            ╱               ╲ Unit Tests (230+)          │
│           ╱─────────────────╲                            │
│          ╱                   ╲                           │
│         ╱                     ╲ Benchmarks (8)          │
│        ╱───────────────────────╲                         │
└─────────────────────────────────────────────────────────┘
```

### Test Distribution

**Total: 293 tests**
- Gateway Tests: 70+
- Server Tests: 145+
- Common Tests: Protocol validation
- Integration Tests: 60+
- Benchmarks: 8 categories

### Coverage Targets

- **Overall**: 90%+ code coverage
- **Critical Paths**: 100% coverage (auth, persistence, combat)
- **Business Logic**: 95%+ coverage
- **UI/Protocol**: 80%+ coverage

### Positive Consequences

* **High Confidence**: Comprehensive coverage catches bugs
* **Fast Feedback**: Unit tests run in seconds
* **Regression Prevention**: Tests prevent breaking changes
* **Documentation**: Tests document expected behavior
* **Refactoring Safety**: Can refactor with confidence

### Negative Consequences

* **Maintenance**: Tests require updates when code changes
* **Execution Time**: Full test suite takes time
* **Complexity**: Managing test infrastructure

## Implementation Details

### Unit Tests

**Location**: Alongside source code in `#[cfg(test)]` modules

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_builder_point_allocation() {
        let mut builder = CharacterBuilder::new("Test", 100, 50);
        assert_eq!(builder.attribute_talent_points, 100);
        
        builder.allocate_attribute(AttributeType::BodyOffense, 15).unwrap();
        assert_eq!(builder.attribute_talent_points, 95);
    }
}
```

### Integration Tests

**Location**: `gateway/tests/`, `server/tests/`

**Categories:**
1. **Character Creation**: Full character creation flow
2. **Combat System**: Combat mechanics and status effects
3. **NPC System**: NPC AI and dialogue
4. **Memory System**: NPC memory and relationships
5. **Builder Commands**: World building functionality

**Example:**
```rust
#[tokio::test]
async fn test_character_creation_flow() {
    let context = setup_test_context().await;
    
    // Create character
    let char_id = create_test_character(&context, "TestChar").await;
    
    // Verify attributes
    let attributes = get_character_attributes(&context, char_id).await;
    assert_eq!(attributes.body.offense, 10);
    
    // Verify persistence
    let loaded = load_character(&context, char_id).await;
    assert_eq!(loaded.name, "TestChar");
}
```

### Performance Benchmarks

**Location**: `gateway/benches/`, `server/benches/`

**Categories:**
1. Session Management
2. Connection Pool
3. ECS Systems
4. Memory Operations
5. Database Queries
6. Protocol Handling
7. AI Planning
8. LLM Requests

**Example:**
```rust
fn bench_session_creation(c: &mut Criterion) {
    c.bench_function("session_creation", |b| {
        b.iter(|| {
            Session::new(black_box(Uuid::new_v4()))
        });
    });
}
```

### Test Utilities

**Location**: `server/src/ecs/test_utils.rs`

**Utilities:**
- `create_test_world()`: Create test ECS world
- `create_test_entity()`: Create test entity with components
- `create_test_character()`: Create test character
- `setup_test_context()`: Setup full test context

### Testing Commands

```bash
# Run all tests
cargo test

# Run specific package tests
cargo test -p wyldlands-gateway
cargo test -p wyldlands-server

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_character_creation

# Run benchmarks
cargo bench

# Run with coverage (using cargo-tarpaulin)
cargo tarpaulin --out Html
```

## Validation

The testing strategy is validated by:

1. **Coverage Reports**: Regular coverage analysis
2. **CI/CD**: Automated test execution on every commit
3. **Bug Detection**: Tests catch regressions
4. **Performance Tracking**: Benchmark trends over time
5. **Code Review**: Test quality reviewed in PRs

## More Information

### Test Organization

**Gateway Tests:**
- Session management (20+ tests)
- Connection pool (15+ tests)
- Protocol adapters (20+ tests)
- Authentication (15+ tests)

**Server Tests:**
- ECS components (40+ tests)
- ECS systems (30+ tests)
- Command system (40+ tests)
- Persistence (20+ tests)
- AI systems (15+ tests)

### CI/CD Integration

```yaml
# .github/workflows/rust.yml
name: Rust CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run tests
        run: cargo test --all
      - name: Run benchmarks
        run: cargo bench --no-run
```

### Test Data Management

**Fixtures:**
- Test accounts
- Test characters
- Test world data
- Test configurations

**Cleanup:**
- Tests use isolated databases
- Automatic cleanup after tests
- No shared state between tests

### Future Enhancements

1. **Property-Based Testing**: Use `proptest` for edge cases
2. **Mutation Testing**: Verify test quality with `cargo-mutants`
3. **Fuzz Testing**: Find edge cases with fuzzing
4. **Load Testing**: Simulate high player counts
5. **Chaos Engineering**: Test failure scenarios

### Related Decisions

- [ADR-0003](ADR-0003-Use-Rust-Programming-Language.md) - Rust enables strong testing
- [ADR-0010](ADR-0010-Cargo-Workspace-Structure.md) - Workspace enables independent testing

### References

- Gateway Tests: [gateway/tests/](../../gateway/tests/)
- Server Tests: [server/tests/](../../server/tests/)
- Test Utilities: [server/src/ecs/test_utils.rs](../../server/src/ecs/test_utils.rs)
- Benchmarks: [server/benches/](../../server/benches/)
- CI Configuration: [.github/workflows/](../../.github/workflows/)
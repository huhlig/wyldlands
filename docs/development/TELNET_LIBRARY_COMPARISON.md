# Telnet Library Comparison for Wyldlands Gateway

**Date**: December 18, 2025  
**Purpose**: Evaluate telnet libraries for Phase 2 implementation

---

## Libraries Evaluated

### 1. termionix (https://github.com/huhlig/termionix/)

**Pros:**
- ✅ **Custom/In-house**: Maintained by project owner (huhlig)
- ✅ **Tailored**: Can be customized for Wyldlands-specific needs
- ✅ **Direct control**: Full control over features and bug fixes
- ✅ **Integration**: Likely designed with this project in mind
- ✅ **Modern Rust**: Built with current Rust best practices

**Cons:**
- ⚠️ **Maturity**: May be less battle-tested than established libraries
- ⚠️ **Documentation**: May have less community documentation
- ⚠️ **Community**: Smaller user base for support

**Features to Verify:**
- Telnet protocol negotiation
- MCCP (MUD Client Compression Protocol)
- MSDP (MUD Server Data Protocol)
- GMCP (Generic MUD Communication Protocol)
- NAWS (Negotiate About Window Size)
- ANSI color support
- Async/await support

### 2. libtelnet-rs

**Pros:**
- ✅ **Established**: Port of well-known libtelnet C library
- ✅ **Feature-complete**: Comprehensive telnet protocol support
- ✅ **Battle-tested**: Used in production systems
- ✅ **Documentation**: Good API documentation

**Cons:**
- ⚠️ **Maintenance**: May have slower update cycle
- ⚠️ **C heritage**: API design influenced by C library
- ⚠️ **Dependencies**: May have more dependencies

**Crate**: `libtelnet-rs = "0.2"`

### 3. nectar

**Pros:**
- ✅ **Pure Rust**: Native Rust implementation
- ✅ **Modern API**: Idiomatic Rust design
- ✅ **Lightweight**: Minimal dependencies

**Cons:**
- ⚠️ **Less mature**: Newer library
- ⚠️ **Feature set**: May not support all MUD-specific protocols
- ⚠️ **Documentation**: Limited examples

---

## Recommendation: termionix

### Rationale

Given that **termionix** is maintained by the project owner (huhlig), it is the **recommended choice** for the following reasons:

1. **Custom Integration**: Can be tailored specifically for Wyldlands' needs
2. **Direct Support**: Issues can be addressed directly by the project team
3. **Consistency**: Likely shares design philosophy with the rest of the project
4. **Control**: Full control over features, bug fixes, and updates
5. **Modern Design**: Built with current Rust async patterns in mind

### Implementation Plan

#### Phase 1: Dependency Integration
```toml
[dependencies]
termionix = { git = "https://github.com/huhlig/termionix", branch = "main" }
```

#### Phase 2: Feature Verification
- [ ] Verify async/await support
- [ ] Test telnet protocol negotiation
- [ ] Validate MCCP support
- [ ] Check MSDP/GMCP capabilities
- [ ] Test NAWS implementation
- [ ] Verify ANSI color handling

#### Phase 3: Adapter Implementation
- [ ] Create TelnetConnection wrapper
- [ ] Implement protocol negotiation handler
- [ ] Add option handling (MCCP, MSDP, GMCP, NAWS)
- [ ] Integrate with ConnectionPool
- [ ] Add session management hooks

---

## Fallback Options

If termionix doesn't meet requirements:

1. **First Fallback**: libtelnet-rs
   - Most feature-complete
   - Well-tested in production
   - Good documentation

2. **Second Fallback**: nectar
   - Pure Rust
   - Modern API
   - May need custom protocol extensions

---

## Testing Strategy

### Unit Tests
- Protocol negotiation sequences
- Option handling (DO, DONT, WILL, WONT)
- Subnegotiation parsing
- ANSI escape sequence handling

### Integration Tests
- Full connection lifecycle
- Multiple concurrent connections
- Protocol feature negotiation
- Compression (MCCP) functionality
- Data protocol (MSDP/GMCP) communication

### Compatibility Tests
- Test with popular MUD clients:
  - MUSHclient
  - Mudlet
  - TinTin++
  - SimpleMU
  - Raw telnet

---

## Performance Considerations

### Benchmarks Needed
- Connection establishment time
- Protocol negotiation overhead
- Compression ratio (MCCP)
- Throughput with/without compression
- Memory usage per connection

### Targets
- Connection setup: < 100ms
- Negotiation: < 50ms
- Throughput: > 1MB/s per connection
- Memory: < 10KB per connection (base)

---

## Next Steps

1. ✅ **Review termionix repository**
   - Check API documentation
   - Review examples
   - Verify feature support

2. **Create prototype integration**
   - Add dependency to Cargo.toml
   - Create basic TelnetConnection wrapper
   - Test with simple echo server

3. **Implement full integration**
   - Complete TelnetConnection implementation
   - Add protocol negotiation
   - Integrate with ConnectionPool
   - Add comprehensive tests

4. **Performance validation**
   - Run benchmarks
   - Test with multiple clients
   - Verify memory usage
   - Validate throughput

---

**Decision**: Proceed with **termionix** as primary telnet library

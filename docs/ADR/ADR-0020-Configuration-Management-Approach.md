---
parent: ADR
nav_order: 0020
title: Configuration Management Approach
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0020: Configuration Management Approach

## Context and Problem Statement

A distributed MUD server requires flexible configuration for:
- Database connections
- Network settings
- LLM providers
- Game parameters
- Logging levels
- Feature flags

How should we manage configuration to support development, testing, and production environments?

## Decision Drivers

* **Environment Flexibility**: Different configs for dev/test/prod
* **Security**: Sensitive data (API keys, passwords) protected
* **Ease of Use**: Simple configuration format
* **Validation**: Catch configuration errors early
* **Documentation**: Self-documenting configuration
* **Overrides**: Environment variables override file settings

## Considered Options

* YAML Configuration with Environment Variable Overrides
* TOML Configuration
* JSON Configuration
* Environment Variables Only

## Decision Outcome

Chosen option: "YAML Configuration with Environment Variable Overrides", because it provides the best balance of readability, flexibility, and security.

### Configuration Architecture

```
┌─────────────────────────────────────────────────────────┐
│            Configuration Loading                         │
│                                                          │
│  1. Load config.yaml (defaults)                         │
│  2. Override with environment variables                  │
│  3. Validate configuration                               │
│  4. Initialize services                                  │
└─────────────────────────────────────────────────────────┘
```

### Configuration Files

**Gateway Configuration (gateway/config.yaml):**
```yaml
server:
  telnet_port: 4000
  websocket_port: 8080
  admin_port: 9000

database:
  host: localhost
  port: 5432
  database: wyldlands
  username: wyldlands
  password: ${DATABASE_PASSWORD}
  max_connections: 10

rpc:
  server_address: "http://localhost:50051"
  timeout_seconds: 30
  max_retries: 3

session:
  reconnect_ttl_seconds: 3600
  max_command_queue_size: 100
  session_timeout_seconds: 7200

logging:
  level: info
  format: json
```

**Server Configuration (server/config.yaml):**
```yaml
server:
  rpc_port: 50051
  max_connections: 1000

database:
  host: localhost
  port: 5432
  database: wyldlands
  username: wyldlands
  password: ${DATABASE_PASSWORD}
  max_connections: 20

llm:
  default_provider: "openai"
  providers:
    openai:
      provider: "openai"
      api_key: "${OPENAI_API_KEY}"
      model: "gpt-4"
      endpoint: "https://api.openai.com/v1/chat/completions"
      timeout_seconds: 30
    ollama:
      provider: "ollama"
      model: "llama2"
      endpoint: "http://localhost:11434/api/chat"
      timeout_seconds: 60

character_creation:
  max_attribute_talent_points: 100
  max_skill_points: 50
  min_attribute_rank: 10
  max_attribute_rank: 20

logging:
  level: debug
  format: pretty
```

### Positive Consequences

* **Readable**: YAML is human-friendly
* **Flexible**: Environment variables for sensitive data
* **Documented**: Configuration is self-documenting
* **Validated**: Type-safe configuration structs
* **Secure**: Secrets not in version control
* **Environment-Specific**: Easy to have different configs

### Negative Consequences

* **YAML Complexity**: YAML can be tricky (indentation, types)
* **Validation Timing**: Errors only caught at startup
* **Documentation**: Must keep docs in sync with code

## Implementation Details

### Configuration Structs

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub llm: Option<LlmConfig>,
    pub character_creation: CharacterCreationConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub rpc_port: u16,
    pub max_connections: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
}
```

### Configuration Loading

```rust
use config::{Config as ConfigBuilder, Environment, File};

pub fn load_config() -> Result<Config> {
    let config = ConfigBuilder::builder()
        // Load from file
        .add_source(File::with_name("config"))
        // Override with environment variables
        .add_source(Environment::with_prefix("WYLDLANDS"))
        .build()?;
    
    // Deserialize into struct
    let config: Config = config.try_deserialize()?;
    
    // Validate
    config.validate()?;
    
    Ok(config)
}
```

### Environment Variable Overrides

**Naming Convention:**
```
WYLDLANDS_<SECTION>_<KEY>

Examples:
WYLDLANDS_DATABASE_PASSWORD=secret123
WYLDLANDS_LLM_PROVIDERS_OPENAI_API_KEY=sk-...
WYLDLANDS_SERVER_RPC_PORT=50052
```

### Validation

```rust
impl Config {
    pub fn validate(&self) -> Result<()> {
        // Validate ports
        if self.server.rpc_port == 0 {
            return Err("Invalid RPC port".into());
        }
        
        // Validate database config
        if self.database.max_connections == 0 {
            return Err("Max connections must be > 0".into());
        }
        
        // Validate LLM config
        if let Some(llm) = &self.llm {
            llm.validate()?;
        }
        
        Ok(())
    }
}
```

### Secrets Management

**Development:**
```bash
# .env file (not in version control)
DATABASE_PASSWORD=dev_password
OPENAI_API_KEY=sk-dev-key
```

**Production:**
```bash
# Environment variables from secrets manager
export WYLDLANDS_DATABASE_PASSWORD=$(aws secretsmanager get-secret-value ...)
export WYLDLANDS_LLM_PROVIDERS_OPENAI_API_KEY=$(aws secretsmanager get-secret-value ...)
```

### Docker Configuration

**docker-compose.yml:**
```yaml
services:
  server:
    image: wyldlands-server
    environment:
      - WYLDLANDS_DATABASE_PASSWORD=${DATABASE_PASSWORD}
      - WYLDLANDS_LLM_PROVIDERS_OPENAI_API_KEY=${OPENAI_API_KEY}
    volumes:
      - ./server/config.yaml:/app/config.yaml:ro
```

## Validation

Configuration management is validated by:

1. **Schema Validation**: Type-safe configuration structs
2. **Startup Tests**: Test configuration loading
3. **Environment Tests**: Test environment variable overrides
4. **Documentation**: Configuration documented in README
5. **Examples**: Example configurations provided

## More Information

### Configuration Best Practices

1. **Defaults**: Provide sensible defaults
2. **Documentation**: Comment configuration options
3. **Validation**: Validate at startup
4. **Secrets**: Never commit secrets
5. **Environment-Specific**: Use environment variables for differences

### Configuration Files

**Development:**
- `config.yaml`: Default development settings
- `.env`: Local secrets (gitignored)

**Testing:**
- `config.test.yaml`: Test-specific settings
- `.env.test`: Test secrets

**Production:**
- `config.yaml`: Production defaults
- Environment variables: Production secrets

### Future Enhancements

1. **Hot Reload**: Reload configuration without restart
2. **Remote Configuration**: Load from configuration service
3. **Feature Flags**: Dynamic feature toggling
4. **Configuration UI**: Web interface for configuration
5. **Configuration Versioning**: Track configuration changes

### Related Decisions

- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - Database configuration
- [ADR-0013](ADR-0013-LLM-Integration-Architecture.md) - LLM provider configuration
- [ADR-0021](ADR-0021-Docker-Deployment-Architecture.md) - Docker configuration

### References

- Gateway Config: [gateway/config.yaml](../../gateway/config.yaml)
- Server Config: [server/config.yaml](../../server/config.yaml)
- Configuration Guide: [docs/CONFIGURATION.md](../CONFIGURATION.md)
- Environment Variables: [.env.example](../../.env.example)
# Configuration Guide

This document explains how to configure the Wyldlands gateway and server using environment variables and configuration files.

## Configuration System

The system supports three levels of configuration (in order of precedence):

1. **Environment Variables** - Highest priority, overrides everything
2. **Environment Files** (.env) - Loaded before config files
3. **Configuration Files** (YAML) - Base configuration with variable substitution

## Environment Variable Substitution

Configuration files support environment variable substitution using the syntax:

```yaml
# Use environment variable or fail if not set
database_url: ${DATABASE_URL}

# Use environment variable or default value
port: ${PORT:-8080}
```

## Gateway Configuration

### Configuration File: `gateway.config.yaml`

```yaml
server:
  addr: ${SERVER_RPC_ADDR:-127.0.0.1}
  port: ${SERVER_RPC_PORT:-6006}

database:
  url: ${DATABASE_URL:-postgres://postgres:postgres@localhost/wyldlands}
  username: ${DATABASE_USER:-postgres}
  password: ${DATABASE_PASSWORD:-postgres}

websocket:
  addr: ${WEBSOCKET_ADDR:-0.0.0.0}
  port: ${WEBSOCKET_PORT:-8080}

telnet:
  addr: ${TELNET_ADDR:-0.0.0.0}
  port: ${TELNET_PORT:-4000}
```

### Environment File: `gateway.env`

```bash
# Database connection
DATABASE_URL=postgres://postgres:postgres@localhost/wyldlands

# Network bindings
TELNET_BINDING=0.0.0.0:4000
WEBSOCKET_BINDING=0.0.0.0:8080
SERVER_RPC_ADDR=127.0.0.1:6006

# Logging
RUST_LOG=info
```

### Command Line Options

```bash
# Start gateway with custom config and env files
wyldlands-gateway --config custom.yaml --env custom.env

# Enable specific protocols
wyldlands-gateway --websocket --telnet

# Help
wyldlands-gateway --help
```

### Environment Variable Overrides

The gateway supports these environment variable overrides:

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@host/db` |
| `TELNET_BINDING` | Telnet server address:port | `0.0.0.0:4000` |
| `WEBSOCKET_BINDING` | WebSocket server address:port | `0.0.0.0:8080` |
| `SERVER_RPC_ADDR` | World server RPC address:port | `127.0.0.1:6006` |
| `RUST_LOG` | Logging level | `debug`, `info`, `warn`, `error` |

## Server Configuration

### Configuration File: `server.config.yaml`

```yaml
database:
  url: ${DATABASE_URL:-postgres://postgres:postgres@localhost/wyldlands}
  username: ${DATABASE_USER:-postgres}
  password: ${DATABASE_PASSWORD:-postgres}

rpc:
  addr: ${RPC_ADDR:-127.0.0.1}
  port: ${RPC_PORT:-6006}
```

### Environment File: `server.env`

```bash
# Database connection
DATABASE_URL=postgres://postgres:postgres@localhost/wyldlands

# RPC server binding
RPC_BINDING=0.0.0.0:6006

# Logging
RUST_LOG=info
```

### Environment Variable Overrides

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@host/db` |
| `SERVER_RPC_ADDR` or `RPC_BINDING` | RPC server address:port | `0.0.0.0:6006` |
| `RUST_LOG` | Logging level | `debug`, `info`, `warn`, `error` |

## Docker Configuration

When running in Docker, use environment variables:

```yaml
# docker-compose.yml
services:
  gateway:
    environment:
      - DATABASE_URL=postgres://postgres:postgres@db:5432/wyldlands
      - WEBSOCKET_BINDING=0.0.0.0:8080
      - TELNET_BINDING=0.0.0.0:4000
      - SERVER_RPC_ADDR=server:6006
      - RUST_LOG=info
  
  server:
    environment:
      - DATABASE_URL=postgres://postgres:postgres@db:5432/wyldlands
      - RPC_BINDING=0.0.0.0:6006
      - RUST_LOG=info
```

## Production Best Practices

1. **Use Environment Files**: Keep sensitive data in `.env` files (not in git)
2. **Set Proper Logging**: Use `RUST_LOG=info` in production, `debug` for development
3. **Secure Database**: Use strong passwords and SSL connections
4. **Network Binding**: Bind to `0.0.0.0` in containers, `127.0.0.1` for local-only
5. **Configuration Validation**: Test configuration changes in staging first

## Examples

### Development Setup

```bash
# gateway.env
DATABASE_URL=postgres://postgres:postgres@localhost/wyldlands_dev
WEBSOCKET_BINDING=127.0.0.1:8080
TELNET_BINDING=127.0.0.1:4000
SERVER_RPC_ADDR=127.0.0.1:6006
RUST_LOG=debug
```

### Production Setup

```bash
# gateway.env (use secrets management in real production)
DATABASE_URL=postgres://wyldlands:${DB_PASSWORD}@db.example.com:5432/wyldlands
WEBSOCKET_BINDING=0.0.0.0:8080
TELNET_BINDING=0.0.0.0:4000
SERVER_RPC_ADDR=server.internal:6006
RUST_LOG=info
```

### Testing Setup

```bash
# gateway/.env.test
DATABASE_URL=postgres://postgres:postgres@localhost/wyldlands_test
WEBSOCKET_BINDING=127.0.0.1:18080
TELNET_BINDING=127.0.0.1:14000
SERVER_RPC_ADDR=127.0.0.1:16006
RUST_LOG=debug
```

## Troubleshooting

### Configuration Not Loading

1. Check file paths are correct
2. Verify environment file exists and is readable
3. Check for YAML syntax errors
4. Enable debug logging: `RUST_LOG=debug`

### Environment Variables Not Working

1. Ensure variables are exported: `export VAR_NAME=value`
2. Check variable names match exactly (case-sensitive)
3. Verify .env file is being loaded
4. Use `--env` flag to specify custom env file

### Connection Issues

1. Verify database URL is correct
2. Check network bindings (0.0.0.0 vs 127.0.0.1)
3. Ensure ports are not already in use
4. Check firewall rules

## Made with Bob
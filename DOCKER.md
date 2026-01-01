# Wyldlands Docker Setup

This document describes how to run Wyldlands using Docker and Docker Compose.

## Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+

## Quick Start

### 1. Build and Start All Services

```bash
docker-compose up --build
```

This will:
- Start PostgreSQL database on port 5432
- Initialize the database with the schema from `001_table_setup.sql`
- Build and start the World Server
- Build and start the Gateway on ports 8080 (WebSocket/HTTP) and 4000 (Telnet)

### 2. Access the Services

- **Web Client**: http://localhost:8080
- **WebSocket**: ws://localhost:8080/ws
- **Telnet**: telnet localhost 4000
- **PostgreSQL**: localhost:5432 (user: postgres, password: postgres, database: wyldlands)

### 3. Stop All Services

```bash
docker-compose down
```

To also remove the database volume:

```bash
docker-compose down -v
```

## Individual Service Management

### Start Only Database

```bash
docker-compose up postgres
```

### Start Database and World Server

```bash
docker-compose up postgres worldserver
```

### Rebuild a Specific Service

```bash
docker-compose build gateway
docker-compose up gateway
```

## Development Workflow

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f gateway
docker-compose logs -f worldserver
docker-compose logs -f postgres
```

### Execute Commands in Containers

```bash
# Access gateway shell
docker-compose exec gateway /bin/bash

# Access world server shell
docker-compose exec worldserver /bin/bash

# Access PostgreSQL
docker-compose exec postgres psql -U postgres -d wyldlands
```

### Restart a Service

```bash
docker-compose restart gateway
docker-compose restart worldserver
```

## Database Management

### Connect to PostgreSQL

```bash
docker-compose exec postgres psql -U postgres -d wyldlands
```

### Run SQL Scripts

```bash
docker-compose exec -T postgres psql -U postgres -d wyldlands < your-script.sql
```

### Backup Database

```bash
docker-compose exec -T postgres pg_dump -U postgres wyldlands > backup.sql
```

### Restore Database

```bash
docker-compose exec -T postgres psql -U postgres -d wyldlands < backup.sql
```

## Configuration

### Environment Variables

You can override environment variables in `docker-compose.yml` or create a `.env` file:

```env
# Database
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
POSTGRES_DB=wyldlands

# Gateway
WYLDLANDS_BINDING=0.0.0.0:8080
RUST_LOG=debug

# Ports
GATEWAY_HTTP_PORT=8080
GATEWAY_TELNET_PORT=4000
POSTGRES_PORT=5432
```

### Custom Configuration

To use a custom `config.yaml`, mount it as a volume in `docker-compose.yml`:

```yaml
services:
  gateway:
    volumes:
      - ./my-config.yaml:/app/config.yaml
```

## Troubleshooting

### Database Connection Issues

If services can't connect to the database:

1. Check if PostgreSQL is healthy:
   ```bash
   docker-compose ps postgres
   ```

2. Check PostgreSQL logs:
   ```bash
   docker-compose logs postgres
   ```

3. Verify the database is accepting connections:
   ```bash
   docker-compose exec postgres pg_isready -U postgres
   ```

### Build Issues

If builds fail:

1. Clean Docker build cache:
   ```bash
   docker-compose build --no-cache
   ```

2. Remove old images:
   ```bash
   docker-compose down --rmi all
   ```

3. Prune Docker system:
   ```bash
   docker system prune -a
   ```

### Port Conflicts

If ports are already in use, modify the port mappings in `docker-compose.yml`:

```yaml
ports:
  - "8081:8080"  # Use 8081 instead of 8080
  - "4001:4000"  # Use 4001 instead of 4000
```

## Production Deployment

For production deployment:

1. Use environment-specific compose files:
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.prod.yml up
   ```

2. Set secure passwords in environment variables

3. Use Docker secrets for sensitive data

4. Configure proper logging and monitoring

5. Set up health checks and restart policies

6. Use a reverse proxy (nginx, traefik) for SSL/TLS

## Performance Tuning

### PostgreSQL

Adjust PostgreSQL settings in `docker-compose.yml`:

```yaml
postgres:
  command:
    - "postgres"
    - "-c"
    - "max_connections=200"
    - "-c"
    - "shared_buffers=256MB"
```

### Rust Services

Adjust resource limits:

```yaml
gateway:
  deploy:
    resources:
      limits:
        cpus: '2'
        memory: 1G
      reservations:
        cpus: '1'
        memory: 512M
```

## Testing

### Run Integration Tests

```bash
# Start database
docker-compose up -d postgres

# Wait for database to be ready
docker-compose exec postgres pg_isready -U postgres

# Run tests
docker-compose run --rm gateway cargo test
docker-compose run --rm worldserver cargo test
```

## Monitoring

### Health Checks

Check service health:

```bash
docker-compose ps
```

### Resource Usage

Monitor resource usage:

```bash
docker stats
```

## Cleanup

### Remove Everything

```bash
# Stop and remove containers, networks, volumes
docker-compose down -v

# Remove images
docker-compose down --rmi all

# Full cleanup
docker system prune -a --volumes
```

## Support

For issues and questions:
- Check the main README.md
- Review logs: `docker-compose logs`
- Open an issue on GitHub
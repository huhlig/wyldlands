---
parent: ADR
nav_order: 0021
title: Docker Deployment Architecture
status: accepted
date: 2026-02-01
deciders: Hans W. Uhlig
---

# ADR-0021: Docker Deployment Architecture

## Context and Problem Statement

The MUD server requires deployment infrastructure that:
- Supports development, testing, and production
- Manages multiple services (gateway, server, database)
- Handles service dependencies
- Enables easy scaling
- Provides consistent environments
- Simplifies deployment

How should we containerize and orchestrate the application for reliable deployment?

## Decision Drivers

* **Consistency**: Same environment across dev/test/prod
* **Isolation**: Services run in isolated containers
* **Scalability**: Easy to scale services
* **Simplicity**: Easy to deploy and manage
* **Development**: Fast local development setup
* **Production**: Production-ready deployment

## Considered Options

* Docker Compose for Orchestration
* Kubernetes
* Docker Swarm
* Manual Deployment

## Decision Outcome

Chosen option: "Docker Compose for Orchestration", because it provides the right balance of simplicity and functionality for a MUD server while enabling future migration to Kubernetes if needed.

### Docker Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Docker Compose Stack                        │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Gateway    │  │    Server    │  │  PostgreSQL  │ │
│  │  Container   │  │  Container   │  │  Container   │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
│         │                  │                  │         │
│         │ gRPC            │ SQL              │         │
│         └──────────────────┼──────────────────┘         │
│                            │                            │
│  ┌─────────────────────────────────────────────────┐  │
│  │          Shared Network                         │  │
│  └─────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Service Configuration

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    container_name: wyldlands-postgres
    environment:
      POSTGRES_DB: wyldlands
      POSTGRES_USER: wyldlands
      POSTGRES_PASSWORD: ${DATABASE_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d:ro
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U wyldlands"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - wyldlands

  server:
    build:
      context: .
      dockerfile: server/Dockerfile
    container_name: wyldlands-server
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      - WYLDLANDS_DATABASE_HOST=postgres
      - WYLDLANDS_DATABASE_PASSWORD=${DATABASE_PASSWORD}
      - WYLDLANDS_LLM_PROVIDERS_OPENAI_API_KEY=${OPENAI_API_KEY}
    volumes:
      - ./server/config.yaml:/app/config.yaml:ro
    ports:
      - "50051:50051"
    networks:
      - wyldlands
    restart: unless-stopped

  gateway:
    build:
      context: .
      dockerfile: gateway/Dockerfile
    container_name: wyldlands-gateway
    depends_on:
      - server
    environment:
      - WYLDLANDS_DATABASE_HOST=postgres
      - WYLDLANDS_DATABASE_PASSWORD=${DATABASE_PASSWORD}
      - WYLDLANDS_RPC_SERVER_ADDRESS=http://server:50051
    volumes:
      - ./gateway/config.yaml:/app/config.yaml:ro
    ports:
      - "4000:4000"   # Telnet
      - "8080:8080"   # WebSocket
      - "9000:9000"   # Admin API
    networks:
      - wyldlands
    restart: unless-stopped

volumes:
  postgres_data:

networks:
  wyldlands:
    driver: bridge
```

### Positive Consequences

* **Easy Setup**: `docker-compose up` starts everything
* **Consistent**: Same environment everywhere
* **Isolated**: Services don't interfere
* **Scalable**: Can scale services independently
* **Portable**: Works on any Docker host
* **Development**: Fast local development

### Negative Consequences

* **Resource Usage**: Containers use more resources than native
* **Complexity**: Adds Docker layer
* **Learning Curve**: Developers must understand Docker

## Implementation Details

### Multi-Stage Dockerfiles

**Server Dockerfile:**
```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY server ./server
RUN cargo build --release --bin server

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY server/config.yaml /app/config.yaml
WORKDIR /app
EXPOSE 50051
CMD ["server"]
```

**Gateway Dockerfile:**
```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY gateway ./gateway
RUN cargo build --release --bin gateway

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/gateway /usr/local/bin/gateway
COPY gateway/config.yaml /app/config.yaml
WORKDIR /app
EXPOSE 4000 8080 9000
CMD ["gateway"]
```

### Development Workflow

**Start services:**
```bash
docker-compose up --build
```

**Stop services:**
```bash
docker-compose down
```

**View logs:**
```bash
docker-compose logs -f server
docker-compose logs -f gateway
```

**Rebuild specific service:**
```bash
docker-compose up --build server
```

**Run tests:**
```bash
docker-compose run --rm server cargo test
```

### Production Deployment

**Environment Variables (.env):**
```bash
DATABASE_PASSWORD=secure_production_password
OPENAI_API_KEY=sk-prod-key
```

**Deploy:**
```bash
# Pull latest images
docker-compose pull

# Start services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f
```

### Health Checks

**PostgreSQL:**
```yaml
healthcheck:
  test: ["CMD-SHELL", "pg_isready -U wyldlands"]
  interval: 10s
  timeout: 5s
  retries: 5
```

**Server:**
```yaml
healthcheck:
  test: ["CMD", "grpc_health_probe", "-addr=:50051"]
  interval: 30s
  timeout: 10s
  retries: 3
```

**Gateway:**
```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:9000/health"]
  interval: 30s
  timeout: 10s
  retries: 3
```

### Volumes and Persistence

**Database Data:**
```yaml
volumes:
  postgres_data:
    driver: local
```

**Configuration:**
```yaml
volumes:
  - ./server/config.yaml:/app/config.yaml:ro
  - ./gateway/config.yaml:/app/config.yaml:ro
```

**Migrations:**
```yaml
volumes:
  - ./migrations:/docker-entrypoint-initdb.d:ro
```

## Validation

Docker deployment is validated by:

1. **Build Tests**: Dockerfiles build successfully
2. **Integration Tests**: Services communicate correctly
3. **Health Checks**: All services report healthy
4. **Deployment Tests**: Deploy to staging environment
5. **Performance Tests**: Measure container overhead

## More Information

### Makefile Commands

```makefile
.PHONY: up down logs build test clean

up:
	docker-compose up -d

down:
	docker-compose down

logs:
	docker-compose logs -f

build:
	docker-compose build

test:
	docker-compose run --rm server cargo test

clean:
	docker-compose down -v
	docker system prune -f
```

### Scaling Services

**Scale gateway:**
```bash
docker-compose up -d --scale gateway=3
```

**Load balancer configuration:**
```yaml
nginx:
  image: nginx:alpine
  volumes:
    - ./nginx.conf:/etc/nginx/nginx.conf:ro
  ports:
    - "80:80"
  depends_on:
    - gateway
```

### Monitoring

**Prometheus + Grafana:**
```yaml
prometheus:
  image: prom/prometheus
  volumes:
    - ./prometheus.yml:/etc/prometheus/prometheus.yml
  ports:
    - "9090:9090"

grafana:
  image: grafana/grafana
  ports:
    - "3000:3000"
  depends_on:
    - prometheus
```

### Future Enhancements

1. **Kubernetes Migration**: Migrate to K8s for production
2. **Service Mesh**: Add Istio or Linkerd
3. **Auto-Scaling**: Horizontal pod autoscaling
4. **Blue-Green Deployment**: Zero-downtime deployments
5. **Backup Automation**: Automated database backups

### Related Decisions

- [ADR-0005](ADR-0005-Gateway-Server-Separation.md) - Separate services enable containerization
- [ADR-0008](ADR-0008-Use-PostgreSQL-for-Persistence.md) - PostgreSQL container
- [ADR-0020](ADR-0020-Configuration-Management-Approach.md) - Configuration in containers

### References

- Docker Compose: [docker-compose.yml](../../docker-compose.yml)
- Server Dockerfile: [server/Dockerfile](../../server/Dockerfile)
- Gateway Dockerfile: [gateway/Dockerfile](../../gateway/Dockerfile)
- Makefile: [Makefile](../../Makefile)
- Docker Guide: [DOCKER.md](../../DOCKER.md)
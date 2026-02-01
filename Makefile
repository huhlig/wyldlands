.PHONY: help build up down restart logs clean test db-shell gateway-shell server-shell

# Default target
help:
	@echo "Wyldlands Docker Management"
	@echo ""
	@echo "Available targets:"
	@echo "  make build          - Build all Docker images"
	@echo "  make up             - Start all services"
	@echo "  make down           - Stop all services"
	@echo "  make restart        - Restart all services"
	@echo "  make logs           - View logs from all services"
	@echo "  make logs-gateway   - View gateway logs"
	@echo "  make logs-server    - View world server logs"
	@echo "  make logs-db        - View database logs"
	@echo "  make clean          - Stop and remove all containers, networks, and volumes"
	@echo "  make test           - Run tests in Docker"
	@echo "  make db-shell       - Open PostgreSQL shell"
	@echo "  make gateway-shell  - Open gateway container shell"
	@echo "  make server-shell   - Open world server container shell"
	@echo "  make db-backup      - Backup database to backup.sql"
	@echo "  make db-restore     - Restore database from backup.sql"
	@echo ""

# Build all images
build:
	docker-compose build

# Start all services
up:
	docker-compose up -d
	@echo "Services started. Access:"
	@echo "  Web Client: http://localhost:8080"
	@echo "  WebSocket:  ws://localhost:8080/ws"
	@echo "  Telnet:     telnet localhost 4000"

# Start all services with logs
up-logs:
	docker-compose up

# Stop all services
down:
	docker-compose down

# Restart all services
restart:
	docker-compose restart

# Restart specific service
restart-gateway:
	docker-compose restart gateway

restart-server:
	docker-compose restart server

restart-db:
	docker-compose restart database

# View logs
logs:
	docker-compose logs -f

logs-gateway:
	docker-compose logs -f gateway

logs-server:
	docker-compose logs -f server

logs-db:
	docker-compose logs -f database

# Clean everything
clean:
	docker-compose down -v --rmi all
	@echo "All containers, volumes, and images removed"

# Clean but keep images
clean-keep-images:
	docker-compose down -v
	@echo "Containers and volumes removed, images kept"

# Run tests
test:
	docker-compose up -d database
	@echo "Waiting for database..."
	@sleep 5
	docker-compose run --rm gateway cargo test
	docker-compose run --rm server cargo test

# Database shell
db-shell:
	docker-compose exec database psql -U postgres -d wyldlands

# Gateway shell
gateway-shell:
	docker-compose exec gateway /bin/bash

# World server shell
server-shell:
	docker-compose exec server /bin/bash

# Database backup
db-backup:
	docker-compose exec -T database pg_dump -U postgres wyldlands > backup.sql
	@echo "Database backed up to backup.sql"

# Database restore
db-restore:
	docker-compose exec -T database psql -U postgres -d wyldlands < backup.sql
	@echo "Database restored from backup.sql"

# Check service status
status:
	docker-compose ps

# View resource usage
stats:
	docker stats

# Rebuild specific service
rebuild-gateway:
	docker-compose build --no-cache gateway
	docker-compose up -d gateway

rebuild-server:
	docker-compose build --no-cache server
	docker-compose up -d server

# Development mode (with live logs)
dev:
	docker-compose up --build

# Production mode (detached)
prod:
	docker-compose up -d --build
	@echo "Production services started"
	@make status
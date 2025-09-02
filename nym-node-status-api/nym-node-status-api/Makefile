# Makefile for nym-node-status-api database management

# --- Configuration ---
TEST_DATABASE_URL := postgres://testuser:testpass@localhost:5433/nym_node_status_api_test

# Docker compose service names
DB_SERVICE_NAME := postgres-test
DB_CONTAINER_NAME := nym_node_status_api_postgres_test

# Default target
.PHONY: default
default: help

# --- Main Targets ---
.PHONY: prepare-pg
prepare-pg: test-db-up test-db-wait test-db-migrate test-db-prepare test-db-down ## Setup PostgreSQL and prepare SQLx offline cache

.PHONY: test-db
test-db: test-db-up test-db-wait test-db-migrate test-db-run test-db-down ## Run tests with PostgreSQL database

.PHONY: dev-db
dev-db: test-db-up test-db-wait test-db-migrate ## Start PostgreSQL for development (keeps running)
	@echo "PostgreSQL is running on port 5433"
	@echo "Connection string: $(TEST_DATABASE_URL)"

# --- Docker Compose Targets ---
.PHONY: test-db-up
test-db-up: ## Start the PostgreSQL test database in the background
	@echo "Starting PostgreSQL test database..."
	docker compose up -d $(DB_SERVICE_NAME)

.PHONY: test-db-wait
test-db-wait: ## Wait for the PostgreSQL database to be healthy
	@echo "Waiting for PostgreSQL database..."
	@while ! docker inspect --format='{{.State.Health.Status}}' $(DB_CONTAINER_NAME) 2>/dev/null | grep -q 'healthy'; do \
		echo -n "."; \
		sleep 1; \
	done; \
	echo " Database is healthy!"

.PHONY: test-db-down
test-db-down: ## Stop and remove the test database
	@echo "Stopping PostgreSQL test database..."
	docker compose down

# --- SQLx Targets ---
.PHONY: test-db-migrate
test-db-migrate: ## Run database migrations against PostgreSQL
	@echo "Running PostgreSQL migrations..."
	DATABASE_URL="$(TEST_DATABASE_URL)" sqlx migrate run --source migrations_pg

.PHONY: test-db-prepare
test-db-prepare: ## Run sqlx prepare for compile-time query verification
	@echo "Running sqlx prepare for PostgreSQL..."
	DATABASE_URL="$(TEST_DATABASE_URL)" cargo sqlx prepare

# --- Build and Test Targets ---
.PHONY: test-db-run
test-db-run: ## Run tests
	@echo "Running tests with PostgreSQL..."
	DATABASE_URL="$(TEST_DATABASE_URL)" cargo test

.PHONY: build
build: ## Build
	@echo "Building..."
	cargo build

.PHONY: check
check: ## Check code
	@echo "Checking code..."
	cargo check

.PHONY: clippy
clippy: ## Run clippy
	@echo "Running clippy..."
	cargo clippy -- -D warnings

# --- Cleanup Targets ---
.PHONY: clean
clean: ## Clean build artifacts and SQLx cache
	cargo clean
	rm -rf .sqlx

.PHONY: clean-db
clean-db: test-db-down ## Stop database and clean volumes
	docker volume rm -f nym-node-status-api_postgres_test_data 2>/dev/null || true

# --- Utility Targets ---
.PHONY: sqlx-cli
sqlx-cli: ## Install sqlx-cli if not already installed
	@command -v sqlx >/dev/null 2>&1 || cargo install sqlx-cli --features postgres

.PHONY: psql
psql: ## Connect to the running PostgreSQL database with psql
	@docker exec -it $(DB_CONTAINER_NAME) psql -U testuser -d nym_node_status_api_test

.PHONY: help
help: ## Show help for Makefile targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

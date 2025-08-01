# PostgreSQL Support for nym-node-status-api

This project now supports both SQLite (default) and PostgreSQL databases.

## Quick Start with PostgreSQL

### 1. Install Prerequisites

```bash
# Install sqlx-cli if not already installed
make sqlx-cli
```

### 2. Prepare PostgreSQL for Development

```bash
# This will:
# - Start PostgreSQL in Docker
# - Run migrations
# - Generate SQLx offline query cache
# - Stop the database
make prepare-pg
```

### 3. Build with PostgreSQL

```bash
# Build with PostgreSQL feature
make build-pg

# Or manually:
cargo build --features pg --no-default-features
```

### 4. Run with PostgreSQL

```bash
# Start PostgreSQL for development (keeps running)
make dev-db

# In another terminal, run the application
DATABASE_URL=postgres://testuser:testpass@localhost:5433/nym_node_status_api_test \
cargo run --features pg --no-default-features
```

## Database Features

- `sqlite` (default): Uses SQLite database
- `pg`: Uses PostgreSQL database

Only one database feature can be active at a time.

## Migration Differences

SQLite migrations are in `migrations/`, PostgreSQL migrations are in `migrations_pg/`.

Key differences:
- **AUTOINCREMENT** → **SERIAL**
- **INTEGER CHECK (0,1)** → **BOOLEAN**
- **REAL** → **DOUBLE PRECISION**
- No table recreation needed for constraint changes in PostgreSQL

## Makefile Targets

```bash
make help                # Show all available targets
make prepare-pg         # Setup PostgreSQL and prepare SQLx cache
make dev-db            # Start PostgreSQL for development
make test-db           # Run tests with PostgreSQL
make build-pg          # Build with PostgreSQL
make build-sqlite      # Build with SQLite
make psql              # Connect to running PostgreSQL
make clean             # Clean build artifacts
make clean-db          # Stop database and clean volumes
```

## Environment Variables

See `.env.example` for all configuration options. Key variable:

```bash
# For PostgreSQL:
DATABASE_URL=postgres://testuser:testpass@localhost:5433/nym_node_status_api_test

# For SQLite:
DATABASE_URL=sqlite://nym-node-status-api.sqlite
```

## Troubleshooting

### SQLx Offline Mode

If you see "no cached data for this query" errors:

1. Ensure PostgreSQL is running: `make dev-db`
2. Run: `make test-db-prepare`

### Connection Refused

If you see "Connection refused" errors:

1. Check Docker is running: `docker ps`
2. Check PostgreSQL container: `docker ps | grep nym_node_status_api_postgres_test`
3. Restart database: `make test-db-down && make dev-db`
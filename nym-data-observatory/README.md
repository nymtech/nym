# Nym Data Observatory

Collects data about the Nym network including:

- **Chain scraper** - that parses blocks, transactions and messages on the Nyx chain
- **Price scraper** - to get the NYM/USD token price from CoinGecko
- **Webhooks** - trigger on messages or all messages to call with details

## Running locally

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

### 3. Build

```bash
make build-pg
```

### 4. Run with PostgreSQL

```bash
# Start PostgreSQL for development (keeps running)
make test-db-up

# In another terminal, run the application
NYM_DATA_OBSERVATORY_DB_URL=postgres://testuser:testpass@localhost:5433/nym_data_observatory_test \
NYM_DATA_OBSERVATORY_WEBHOOK_URL="https://webhook.site" \
NYM_DATA_OBSERVATORY_WEBHOOK_AUTH=1234 \
cargo run -- run
```

To start from a block add the env var: `NYXD_SCRAPER_START_HEIGHT=19266184`.

## Deploying

Connect with `psql` to your local database:

```sql
CREATE USER nym_data_observatory WITH PASSWORD 'data-data-data';

CREATE DATABASE nym_data_observatory_data;
GRANT ALL ON DATABASE nym_data_observatory_data TO nym_data_observatory;
```

Then run:

```
cargo run -- init --db_url postgres://nym_data_observatory:data-data-data@localhost/nym_data_observatory_data
```

and then:

```
NYM_DATA_OBSERVATORY_DB_URL=postgres://nym_data_observatory:data-data-data@localhost/nym_data_observatory_data \
NYM_DATA_OBSERVATORY_WEBHOOK_URL="https://webhook.site" \
NYM_DATA_OBSERVATORY_WEBHOOK_AUTH=1234 \
cargo run -- run
```

## Troubleshooting

### SQLx Offline Mode

If you see "no cached data for this query" errors:

1. Ensure PostgreSQL is running: `make dev-db`
2. Run: `make test-db-prepare`

Also see [README_SQLX.md](../nyx-chain-watcher/README_SQLX.md).

### Connection Refused

If you see "Connection refused" errors:

1. Check Docker is running: `docker ps`
2. Check PostgreSQL container: `docker ps | grep nym_data_observatory
3. Restart database: `make test-db-down && make dev-db`




# Nym Node Status API
The Node Status API serves information about individual `nym-nodes` in the Mixnet, such as which role they are operating in, statistics about them, services such as Network Requesters, as well as summaries of the state of the Mixnet.

We recommend that developers building applications such as explorers or analytics interfaces about the Mixnet run their own instance of the API, in order to promote a robust network of downstream services, and spread the load of API calls amongst as many endpoints as possible.

You can find build and operation instructions in the [docs](https://nym.com/docs/apis/ns-api).

## Database Support

The Node Status API supports both SQLite and PostgreSQL databases through Cargo feature flags:

- **SQLite** (default): Lightweight, file-based database suitable for development and small deployments
- **PostgreSQL**: Full-featured database recommended for production deployments

### Building with Different Database Backends

```bash
# Build with SQLite (default)
cargo build --features sqlite --no-default-features

# Build with PostgreSQL
cargo build --features pg --no-default-features
```

### Running Tests

```bash
# Test with SQLite
cargo test --features sqlite --no-default-features

# Test with PostgreSQL
make test-db  # This sets up a test PostgreSQL instance
```

### Development Commands

The project includes a Makefile with helpful commands for both database backends:

```bash
# Check code compilation
make check-sqlite     # Check with SQLite
make check-pg        # Check with PostgreSQL

# Run clippy linter
make clippy-sqlite   # Lint with SQLite
make clippy-pg      # Lint with PostgreSQL
make clippy         # Run both

# PostgreSQL development
make dev-db         # Start a PostgreSQL instance for development
make prepare-pg     # Prepare SQLx offline cache for PostgreSQL
```

### Implementation Details

The database abstraction is implemented using a query wrapper that automatically converts SQLite-style `?` placeholders to PostgreSQL-style `$1, $2, ...` placeholders at runtime. This allows writing queries once using SQLite syntax while maintaining compatibility with both databases.

Key differences handled:
- Placeholder syntax (`?` vs `$1, $2, ...`)
- Type conversions (SQLite uses i64, PostgreSQL uses i32 for many fields)
- SQL dialect differences (e.g., `INSERT OR IGNORE` vs `ON CONFLICT DO NOTHING`)
- RETURNING clause behavior

For more details on PostgreSQL setup, see [README_PG.md](nym-node-status-api/README_PG.md).

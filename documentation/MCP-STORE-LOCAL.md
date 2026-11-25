# Run a local retrieval store

This guide runs a Postgres + pgvector store on your machine, fills it with the nym docs and code index, and runs the docs site with the MCP route reading it. Use it to work on the store path before or without the public store. For what the store is, read [`MCP-STORE.md`](MCP-STORE.md). The public store is created with `bootstrap/init.sql` and the `store-migrate` workflow in [`nymtech/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer).

The local store has no TLS and no PgBouncer, and it listens on `127.0.0.1` only.

## Before you start

- Docker.
- A checkout of `nym` with `pnpm install` done in `documentation/docs`.
- A Voyage API key, to embed the index and the queries. A full first run embeds about 10,000 chunks; later runs reuse the embed cache in `documentation/docs/node_modules/.cache/nym-docs`.

Run every command below from `documentation/docs`.

## 1. Start the store

```sh
export STORE_PASSWORD=$(head -c 18 /dev/urandom | base64 | tr -d '/+=')
docker run -d --name retrieval-store-dev -p 127.0.0.1:54329:5432 -e POSTGRES_PASSWORD="$STORE_PASSWORD" -e POSTGRES_DB=retrieval pgvector/pgvector:pg17
until docker exec retrieval-store-dev pg_isready -U postgres -d retrieval >/dev/null 2>&1; do sleep 1; done
```

1. `STORE_PASSWORD` is a random password for the owner account (`postgres`).
2. `-p 127.0.0.1:54329:5432` publishes the database on port 54329 of this machine only.
3. The `until` loop waits until Postgres accepts connections.

## 2. Create the schema and the roles

```sh
node --input-type=module -e "import pg from 'pg'; import { ensureSchema } from '@nymproject/retrieval-indexer/store'; const db = new pg.Client({ connectionString: 'postgresql://postgres:$STORE_PASSWORD@127.0.0.1:54329/retrieval?sslmode=disable' }); await db.connect(); await ensureSchema(db); await db.end()"
export WRITER_PASSWORD=$(head -c 18 /dev/urandom | base64 | tr -d '/+=') READER_PASSWORD=$(head -c 18 /dev/urandom | base64 | tr -d '/+=')
docker exec -i retrieval-store-dev psql -U postgres -d retrieval -v ON_ERROR_STOP=1 -q <<SQL
create role retrieval_writer login password '$WRITER_PASSWORD';
grant select, insert, update, delete on chunks to retrieval_writer;
grant select, insert, update on shards to retrieval_writer;
create role retrieval_reader login connection limit 20 password '$READER_PASSWORD';
grant select on chunks, shards to retrieval_reader;
alter role retrieval_reader set statement_timeout = '5s';
SQL
```

These are the schema and the grants of the public store. `ensureSchema` applies the package's schema without node-pg-migrate, which is enough for a store you delete afterwards. The grants run with `psql` inside the container, so you do not need it on your machine. The passwords are in the SQL text here, which is acceptable for a store that holds only public content and listens only on this machine.

## 3. Save the connection strings

```sh
cat > ~/.retrieval-store-dev.env <<ENV
RETRIEVAL_STORE_WRITE_URL=postgresql://retrieval_writer:$WRITER_PASSWORD@127.0.0.1:54329/retrieval?sslmode=disable
RETRIEVAL_STORE_READ_URL=postgresql://retrieval_reader:$READER_PASSWORD@127.0.0.1:54329/retrieval?sslmode=disable
ENV
chmod 600 ~/.retrieval-store-dev.env
```

Later sessions load the file with `set -a && . ~/.retrieval-store-dev.env && set +a`.

## 4. Fill the store

```sh
read -rs VOYAGE_API_KEY && export VOYAGE_API_KEY
REQUIRE_EMBEDDINGS=1 node ../scripts/next-scripts/generate-index.mjs
REQUIRE_EMBEDDINGS=1 node ../scripts/next-scripts/generate-code-index.mjs
set -a && . ~/.retrieval-store-dev.env && set +a
node lib/retrieval/write-store.mts
```

1. `read -rs` reads the key without showing it and without putting it in your shell history.
2. The two generators write `public/docs-index.json` and `public/code-index.json` with vectors. `REQUIRE_EMBEDDINGS=1` makes them fail instead of writing files without vectors, which the writer refuses.
3. `write-store.mts` writes the `nym/docs` and `nym/code` shards with the vectors from those files. It embeds nothing itself.

Expected result: one line for each shard, for example `nym/docs (voyage-3-large): 2039 chunks, 2039 changed, 0 unchanged, 0 deleted`. A second run prints `0 changed`.

## 5. Run the site on the store

```sh
pnpm run build:data
MCP_RETRIEVAL_SOURCE=store npx next dev -p 3999
```

`build:data` writes the threat-model records and the snippet link map that pages render from. `pnpm dev` runs it in `predev`, and `npx next dev` does not. The route reads `RETRIEVAL_STORE_READ_URL` from the environment of step 4, and embeds queries with `VOYAGE_API_KEY`. In a second terminal, from `documentation`, check it:

```sh
bash scripts/check-mcp-server.sh http://localhost:3999
```

Expected result: `50 passed, 0 failed`.

## Stop or remove the store

- `docker stop retrieval-store-dev` stops it and keeps the data. `docker start retrieval-store-dev` starts it again.
- `docker rm -f retrieval-store-dev` deletes it with its data. Delete `~/.retrieval-store-dev.env` with it.

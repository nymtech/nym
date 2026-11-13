# Provision the public retrieval store

This guide creates the public Postgres store that the MCP server reads after the cutover, and puts its credentials where the producers and the route can use them. For what the store is and why it exists, read [`MCP-STORE.md`](MCP-STORE.md).

The guide covers task 4.1 of the OpenSpec change `add-shared-retrieval-index`, for the public store only. The private store is a separate database on Nym infrastructure.

## Before you start

- Our Postgres server on Exoscale, with the `pgvector` extension installed and a PgBouncer pooler (transaction mode) in front of it.
- `psql` on your machine.
- A checkout of `nym` with `pnpm install` done in `documentation/docs`, so that the package `@nymproject/retrieval-indexer` is installed.
- Admin access to the `nymtech/nym` repository secrets and to the docs Vercel project's environment variables.

## 1. Create the database

1. Create a database for the store on the Exoscale Postgres server. The store is about 64 MB today and about 130 MB at 20,000 chunks, and the server needs enough RAM to keep it cached.
2. Note the server's direct address (Postgres) and its pooled address (PgBouncer). You need both in step 4.
3. Put the owner account's direct connection string in a shell variable for the next steps:

   ```
   export OWNER_URL='postgresql://<owner>:<password>@<direct address>/<database>?sslmode=require'
   ```

## 2. Create the schema

Run this from `documentation/docs`. It prints the package's schema statements and runs them on the database:

```
node --input-type=module -e "import { SCHEMA } from '@nymproject/retrieval-indexer/store'; console.log(SCHEMA.join(';\n') + ';')" | psql -v ON_ERROR_STOP=1 -1 "$OWNER_URL"
```

1. `node -e` imports `SCHEMA` from the installed package, so the tables match the package version that the producers use.
2. `SCHEMA.join(';\n') + ';'` ends each statement with a semicolon, so that `psql` can read them from standard input.
3. `-v ON_ERROR_STOP=1` makes `psql` stop at the first failed statement and exit non-zero. `-1` runs all statements in one transaction, so a failure leaves no half-made schema.
4. The first statement, `create extension if not exists vector`, needs the owner's privileges.

## 3. Create the writer and reader roles

Open a session as the owner:

```
psql "$OWNER_URL"
```

Then run:

```sql
create role retrieval_writer login;
\password retrieval_writer
grant select, insert, update, delete on chunks to retrieval_writer;
grant select, insert, update on shards to retrieval_writer;

create role retrieval_reader login connection limit 20;
\password retrieval_reader
grant select on chunks, shards to retrieval_reader;
alter role retrieval_reader set statement_timeout = '5s';
```

1. `\password` asks for the password and sends it hashed, so the password does not go into your shell history or the server log. Use a new random password for each role.
2. The writer gets what a write run uses: all four privileges on `chunks`, and `select`, `insert`, and `update` on `shards`. A run never deletes a shard.
3. The reader gets `select` only. Every warm function instance holds a connection, and preview deployments use the same credential, so the connection limit and the statement timeout stop one deployment from using the whole database.

## 4. Store the credentials

Make two connection strings from the addresses of step 1:

```
writer: postgresql://retrieval_writer:<password>@<direct address>/<database>?sslmode=verify-full
reader: postgresql://retrieval_reader:<password>@<pooled address>/<database>?sslmode=verify-full
```

The producers and the route connect with `node-postgres`, which treats `sslmode=require` as `verify-full` and prints a warning, so the strings say `verify-full`.

| Secret | Value | Where |
|---|---|---|
| `RETRIEVAL_STORE_WRITE_URL` | The writer string | GitHub Actions secret in `nymtech/nym`, and later in `nymtech/nym-vpn-client` |
| `RETRIEVAL_STORE_READ_URL` | The reader string | Vercel environment variable of the docs project, for Production and Preview |

The writer uses the direct address. A write run is one long transaction on one `pg.Client` (the package rejects a `pg.Pool`), and it needs no pooling. The reader uses the pooled address, because the route's function instances each hold a connection, and the pooler shares them.

Vercel applies a new environment variable only to new deployments. Redeploy, or wait for the next deployment, before you test the route.

## 5. Check the store

1. Check the privileges of both roles. This query reads the catalogue and changes no data:

   ```
   psql "$OWNER_URL" -c "select r as role, t as table, p as privilege, has_table_privilege(r, t, p) as granted from (values ('retrieval_reader'), ('retrieval_writer')) as rs(r) cross join (values ('chunks'), ('shards')) as ts(t) cross join (values ('select'), ('insert'), ('update'), ('delete')) as ps(p) order by 1, 2, 3"
   ```

   Expected result:

   | Role | `chunks` | `shards` |
   |---|---|---|
   | `retrieval_reader` | `select` only | `select` only |
   | `retrieval_writer` | all four | `select`, `insert`, `update` |

   Any other `t` means the role has more rights than the grants give. Revoke them before you store the credential.

2. Check that the reader can connect through the pooler:

   ```
   psql "<reader string>" -c "select count(*) from chunks"
   ```

   Expected result: `0`, because no producer has written yet. If `psql` reports a certificate error, add `&sslrootcert=system` to the string (libpq 16 or later).

## After a package release

`SCHEMA` uses `create ... if not exists`, so step 2 does not change a table that exists. A package release that changes `SCHEMA` needs its own migration, and a new table needs the grants of step 3.

## Next

The store is ready for task 5.0, the parity check between the package and the live route. Task 5.1 then adds the write step to the nym docs CI. That job runs on pushes to develop only, needs `RETRIEVAL_STORE_WRITE_URL` and `VOYAGE_API_KEY`, and needs a `concurrency` group, so that two runs never write at once.

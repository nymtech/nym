# Provision the public retrieval store

This guide creates the public Postgres store that the MCP server reads after the cutover, and puts its credentials where the producers and the route can use them. For what the store is and why it exists, read [`MCP-STORE.md`](MCP-STORE.md). To run a store on your machine instead, follow [`MCP-STORE-LOCAL.md`](MCP-STORE-LOCAL.md).

The guide covers task 4.1 of the OpenSpec change `add-shared-retrieval-index`, for the public store only. The private store is a separate database on Nym infrastructure.

## Before you start

- Our Postgres server on Exoscale (DBaaS), with the `pgvector` extension available. No pooler: the route keeps its own connection pool (step 4).
- `psql` on your machine.
- Admin access to `nymtech/retrieval-indexer`, to create a GitHub Environment and run its `store-migrate` workflow.
- Admin access to the `nymtech/nym` repository secrets and to the docs Vercel project's environment variables.

## 1. Create the database

1. As the server admin (`avnadmin`), connected to `defaultdb`, run `bootstrap/init.sql` from `nymtech/retrieval-indexer`. It creates the `codex` database, the owner role `retrieval_ddl`, the roles `retrieval_writer` and `retrieval_reader`, and their default privileges. Replace the placeholder passwords with `\password <role>`. The store takes about 130 MB at 20,000 chunks; give the server enough RAM to keep it cached.
2. Note the server's address. Every connection string in this guide ends in `/codex`. The connection URI in the Exoscale console ends in `/defaultdb`: change it when you copy it.
3. Put the owner password in `~/.pgpass`, so that it does not go into your shell history or into a connection string:

   ```
   touch ~/.pgpass && chmod 600 ~/.pgpass
   ```

   Then add this line to the file with an editor:

   ```
   <direct host>:*:<database>:<owner>:<password>
   ```

   `<direct host>` is the host name of the direct address, without the port, and `*` matches any port. If the password contains `:` or `\`, put a `\` before each one (`a:b` becomes `a\:b`). `libpq` ignores the file if other users can read it, so set the mode before you write the password.
4. Put the owner account's direct connection string, with no password, in a shell variable for the next steps:

   ```
   export OWNER_URL='postgresql://<owner>@<direct address>/<database>?sslmode=verify-full&sslrootcert=<CA certificate file>'
   ```

   With `verify-full`, `psql` checks the server certificate against the CA certificate in `sslrootcert`, and checks that the certificate names the host. `require` only encrypts the connection, and it accepts a server that uses a certificate for a different host.

## 2. Create the schema

The schema is the SQL files in `migrations/` of `nymtech/retrieval-indexer`. That repository's `store-migrate` workflow applies them with node-pg-migrate, which records each applied file in the store's `pgmigrations` table. This is the same setup as the VPN API's `vpn-api-postgres-migrate` workflow.

1. In `nymtech/retrieval-indexer`, create a GitHub Environment, for example `retrieval-store`, with two secrets:
   - `PG_URL`: the owner's (`retrieval_ddl`) connection string, ending in `/codex`, with the password and without `sslrootcert`. Connect directly, not through a transaction-mode pooler: node-pg-migrate holds a session advisory lock.
   - `PG_CA_CERT`: the CA certificate of the database service as PEM text, from `-----BEGIN CERTIFICATE-----` to `-----END CERTIFICATE-----`, not base64-encoded. A wrong value fails with `self-signed certificate in certificate chain`. The workflow connects with `sslmode=verify-full`.

   Add a required reviewer to the Environment if a second person must approve each run.
2. Run the workflow as a dry run: Actions, `store-migrate`, "Run workflow", the Environment name, and `dry_run` checked. The log lists `0001_initial`. A dry run applies no migration, but it creates the empty `pgmigrations` table.
3. Run it again with `dry_run` cleared. The log ends with `Migrations complete!`.

- The first migration runs `create extension if not exists vector`, so the owner needs the privilege to create extensions. node-pg-migrate also needs `create` on the schema, for its `pgmigrations` table.
- Each run logs `Can't determine timestamp for 0001`. node-pg-migrate expects a timestamp prefix, and it orders the numbered files by their number instead. Nothing is wrong.

## 3. Finish the writer and reader roles

`bootstrap/init.sql` gives the writer `select`, `insert` and `update`, and the reader `select`, on each table the migrations create. Two settings remain.

As the owner (`retrieval_ddl`), after step 2:

```sql
grant delete on chunks to retrieval_writer;
```

As the server admin:

```sql
alter role retrieval_reader connection limit 20;
```

1. A write run deletes the chunks that left a shard, and never deletes a shard, so the writer has `delete` on `chunks` only.
2. The connection limit covers every route instance and preview together. The route sets a 5 second `statement_timeout` on its connections.
3. To check the rights of both roles, use the query in step 5.

## 4. Store the credentials

Make two connection strings from the addresses of step 1:

```
writer: postgresql://retrieval_writer:<password>@<direct address>/<database>
reader: postgresql://retrieval_reader:<password>@<direct address>/<database>
```

The strings carry no `sslmode` or `sslrootcert`. The producers and the route read the CA from `RETRIEVAL_STORE_CA_CERT` and verify the certificate chain and the host name. They refuse a string that sets TLS parameters, and a CA that is not PEM text. Percent-encode `@`, `:`, `/`, `#` or `?` in a password (`@` becomes `%40`).

| Secret | Value | Where |
|---|---|---|
| `RETRIEVAL_STORE_WRITE_URL` | The writer string | GitHub Actions secret in `nymtech/nym` and in `nymtech/nym-vpn-client` |
| `RETRIEVAL_STORE_READ_URL` | The reader string | Vercel environment variable of the docs project, for Production and Preview |
| `RETRIEVAL_STORE_CA_CERT` | The CA certificate as PEM text, the same value as `PG_CA_CERT` in step 2 | GitHub Actions secret in `nymtech/nym` and in `nymtech/nym-vpn-client`, like `PG_CA_CERT`, and a Vercel environment variable of the docs project beside `RETRIEVAL_STORE_READ_URL` |
| `RETRIEVAL_STORE_ENABLED` | `1` | GitHub Actions repository variable (not a secret) in `nymtech/nym` and in `nymtech/nym-vpn-client`. The `cd-retrieval-store` workflow of each repository writes the store only while it is `1`; set it to `0` to stop writes without deleting the secret |
| `VOYAGE_API_KEY` | The Voyage API key | GitHub Actions secret in `nymtech/nym` and in `nymtech/nym-vpn-client`, apart from the docs site's Vercel variable |

Both roles use the direct address. A write run is one transaction on one `pg.Client`. The route keeps a `pg.Pool` of at most 2 connections in each function instance, closed after 30 seconds idle, and the reader's limit of 20 caps them together. At the limit, searches fail with `too many connections for role "retrieval_reader"`; raise the limit or add a pooler.


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

2. Check that the reader can connect to the store database:

   ```
   psql "<reader string>?sslmode=verify-full&sslrootcert=<CA certificate file>" -c "select count(*) from chunks"
   ```

   Expected result: `0`, because no producer has written yet. The stored string has no TLS settings, so this command adds them for `psql`.

## After a package release

A package release that adds a migration file needs a `store-migrate` run, dry run first, as in step 2. node-pg-migrate applies only the files that `pgmigrations` does not list. A migration that adds a table names the grants the writer and reader need, and you run those grants as in step 3.

## Next

The store is ready for task 5.0, the parity check between the package and the live route. The `cd-retrieval-store` workflow (task 5.1) writes the nym shards on each push to develop once `RETRIEVAL_STORE_ENABLED` is `1`. It uses the `RETRIEVAL_STORE_WRITE_URL` secret and the existing `VOYAGE_API_KEY` secret, and its `concurrency` group keeps two runs from writing at once.

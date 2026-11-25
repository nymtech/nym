# Provision the public retrieval store

This guide creates the public Postgres store that the MCP server reads after the cutover, and puts its credentials where the producers and the route can use them. For what the store is and why it exists, read [`MCP-STORE.md`](MCP-STORE.md). To run a store on your machine instead, follow [`MCP-STORE-LOCAL.md`](MCP-STORE-LOCAL.md).

The guide covers task 4.1 of the OpenSpec change `add-shared-retrieval-index`, for the public store only. The private store is a separate database on Nym infrastructure.

## Before you start

- Our Postgres server on Exoscale (DBaaS), with the `pgvector` extension available. The store uses no pooler: the route keeps its own small connection pool (see step 4).
- `psql` on your machine.
- Admin access to `nymtech/retrieval-indexer`, to create a GitHub Environment and run its `store-migrate` workflow.
- Admin access to the `nymtech/nym` repository secrets and to the docs Vercel project's environment variables.

## 1. Create the database

1. Create a database for the store on the Exoscale Postgres server. Our store database is `codex`. The store is about 64 MB today and about 130 MB at 20,000 chunks, and the server needs enough RAM to keep it cached.
2. Note the server's address. Every connection string in this guide uses it, and each one ends in `/<database>`. The connection URI in the Exoscale console ends in `/defaultdb`, the server's default database. Change that part to the store database every time you copy the URI, or the schema and the data go to `defaultdb`.
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
   - `PG_URL`: the owner's connection string, with the password and without `sslrootcert`. It must end in the store database (`/codex`), not `/defaultdb`. Do not put a transaction-mode pooler between the workflow and the server: node-pg-migrate holds a session advisory lock, which transaction pooling does not keep.
   - `PG_CA_CERT`: the CA certificate of the database service as PEM text, from `-----BEGIN CERTIFICATE-----` to `-----END CERTIFICATE-----`. Do not encode it with base64: the workflow writes the value to a file as it is, and Node accepts a value that is not a certificate without an error, so the connection then fails with `self-signed certificate in certificate chain`. The workflow adds the file to the connection with `sslmode=verify-full`.

   Add a required reviewer to the Environment if a second person must approve each run.
2. Run the workflow as a dry run: Actions, `store-migrate`, "Run workflow", the Environment name, and `dry_run` checked. The log lists `0001_initial`. A dry run applies no migration, but it creates the empty `pgmigrations` table.
3. Run it again with `dry_run` cleared. The log ends with `Migrations complete!`.

- The first migration runs `create extension if not exists vector`, so the owner needs the privilege to create extensions. node-pg-migrate also needs `create` on the schema, for its `pgmigrations` table.
- Each run logs `Can't determine timestamp for 0001`. node-pg-migrate expects a timestamp prefix, and it orders the numbered files by their number instead. Nothing is wrong.

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
```

1. `\password` asks for the password and sends it hashed, so the password does not go into your shell history or the server log. Use a new random password for each role.
2. The writer gets what a write run uses: all four privileges on `chunks`, and `select`, `insert`, and `update` on `shards`. A run never deletes a shard.
3. The reader gets `select` only. Every warm function instance holds a connection, and preview deployments use the same credential, so the connection limit stops one deployment from using the whole database. The route sets a 5 second `statement_timeout` on each of its connections, so the server stops a slow search.

## 4. Store the credentials

Make two connection strings from the addresses of step 1:

```
writer: postgresql://retrieval_writer:<password>@<direct address>/<database>
reader: postgresql://retrieval_reader:<password>@<direct address>/<database>
```

The strings have no `sslmode` or `sslrootcert`. The server certificate is signed by the provider's own CA, which GitHub runners and Vercel do not trust, and they have no CA file. So the producers and the route take the CA as PEM text in `RETRIEVAL_STORE_CA_CERT`, and `node-postgres` then checks the certificate chain and the host name (`verify-full`). `node-postgres` lets TLS settings in a URL replace that CA, so the code refuses a URL that sets them, and it refuses a CA that is not PEM text. If a password contains `@`, `:`, `/`, `#` or `?`, percent-encode it (`@` becomes `%40`).

| Secret | Value | Where |
|---|---|---|
| `RETRIEVAL_STORE_WRITE_URL` | The writer string | GitHub Actions secret in `nymtech/nym` and in `nymtech/nym-vpn-client` |
| `RETRIEVAL_STORE_READ_URL` | The reader string | Vercel environment variable of the docs project, for Production and Preview |
| `RETRIEVAL_STORE_CA_CERT` | The CA certificate as PEM text, the same value as `PG_CA_CERT` in step 2 | GitHub Actions repository variable (not a secret: a CA certificate is public) in `nymtech/nym` and in `nymtech/nym-vpn-client`, and a Vercel environment variable of the docs project beside `RETRIEVAL_STORE_READ_URL` |
| `RETRIEVAL_STORE_ENABLED` | `1` | GitHub Actions repository variable (not a secret) in `nymtech/nym` and in `nymtech/nym-vpn-client`. The `cd-retrieval-store` workflow of each repository writes the store only while it is `1`; set it to `0` to stop writes without deleting the secret |
| `VOYAGE_API_KEY` | The Voyage API key | GitHub Actions secret in `nymtech/nym-vpn-client`. `nymtech/nym` has it already, for its docs builds |

Both roles use the direct address. A write run is one long transaction on one `pg.Client` (the package rejects a `pg.Pool`). The route keeps a `pg.Pool` of at most 2 connections in each warm function instance, and closes a connection after 30 seconds idle. The reader's connection limit of 20 caps all instances together. If traffic ever reaches that limit, searches fail with `too many connections for role "retrieval_reader"`, and the writer is not affected; raise the limit or add a pooler then.


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

   Expected result: `0`, because no producer has written yet. `psql` takes the CA from a file, so this check adds the TLS settings that the stored string leaves out.

## After a package release

A package release that adds a migration file needs a `store-migrate` run, dry run first, as in step 2. node-pg-migrate applies only the files that `pgmigrations` does not list. A migration that adds a table names the grants the writer and reader need, and you run those grants as in step 3.

## Next

The store is ready for task 5.0, the parity check between the package and the live route. The `cd-retrieval-store` workflow (task 5.1) writes the nym shards on each push to develop once `RETRIEVAL_STORE_ENABLED` is `1`. It uses the `RETRIEVAL_STORE_WRITE_URL` secret and the existing `VOYAGE_API_KEY` secret, and its `concurrency` group keeps two runs from writing at once.

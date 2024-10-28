# Making `sqlx` work

Some of the errors encountered and possible solutions.

## `The cargo feature offline has to be enabled to use SQLX_OFFLINE`

Did you enable `offline` **cargo feature** of `sqlx` dependency in your
`Cargo.toml`?

Also, it may happen if you have a version mismatch beetween

- `sqlx` cargo dependency in `Cargo.toml`
- `sqlx-cli` installed by cargo

To ensure correct version, do

```
cargo uninstall sqlx-cli
cargo install --version <version> sqlx-cli
```

where `<version>` matches the version of sqlx in your `Cargo.toml`.

## `Error: failed to connect to database: password authentication failed`

If it's in-code, make sure you don't "double authenticate", i.e.

- if username and password are already specified in `DATABASE_URL`
- then, you don't have to use

```rust
ConnectOptions::from_str(&database_Url)
    .username() // unnecessary
    .password() // unnecessary
```

If it's outside of code (i.e. when running `cargo check`)

- make sure password doesn't have any special characters that could be
  interpreted by the command line/shell weirdly, like `$#\` etc.

## Offline query data looks like this

```
.sqlx/
├─ new_file
├─ query-249faa11b88b749f50342bb5c9cc41d20896db543eed74a6f320c041bcbb723d.json
├─ query-aff7fbd06728004d2f2226d20c32f1482df00de2dc1d2b4debbb2e12553d997b.json
├─ ...
├─ query-e53f479f8cead3dc8aa1875e5d450ad69686cf6a109e37d6c3f0623c3e9f91d0.json
```

The offline mode for the queries uses a separate file per `query!()` invocation

Each workspace member that works with `sqlx` has `.sqlx` directory, containing
its own schema description. This allows compile-time checks without needing a
live DB connection (so called `OFFLINE_MODE`).

To initialize those files, you need to run `cargo sqlx prepare` with a live
connection to DB (to pull schema information).

### Similar to:

```
warning: no queries found; do you have the `offline` feature enabled
```

### Possible solutions

- does your `sqlx-cli` version match `sqlx` version from `Cargo.toml`?
  + `cargo install -f sqlx-cli --version <specific version>`
```
cargo install sqlx-cli --version <exact semver version as sqlx> --force
```
- is your crate a library?
```
cargo sqlx prepare -- --lib
```
- are your `query!` invocations hidden behind a feature?
```
cargo sqlx prepare -- --features <feature_name>
```
- do you have `offline` cargo feature enabled?
- make sure to `cargo clean` after these updates

## Any many, many more

- `EOF while parsing a value at line`
- `failed to find data for query`

### Possible solutions

- Usually a DB connection issue
- Retry everything
- Throw in a `cargo clean -p <your package>` for good measure

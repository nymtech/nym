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

## Cannot generate `sqlx-data.json`

In order for `sqlx` to generate schema for "offline" work (without DB
connection), as of `v0.6.3` you first **need an active DB connection**.

So make sure

- DB is running
- `DATABASE_URL` is set correctly
- `SQLX_OFFLINE` isn't exported to true

Then run `cargo sqlx prepare`

After you have the file, you can ignore `DATABASE_URL` and terminate the DB
instance. This file represents the DB schema, so when your migrations change,
you'll need to re-generate it

Make sure to commit the file to VCS if you want to avoid re-doing this again on
each machine (e.g. other developers, CI).

## Generated `sqlx-data.json` looks like this

```json
{
  "db": "PostgreSQL"
}
```

after running `cargo sqlx prepare`

### Similar to:

```
warning: no queries found; do you have the `offline` feature enabled
```

### Possible solutions

- does your `sqlx-cli` version match `sqlx` version from `Cargo.toml`?
  + `cargo install -f sqlx-cli --version <specific version>`
- do you have `offline` cargo feature enabled?
- make sure to `cargo clean` after these updates

## Any many, many more

- `EOF while parsing a value at line`
- `failed to find data for query`

### Possible solutions

- Usually a DB connection issue
- Retry everything
- Throw in a `cargo clean -p <your package>` for good measure

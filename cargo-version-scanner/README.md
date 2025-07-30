# Cargo Version Scanner
Simple tool to parse + check all versions of crates in the monorepo. Optionally outputs a nice table for quickly checking versions.

```sh
❯ cargo run -- -h
 Scan Cargo.toml files in a Rust monorepo and analyze versions

 Usage: cargo-version-scanner [OPTIONS]

 Options:
   -v, --verbose          Show verbose list of all crates, paths & versions
   -u, --unset-only       Only show crates with UNSET versions
       --sort-by-version  Sort by version instead of path (alphabetical)
   -h, --help             Print help

# Logs the verbose table @ the end sorted by version, instead of alphabeticly
❯ cargo run -- -v --sort-by-version
  ```

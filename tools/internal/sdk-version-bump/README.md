# sdk-version-bump

Tool to update version numbers across the SDK packages, both Cargo
(`wasm/smolmix`, `wasm/client`) and TS (`@nymproject/mix-tunnel`,
`@nymproject/mix-fetch`, `@nymproject/mix-dns`, `@nymproject/mix-websocket`,
plus the legacy `@nymproject/sdk` family).

## Usage

This tool is expected to be run by CI; the snippets below are for manual use
from the repo root.

### Full release (Rust + TS in parity)

This is the default. Use it when smolmix-wasm has changed and the TS SDK
must publish a matching version.

1. `cargo run -p sdk-version-bump remove-suffix`
2. Build everything and publish to crates.io / npm.
3. `cargo run -p sdk-version-bump bump-version` — bumps every package from
   `X.Y.Z` to `X.Y.(Z+1)-rc.0`, and rewrites every `@nymproject/...` dep
   specifier of the form `>=X.Y.Z-rc.W || ^X` accordingly.

### TS-only release

Use this when the TS SDK needs a bump but smolmix-wasm itself hasn't changed.
The Cargo crates are left untouched, and downstream package.json files keep
their existing dep ranges for `@nymproject/smolmix-wasm` and
`@nymproject/nym-client-wasm` (so they continue to resolve to the last
published wasm version).

1. `cargo run -p sdk-version-bump remove-suffix --ts-only`
2. Build the TS packages and publish to npm.
3. `cargo run -p sdk-version-bump bump-version --ts-only`

### Pre-release (rc bump)

To increment just the `-rc.X` counter (`X.Y.Z-rc.W` → `X.Y.Z-rc.W+1`):

```
cargo run -p sdk-version-bump bump-version --pre-release
```

Combine with `--ts-only` if you're rc-bumping the TS layer alone.

## What gets touched

| What | Where it's declared | Behaviour under `--ts-only` |
|---|---|---|
| `Cargo.toml` `package.version` of each Cargo crate | `register_cargo(...)` | skipped |
| `package.json` `version` of each TS package | `register_json(...)` | bumped |
| `@nymproject/...` dep ranges in `dependencies` / `peerDependencies` / `devDependencies` / `optionalDependencies` / `bundledDependencies` | `register_known_js_dependency(...)` | bumped, except cargo-derived ones |
| Cargo-derived JS dep ranges (`@nymproject/smolmix-wasm`, `@nymproject/nym-client-wasm`) | `register_cargo_derived_js_dependency(...)` | left alone |
| `workspace:*`, `file:..`, `link:..`, `npm:..` specifiers | n/a (skipped at parse time) | always skipped — pnpm rewrites them at publish |

## Adding a new package

1. If it's a Cargo crate: `packages.register_cargo("relative/path")`.
2. If it's a TS package: `packages.register_json("relative/path")`.
3. If the package is consumed by other packages as a dep, also add its npm
   name to either `register_known_js_dependency` (TS-driven version) or
   `register_cargo_derived_js_dependency` (wasm-pack-driven version).

# Proposal: nym-prefix-smol-crates

## Why

The `smol*` family (`smol-core`, `smolmix`, `smoldvpn`, and the internal
`smolmix-wasm`) shipped without the `nym-` package-name prefix that the rest of
the workspace follows (`nym-crypto`, `nym-client-core`, ...). This is naming
drift: the crates.io namespace convention is `nym-<name>`, and these four were
the exceptions. Aligning them now is cheap — nothing in-tree depends on
`smolmix`/`smoldvpn` as a library, and `smol-core` has only two in-tree
consumers (`smolmix/core`, `smoldvpn`).

## What Changes

- **Rename the packages** (directories unchanged; the repo already mixes
  prefixed and un-prefixed directory names, e.g. `common/crypto` → `nym-crypto`
  but `common/nym-kcp` → `nym-kcp`):
  - `smol-core` → `nym-smol-core` (lib `nym_smol_core`)
  - `smolmix` → `nym-smolmix` (lib `nym_smolmix`)
  - `smoldvpn` → `nym-smoldvpn` (lib `nym_smoldvpn`)
  - `smolmix-wasm` → `nym-smolmix-wasm`
- **Cascade** into workspace dep-table keys, the `[profile.release.package.*]`
  key, every `use nym_smol_core::…`, and the `cargo …-p` invocations in docs.
  Type names (`SmolCoreError`, `SmolmixError`) are unchanged — only crate/lib
  identities move.
- **`nym-smoldvpn` logging target**: the examples' `EnvFilter` default and the
  `RUST_LOG` hints move from `smoldvpn=…` to `nym_smoldvpn=…` (tracing targets
  derive from the crate's module path).
- **Product vs crate naming**: `smolmix` keeps its **product** name in prose,
  its docs page (`/developers/smolmix`), and the TypeScript SDK family; only the
  **crate** becomes `nym-smolmix` (install snippet, `docs.rs/nym-smolmix`,
  `cargo run -p nym-smolmix`). `smol-core`/`smoldvpn` have no separate product
  identity, so they take the full crate name everywhere.
- **`nym-smolmix-wasm` pins its artifact**: the crate is renamed, but the
  generated wasm artifact (`smolmix_wasm_bg.wasm`) and the private, never-published
  npm package (`@nymproject/smolmix-wasm`) keep their names via `--out-name` and a
  `package.json` `name` pin, so `@nymproject/mix-tunnel` and its inlining wiring
  are untouched.
- **Example binaries kept**: `smoldvpn-config`/`-topup`/`-grpc` stay (they are
  `[[example]]` identifiers, not the crate, and were renamed once already).
- **BREAKING** (out-of-tree consumers only): the crates.io package and lib names
  change. `smol-core`/`smoldvpn`/`smolmix` remain published under the old names
  (yank handled at publish time); `smolmix-wasm` was never published.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `dvpn-tunnel`: requirement text naming the crate updated `smoldvpn` →
  `nym-smoldvpn` (deliverable name only; no behavioural change).
- `dvpn-quic-bridge`: requirement text naming the crate updated `smoldvpn` →
  `nym-smoldvpn` (deliverable name only; no behavioural change).
- `dvpn-tools`: requirement text naming the crate updated `smoldvpn` →
  `nym-smoldvpn`; the `smoldvpn-*` example CLI names are unchanged (deliverable
  name only; no behavioural change).
- `smol-core-stack`: requirement text naming the crate updated `smol-core` →
  `nym-smol-core` (deliverable name only; no behavioural change). `smolmix`
  keeps its product name in the refactor requirement.

## Impact

- **Code**: package/lib rename; `use` paths in `nym-smol-core`'s two consumers
  and each crate's own examples/tests; no behavioural source change.
- **Workspace**: root `Cargo.toml` dep-table keys + the wasm profile key; the
  `nym-smoldvpn` dep-table entry added (previously a TODO); `Cargo.lock`
  regenerates.
- **CI**: `ci-crates-version-bump.yml` mdx-snippet alternation `smolmix` →
  `nym-smolmix`; `wasm/smolmix/Makefile` `--out-name` + `mark-pkg-private`
  ordering/name-pin.
- **Docs**: the three crate READMEs, `documentation/docs/pages/developers/smolmix.mdx`
  (install/`-p`/docs.rs lines), and the `docs/design/smoldvpn/**` architecture
  docs.
- **Not affected**: `@nymproject/mix-*` published package names/APIs; the
  `smolmix` docs page URL and nav; `SmolCoreError`/`SmolmixError` type names;
  gateway/protocol behaviour.

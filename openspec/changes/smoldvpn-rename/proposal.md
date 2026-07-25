# Proposal: smoldvpn-rename

## Why

The dVPN datapath crate lives at `sdk/rust/smol-dvpn` as `nym-smol-dvpn`,
while its architectural sibling lives at the repo root as `smolmix` — two
different naming conventions and two different locations for the two
smol-core-based datapaths. Moving the crate to the repo root as `smoldvpn`
makes the pair symmetric: `smolmix` and `smoldvpn`, siblings at the root,
consistently un-prefixed and un-hyphenated. Doing it now is cheap: nothing
in-tree depends on `nym-smol-dvpn` (the dependency direction is
smoldvpn → nym-sdk-session, not the reverse), and no CI workflow references
the path.

## What Changes

- **Move** `sdk/rust/smol-dvpn/` → `smoldvpn/` (repo root, sibling of
  `smolmix/`), via `git mv` so history follows. Per the request the crate
  sits directly at `smoldvpn/` (not nested like `smolmix/core/`).
- **Rename** the package `nym-smol-dvpn` → `smoldvpn` — which also renames
  the lib target `nym_smol_dvpn` → `smoldvpn`, cascading into:
  - every `use nym_smol_dvpn::…` in examples and tests → `use smoldvpn::…`;
  - every `RUST_LOG` target: the examples' default filter
    `nym_smol_dvpn=info,boringtun=info` → `smoldvpn=info,boringtun=info`
    (tracing targets derive from module paths, so log output follows
    automatically);
  - every `cargo run -p nym-smol-dvpn --example …` invocation in docs.
- **Rename the `smol-dvpn-*` example binaries** `smol-dvpn-config`,
  `smol-dvpn-grpc`, `smol-dvpn-topup` → `smoldvpn-config`, `smoldvpn-grpc`,
  `smoldvpn-topup` (files, `[[example]]` entries, and their per-example data
  directories `data/smol-dvpn-*` → `data/smoldvpn-*`).
- **Update the root workspace**: member `"sdk/rust/smol-dvpn"` →
  `"smoldvpn"`.
- **Update all prose**: the crate README, `common/smol-core/README.md`,
  `nym-sdk-session` rustdoc/README mentions, and the mirrored design-doc
  directory `docs/design/sdk/smol-dvpn/` → `docs/design/smoldvpn/` (moved to
  mirror the new location, contents updated).
- **Update live openspec specs** that name the crate/binaries in requirement
  text (`dvpn-tunnel`, `dvpn-tools`, `dvpn-quic-bridge`) via MODIFIED
  deltas. Historical artifacts under `openspec/changes/` (including
  archives) are deliberately left untouched — they describe work as it
  shipped.
- **BREAKING** (for out-of-tree consumers only): the package and lib names
  change. In-tree there are no reverse dependencies. If `nym-smol-dvpn` was
  already published to crates.io (`publish = true` is set; publish status
  must be checked pre-implementation — the API was unreachable from this
  environment), the rename means a fresh crates.io name and the old crate
  should get a final deprecation-pointer release; if it was never published,
  there is nothing to do.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `dvpn-tunnel`: requirement text naming `nym-smol-dvpn` updated to
  `smoldvpn` (deliverable name only; no behavioral change).
- `dvpn-tools`: requirement text naming the crate and the `smol-dvpn-*`
  example CLIs updated to the `smoldvpn`/`smoldvpn-*` names (deliverable
  names only; no behavioral change).
- `dvpn-quic-bridge`: requirement text naming the crate updated
  (deliverable name only; no behavioral change).

## Impact

- **Code**: directory move + package/lib rename; `use` paths in
  `smoldvpn`'s own examples/tests; no source changes in any other crate
  beyond doc-comment mentions in `nym-sdk-session`.
- **Workspace**: root `Cargo.toml` members list; `Cargo.lock` regenerates.
- **Docs**: crate README, smol-core README, session README/rustdoc,
  `docs/design/sdk/smol-dvpn/` (moved + updated).
- **Users of the examples**: run commands change to
  `cargo run -p smoldvpn --example …`; the renamed `smoldvpn-*` examples
  write to new `data/smoldvpn-*/` directories — anyone with state under the
  old `data/smol-dvpn-*/` names keeps it by renaming the local directory
  (the shared-name examples `zcash-sync`, `two-hop-ip`, `two-hop-quic` are
  unaffected and keep their ticketbooks/registrations).
- **Not affected**: `common/` crates, `nym-sdk-session` code paths,
  gateway/protocol behavior, the `data/<example>/<network>` layout
  convention, CI (no workflow references the old path).

# Tasks: nym-prefix-smol-crates

## 1. Package renames (committed)

- [x] 1.1 `smol-core` → `nym-smol-core`: `[package] name`, workspace dep-table
      key, both consumer dep keys (`smolmix/core`, `smoldvpn`), all
      `use nym_smol_core::…`, crate READMEs
- [x] 1.2 `smolmix` → `nym-smolmix`: `[package] name`, workspace dep-table key,
      lib paths + `-p` flag, `ci-crates-version-bump.yml` alternation,
      `smolmix.mdx` functional lines (install / `-p` / docs.rs). Product name
      "smolmix" kept in prose/page/nav
- [x] 1.3 `smoldvpn` → `nym-smoldvpn`: `[package] name`, added the workspace
      dep-table entry (was a TODO), lib paths + `-p` flag + `nym_smoldvpn`
      `EnvFilter`/`RUST_LOG` target, README (migration note updated to the
      three-name lineage). Example binaries `smoldvpn-*` kept
- [x] 1.4 `smolmix-wasm` → `nym-smolmix-wasm`: `[package] name`, profile key,
      Makefile `--out-name smolmix_wasm` + `mark-pkg-private` name pin
      (`@nymproject/smolmix-wasm`). Verified: `pkg/package.json` name pinned +
      `private: true`; artifact basename `smolmix_wasm_bg.wasm`

## 2. Docs

- [x] 2.1 `docs/design/smoldvpn/{README,design}.md`: crate names →
      `nym-smoldvpn` / `nym-smol-core`; directory paths, `smoldvpn-*` example
      names, `smol-core-stack` capability name, and the `smolmix` product name
      left as-is. Pre-existing stale links (`sdk/rust/smoldvpn`, depth) fixed
- [x] 2.2 Live specs' Purpose lines (outside the delta grammar) updated directly
      to `nym-smoldvpn` / `nym-smol-core`; requirement bodies covered by this
      change's deltas (applied at archive time)

## 3. Spec deltas (this change)

- [x] 3.1 `dvpn-tunnel`: MODIFIED "Userspace WireGuard datapath with boringtun"
- [x] 3.2 `dvpn-quic-bridge`: MODIFIED "QUIC bridge client reimplemented inline"
- [x] 3.3 `dvpn-tools`: MODIFIED the five requirements naming the crate
      (config export, top-up, gRPC, public-IP, Zcash); `smoldvpn-*` example
      names unchanged
- [x] 3.4 `smol-core-stack`: MODIFIED "Transport-agnostic userspace TCP/IP
      stack" and "smolmix refactored onto nym-smol-core" (`smolmix` kept)

## 4. Verification

- [x] 4.1 `cargo clippy --all-targets` + `cargo test --doc` green for the three
      native renamed crates
- [x] 4.2 `make -C wasm/smolmix build` → `pkg/package.json` name
      `@nymproject/smolmix-wasm`, `private: true`
- [x] 4.3 Link check: zero broken links in `docs/` and `openspec/specs`
- [ ] 4.4 `openspec validate nym-prefix-smol-crates` passes (run before apply)

## 5. Apply / publish (post-merge)

- [ ] 5.1 `openspec apply nym-prefix-smol-crates` (propagates the deltas into the
      live specs' requirement bodies), then archive
- [ ] 5.2 Release train publishes the new crate names; then `cargo yank` the
      old-named versions (yank breadth for `smolmix` stable still to decide)

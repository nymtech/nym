# Tasks: smoldvpn-rename

## 1. Pre-flight

- [x] 1.1 Check crates.io: was `nym-smol-dvpn` ever published? — RESOLVED
      by the crate owner (2026-07-24): never published; the rename precedes
      first publication, so no deprecation follow-up exists. Recorded in
      design.md (D6). Check `smoldvpn` name availability at first-publish
      time
- [x] 1.2 Confirm a clean working tree (the move must be its own commit for
      rename detection)

## 2. Move + rename

- [x] 2.1 `git mv sdk/rust/smol-dvpn smoldvpn`
- [x] 2.2 Root `Cargo.toml`: members entry `"sdk/rust/smol-dvpn"` →
      `"smoldvpn"` (keep the members list sorted per its existing order)
- [x] 2.3 `smoldvpn/Cargo.toml`: `name = "smoldvpn"`; update the description
      if it spells the old name; rename the `[[example]]` entries and files
      `examples/smol-dvpn-{config,grpc,topup}.rs` →
      `examples/smoldvpn-{config,grpc,topup}.rs` (git mv)
- [x] 2.4 Apply the three-token rewrite (design D3: `nym-smol-dvpn` →
      `smoldvpn`, then `nym_smol_dvpn` → `smoldvpn`, then `smol-dvpn` →
      `smoldvpn`) across the moved crate: src/, examples/ (incl. the
      `data/smoldvpn-*` dir strings and `RUST_LOG` default filters),
      tests/, README.md, .gitignore comments
- [x] 2.5 Same rewrite on the known external references:
      `common/smol-core/README.md`, `sdk/rust/nym-sdk-session/README.md`,
      `nym-sdk-session` doc comments (`src/lib.rs`, `src/session.rs`,
      `src/registration_cache.rs`)
- [x] 2.6 `git mv docs/design/sdk/smol-dvpn docs/design/smoldvpn`, apply the
      rewrite to its README.md + design.md, and remove `docs/design/sdk/`
      if now empty
- [x] 2.7 Update the live specs' Purpose lines that name the crate
      (`dvpn-tunnel`, `dvpn-tools`) directly — Purpose is outside the
      delta grammar; requirement bodies are covered by this change's deltas
      at archive time

## 3. Verification

- [x] 3.1 `grep -rn 'nym-smol-dvpn\|nym_smol_dvpn\|smol-dvpn' --exclude-dir
      target --exclude-dir .git .` → remaining hits ONLY under
      `openspec/changes/` (historical artifacts, deliberately untouched)
      and `Cargo.lock` history if any
- [x] 3.2 `cargo test -p smoldvpn --lib --tests` and
      `cargo test -p nym-sdk-session` green;
      `cargo clippy -p smoldvpn -p nym-sdk-session --tests --examples`
      clean; `Cargo.lock` regenerated
- [x] 3.3 Spot-check the rename cascade: `cargo run -p smoldvpn --example
      zcash-sync -- --help` prints usage, and the examples' default log
      filter now spells `smoldvpn=info,boringtun=info`
- [x] 3.4 `git log --follow smoldvpn/src/tunnel.rs` shows pre-move history
      (rename detection intact)
- [x] 3.5 README migration note present: `data/smol-dvpn-*` →
      `data/smoldvpn-*` local rename preserves example credentials;
      `zcash-sync`/`two-hop-*` state unaffected; `RUST_LOG` target change
      called out for anyone with shell aliases

## 4. Manual validation (documented)

- [x] 4.1 One live run from the repo root against sandbox or mainnet
      (e.g. `cargo run --release -p smoldvpn --example two-hop-ip`)
      confirming provisioning reuses existing state under the unchanged
      `data/<example>/<network>` dirs and logs appear under the new
      `smoldvpn::…` targets — VALIDATED 2026-07-24 (the run that also
      completed dvpn-registration-reuse 5.3: cached registrations reused
      post-rename, no ticket spent)

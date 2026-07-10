# Design: smoldvpn-rename

## Context

Scouted facts the plan builds on:

- `smolmix` (the consistency target) is workspace member `"smolmix/core"`,
  package name `smolmix` — root-level, no `nym-` prefix. The request places
  the crate directly at `smoldvpn/` (flat, not `smoldvpn/core/`).
- No in-tree crate depends on `nym-smol-dvpn`: the root `Cargo.toml`
  mentions it only in the members list (no `[workspace.dependencies]`
  entry), and the dependency direction is smoldvpn → nym-sdk-session.
- No `.github/` workflow references the path or crate name.
- References needing update, by kind:
  - root `Cargo.toml` members list (1 line);
  - the crate itself (`Cargo.toml` name/description, `[[example]]` names);
  - `use nym_smol_dvpn::…` in the crate's examples and tests;
  - `RUST_LOG` default filters and doc text in examples + README
    (`nym_smol_dvpn=…` targets);
  - prose mentions in `common/smol-core/README.md`,
    `sdk/rust/nym-sdk-session/README.md`, and `nym-sdk-session` doc comments
    (`lib.rs`, `session.rs`, `registration_cache.rs`);
  - `docs/design/sdk/smol-dvpn/{README.md,design.md}`;
  - live specs: `dvpn-tunnel` (2 mentions), `dvpn-tools` (9),
    `dvpn-quic-bridge` (3).
- crates.io status of `nym-smol-dvpn` could not be verified from this
  environment (API blocked); `publish = true` is set and the crate was
  written to be publishable.

## Goals / Non-Goals

**Goals:**

- `smoldvpn/` at the repo root, package + lib named `smoldvpn`, examples
  named `smoldvpn-*` — fully consistent with `smolmix`.
- Every in-repo reference (code, docs, specs, run commands, log-filter
  examples) updated in one change; `git log --follow` history preserved.
- Green build: full test suites + clippy for `smoldvpn` and
  `nym-sdk-session` after the move.

**Non-Goals:**

- Renaming anything else for symmetry (e.g. `nym-sdk-session`,
  `smol-core`) — out of scope.
- Restructuring into `smoldvpn/core/` to mirror smolmix's nesting — the
  request is explicit about the flat layout.
- Rewriting historical openspec change artifacts or archives — they
  describe shipped work under its then-current names.
- Publishing/yanking on crates.io — implementation only *checks* the
  status and records what follow-up (if any) the owner should do.

## Decisions

### D1. `git mv` the whole directory, then rename in place

`git mv sdk/rust/smol-dvpn smoldvpn` first (history follows renames with
`--follow`/rename detection), then edit names inside the moved tree. One
commit for the whole change keeps rename detection intact (same-commit
content edits are fine; git's similarity detection tolerates them).

### D2. Package, lib, and example names all drop the prefix and hyphen

- `[package] name = "smoldvpn"` — lib target becomes `smoldvpn`
  automatically (no explicit `[lib] name` needed, matching smolmix).
- Example files + `[[example]]` entries: `smol-dvpn-config` →
  `smoldvpn-config`, `smol-dvpn-grpc` → `smoldvpn-grpc`,
  `smol-dvpn-topup` → `smoldvpn-topup`. Their `example_data_dir` names and
  the standalone data-dir strings follow (`data/smoldvpn-config/…` etc.),
  with a README migration note for anyone holding state under the old
  names. `zcash-sync`, `two-hop-ip`, `two-hop-quic`, `quic-probe` keep
  their names (no crate reference in them).

### D3. Mechanical rewrite by three exact tokens, then review the diff

The rename is three disjoint textual substitutions applied across the
moved crate + the known reference files (never repo-wide blind sed):

1. `nym-smol-dvpn` → `smoldvpn` (package name, `-p` flags, prose)
2. `nym_smol_dvpn` → `smoldvpn` (use paths, RUST_LOG targets, doc text)
3. `smol-dvpn` → `smoldvpn` (bare directory/asset mentions, example
   binary names — applied AFTER (1) so the longer token wins first)

Applied file-by-file to the inventory in Context, followed by a manual
diff review — the token `smol-dvpn` also appears inside historical
openspec change dirs, which are explicitly excluded.

### D4. Docs directory moves to mirror the new location

`docs/design/sdk/smol-dvpn/` → `docs/design/smoldvpn/` (git mv), contents
updated by D3. The old parent `docs/design/sdk/` is removed if it becomes
empty.

### D5. Spec deltas: MODIFIED requirements with full updated text

The three affected specs name the crate/binaries inside requirement bodies,
so each touched requirement is copied in full under `## MODIFIED
Requirements` with only the names swapped (per openspec delta rules; no
scenario semantics change). Requirement headers do not contain the crate
name, so no RENAMED entries are needed.

### D6. crates.io: RESOLVED — never published

Confirmed by the crate owner (2026-07-24): `nym-smol-dvpn` was **never
published** to crates.io — this rename lands before first publication, so
there is no deprecation follow-up and no legacy name to manage.
`publish = true` carries over to `smoldvpn`; the `smoldvpn` name's
availability on crates.io should be eyeballed at first-publish time.

### D7. Log-target continuity is a doc concern, not a code concern

Tracing targets derive from module paths, so `nym_smol_dvpn::tunnel` events
become `smoldvpn::tunnel` events with zero code changes beyond the rename.
What must be hand-updated is every place that *spells* the old target:
the examples' `init_logging` default filters, README logging docs, and any
`RUST_LOG=nym_smol_dvpn=debug` examples in doc comments. Users with shell
aliases/scripts using the old target need the README migration note.

## Risks / Trade-offs

- [Out-of-tree consumers of `nym-smol-dvpn` break] → Accepted and
  intentional (the rename is the point); the crate is young and in-repo
  usage is zero. Migration is `s/nym-smol-dvpn/smoldvpn/` +
  `s/nym_smol_dvpn/smoldvpn/`.
- [Blind-substitution damage from the short `smol-dvpn` token] → Mitigated
  by D3's explicit file inventory (no repo-wide sed), ordering (longest
  token first), and diff review before commit; historical openspec dirs
  excluded by construction.
- [Stale local example data dirs after binary renames] → README migration
  note; only affects the three renamed `smoldvpn-*` examples, not the
  mainnet ticketbook/registration state (which lives under `zcash-sync`,
  unchanged).
- [crates.io name squatting / prior publish unknown] → D6 makes it an
  explicit pre-flight check with recorded outcome rather than a silent
  assumption.
- [History readability] → `git mv` + single commit keeps `--follow` intact;
  the commit message documents the rename mapping for future archaeology.

## Migration Plan

Single commit, no data migration. Rollback = revert the commit. Local
developer state: rename `data/smol-dvpn-*` dirs to `data/smoldvpn-*` to
keep example credentials (documented in the README note); all other data
dirs are name-stable.

## Open Questions

_None — the crates.io publish status (D6) was confirmed by the owner:
never published; the rename precedes first publication._

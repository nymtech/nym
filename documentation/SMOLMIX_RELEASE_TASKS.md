# smolmix docs — crates.io release tasks

Branch: `max/smolmix-docs`

Preparing docs for the next code release when smolmix ships on crates.io.

---

## Version strategy (decided)

- **Components:** Shared `components/versions.ts` exporting all version constants. Both `CratesPaused` and `CodeVerified` import from it.
- **Code blocks:** Hardcode versions in fenced Cargo.toml blocks (copy-pasteable). `versions.ts` includes a comment listing which files need manual updates per release.
- **`llms-full.txt`:** Auto-generated on build — no manual update needed.
- **`llms.txt`:** Manually maintained — update version refs there.
- **blake3 pin:** Keep `=1.7.0` — digest conflict still present.

---

## Completed

- [x] **Add `tutorial-udp` to sidebar nav** — added to `_meta.json`
- [x] **Check if blake3 `=1.7.0` pin can be removed** — No, keep it
- [x] **Audit all pages for pre-release language** — found + fixed all instances
- [x] **Decide version strategy** — shared `versions.ts` + hardcoded code blocks
- [x] **Create `components/versions.ts`** — NYM_SDK_VERSION, SMOLMIX_VERSION, BLAKE3_PIN
- [x] **Update `CratesPaused` component** — now shows neutral version info callout (imports from versions.ts)
- [x] **Update `CodeVerified` component** — now shows crate version instead of commit hash (imports from versions.ts)
- [x] **Convert git rev → crates.io imports** in smolmix tutorials
- [x] **Update `importing.mdx`** — crates.io as primary install, git as bleeding-edge alternative
- [x] **Update other Rust tutorials** — mixnet, stream, tcpproxy, client-pool all converted
- [x] **Update "(upcoming)" language** — removed from smolmix.mdx and developers/index.mdx
- [x] **Fix "Lewes Protocol" reference** — stream/architecture.mdx updated
- [x] **Fix corrupted content** — removed terminal session artifacts from tutorial-udp.mdx
- [x] **Update `llms.txt`** version reference
- [x] **Update frontmatter dates** — all modified pages set to 2026-04-17
- [x] **Remove anyhow dependency** — replaced with BoxError pattern in smolmix TCP tutorial, tcpproxy, client-pool
- [x] **Replace Public API section with docs.rs link** — smolmix.mdx now points to docs.rs

## Before release

- [ ] **Replace `X.Y.Z` placeholders with real version numbers** — search for `X.Y.Z` in:
  - `components/versions.ts` (NYM_SDK_VERSION, SMOLMIX_VERSION)
  - All tutorial Cargo.toml code blocks (listed in versions.ts comment)
  - `public/llms.txt`
- [ ] **Modify logging in smolmix-udp tutorial** — `tutorial-udp.mdx` logging setup needs updating (both step-by-step and complete code blocks)
- [ ] **Build docs site + visual check** — `pnpm run build`, verify sidebar, code blocks, links

---

## Diataxis improvements (post-release or as time allows)

Based on a Diataxis framework assessment. Current docs are strong on **Tutorials** and have good **Explanation** fragments, but are missing **How-to guides** entirely and have incomplete **Reference**.

### Short-term

- [ ] **Add CodeVerified callout to smolmix tutorials** — TCP + UDP tutorials lack the version-verification callout that all Rust SDK tutorials have
- [ ] **Trim Quick start section on landing page** — 20-line code block is too long for a teaser, too short for a tutorial. Trim to ~5 lines or replace with a link to Tutorial 1

### How-to guides (new pages)

- [ ] **How-to: WebSocket through the mixnet** — task-oriented guide based on `websocket.rs` example. Assumes reader knows smolmix basics.
- [ ] **How-to: Target a specific exit gateway** — expand the `--ipr` pattern into standalone guidance (why, how to find IPRs, TunnelBuilder usage)
- [ ] **How-to: Troubleshooting connections** — timeouts, DNS failures, gateway issues, RUST_LOG guidance. Equivalent of tcpproxy's troubleshooting section.

### Explanation (restructure)

- [ ] **Extract architecture + explanation into dedicated page** — move "Why IP", architecture diagrams, comparison table out of landing page into `smolmix/architecture.mdx`. Landing page becomes concise orientation.
- [ ] **Write security model explanation** — Sphinx encryption boundaries, IPR trust model, why TLS matters on the final hop, comparison with VPN/Tor exit trust

---

## Files changed

| File | What | Done |
|------|------|------|
| `components/versions.ts` | **NEW** — shared version constants | ✅ |
| `components/crates-paused.tsx` | Neutral version callout (was: "paused" warning) | ✅ |
| `components/code-verified.tsx` | Crate version (was: commit hash) | ✅ |
| `pages/developers/smolmix.mdx` | Removed "(upcoming)", API→docs.rs, updated date | ✅ |
| `pages/developers/smolmix/tutorial.mdx` | crates.io imports, anyhow→BoxError, updated date | ✅ |
| `pages/developers/smolmix/tutorial-udp.mdx` | crates.io imports, fixed corruption, updated date | ✅ |
| `pages/developers/smolmix/_meta.json` | Added tutorial-udp to sidebar | ✅ |
| `pages/developers/index.mdx` | Removed "upcoming" | ✅ |
| `pages/developers/rust/importing.mdx` | crates.io-first install guide, updated date | ✅ |
| `pages/developers/rust/mixnet/tutorial.mdx` | crates.io imports, updated date | ✅ |
| `pages/developers/rust/stream/tutorial.mdx` | crates.io imports, updated date | ✅ |
| `pages/developers/rust/stream/architecture.mdx` | Removed "Lewes Protocol" reference | ✅ |
| `pages/developers/rust/tcpproxy.mdx` | crates.io imports, anyhow→BoxError, updated date | ✅ |
| `pages/developers/rust/client-pool.mdx` | anyhow→BoxError | ✅ |
| `pages/developers/rust/client-pool/tutorial.mdx` | crates.io imports, updated date | ✅ |
| `public/llms.txt` | Updated version reference | ✅ |

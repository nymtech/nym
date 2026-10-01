# Tasks

## 1. Phase 1 — cleanup (done)

- [x] 1.1 Delete post-process glue (`post_process.sh`, `scripts/post-process/`) — no workflow caller
- [x] 1.2 Delete `backup-pages/` and the duplicate `scripts/cmdrun/api_targets.py` (+ config)
- [x] 1.3 `python-prebuild.sh`: drop the duplicate `nym-node-cli-install-help.md` write
- [x] 1.4 `api-scraping/api_targets.py`: remove the broken, unused `nym_vpn` path
- [x] 1.5 autodoc crate: drop nym-cli + both client dumps; keep nym-node/nym-api/nymvisor
- [x] 1.6 `autodoc.sh`: build only the three packages; drop `git checkout master` + commit/push
- [x] 1.7 Add `ci-docs-autodoc.yml` (path-filtered, owns the commit-back)
- [x] 1.8 Move the four top-level `--help` captures from `python-prebuild.sh` into `autodoc.sh` (predev builds no Rust)
- [x] 1.9 Drop 20 orphaned nym-client/socks5 command snippets; regenerate the current ones
- [x] 1.10 `next.config.js`: remove 17 duplicate redirect rules (behaviour-preserving); resolve 3 conflicting redirects
- [x] 1.11 Reduce `cargo build --workspace --release` to the three `-p` packages in ci-docs/cd-docs (kept, not removed, per decision)
- [x] 1.12 autodoc builds **release** (build-info profile correctness)

## 2. Phase 2 — retrieval seam (done)

- [x] 2.1 `PAGES_DIR` honours `DOCS_CONTENT_DIR` (default `content`); route `generate-typedoc-meta.mjs` through it
- [x] 2.2 Prove the index builds from an arbitrary content dir (offline test)

## 3. Phase 3 — Nextra 4 App Router (done)

- [x] 3.1 Dep bump: nextra/nextra-theme-docs 4.6.1, react/react-dom 19, drop @coreui; drop @nextui-org (port accordion to MUI)
- [x] 3.2 Content move `pages/` -> `content/`; `_meta.json` -> `_meta.js` (74); remove 5 stale `_meta` keys
- [x] 3.3 `next.config.js` -> Nextra 4 wrapper (`require('nextra').default`); keep webpack block (Railgun polyfills)
- [x] 3.4 `mdx-components.jsx`; `app/layout.tsx`; `app/(docs)/layout.tsx`; `app/(docs)/seo.ts`; `app/(docs)/[[...mdxPath]]/page.tsx`; delete `theme.config.tsx` + `pages/_app.tsx`
- [x] 3.5 `"use client"` on ~27 interactive components
- [x] 3.6 redoc -> `"use client"` ssr:false wrapper; demo `dynamic(ssr:false)` -> `components/client-demos.tsx`; inline MDX snippets -> `components/operators/mdx-client-snippets.tsx`
- [x] 3.7 `PageActions` -> `next/navigation`; tsconfig `moduleResolution: bundler` + `strictNullChecks: false`; `outputFileTracingRoot` pinned; Footer children; Wrapper `{...rest}`
- [x] 3.8 Pin `zod` to 4.1.12 via `pnpm-workspace.yaml` (nextra-theme-docs 4.6.1 Layout/children bug)
- [x] 3.9 `next build` green: 272/272 pages, `tsc` clean

## 4. Build / search / UX (done)

- [x] 4.1 Pagefind wired into the build
- [x] 4.2 Turbopack for dev; single-instance ethers/shared-models aliases; Railgun demo verified end-to-end under Turbopack
- [x] 4.3 Turbopack as default `build`; webpack kept as `build:webpack`
- [x] 4.4 Right-side TOC restored (removed leftover `.nextra-toc` hide); "Use with AI" hidden on landing
- [x] 4.5 Research open items resolved (Nextra 4.6.1 App-Router-only; no built-in llms.txt/per-page .md; move-to-dist stays)
- [x] 4.6 Fresh code review; findings triaged and fixed

## 5. Remaining — follow-on work (not in this change)

### Python cleanse
- [ ] 5.1 Supply/reward figures -> runtime ISR components (stop writing live network numbers into MDX at build). This retires `api_targets.py` `validator`/`calculate`, pandas, tabulate, requests, and the `curl | jq` calls. Lands with a shared TS format helper (u-nym -> NYM).
- [ ] 5.2 `csv2md.py` -> a `<CsvTable>` component or a small Node transform (removes pandas/tabulate from the CSV tables)
- [ ] 5.3 `described_nodes` -> ask nym-api for a summary endpoint, then ISR (cross-team; blocked until it ships). Until then it stays build-time.
- [ ] 5.4 The two argparse `--help` captures (`node_api_check.py`, `nym-node-cli.py`): port to Node/Rust or freeze a snapshot, to remove the last python3 from the docs pipeline (owned by the teams that own those tools)

### TypeScript / component hygiene
- [ ] 5.5 Convert remaining `.jsx`/`.mjs` to `.tsx`/typed modules where practical (mdx-components, reward-calculator.jsx, the retrieval generators) — "move to TS components when we can"
- [ ] 5.6 Consider further UI-library consolidation now that NextUI is gone (MUI + emotion is the remaining heavy client dep; the plan wanted one UI library)

### Framework / infra
- [ ] 5.7 Port the MCP route `pages/api/mcp.ts` -> `app/api/mcp/route.ts` (deferred: the SDK transport is Node-shaped; needs an adapter). Update the `outputFileTracingIncludes` key with it.
- [ ] 5.8 Unpin `zod` once nextra-theme-docs fixes the Layout/children validation bug upstream (file an issue with the repro)
- [ ] 5.9 Add a CI lint that fails on a duplicate redirect `source`, so the table cannot re-bloat
- [ ] 5.10 Confirm `public/_pagefind` reaches the Vercel deploy (`move-to-dist.sh` only rsyncs `.next/server`); adjust the deploy if search is missing in prod
- [ ] 5.11 Regenerate the committed `*-build-info.md` / command snippets under release (CI autodoc will on its next path-filtered run; or run `pnpm run generate:commands` once)
- [ ] 5.12 Re-evaluate the kept (currently dead) `cargo build -p … --release` step in ci-docs/cd-docs; remove if nothing starts consuming the binaries
- [ ] 5.13 Optional: port the Railgun webpack polyfill block to turbopack and retire `build:webpack` (Railgun now works under turbopack via the aliases; the webpack block is only the fallback's reason to exist)

### Docs content (deferred by the plan)
- [ ] 5.14 `next.config.js` redirect-table dedupe at scale + the anti-rebloat lint (5.9)
- [ ] 5.15 Decide on `nymvisor-init-daemon.md` / other hand-written committed snippets vs regeneration

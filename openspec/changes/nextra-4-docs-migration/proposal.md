## Why

The docs ran `next@15.5` on `nextra@2` + `nextra-theme-docs@2` using the Next.js **pages router**, a pairing Nextra no longer supports. Hand-written prebuild scripts held logic the framework should own, and the build needed Python, pandas, jq, curl and a full Rust workspace build before `next dev` would start. `documentation/NEXTRA4-DEFERRAL-PLAN.md` sorted the tree into keep/replace/delete/defer and set the move to Nextra 4 (App Router only, React 19) as the goal, with the scripts as the small part and the client components as the large part.

This change records the migration as executed on branch `docs/nextra4-phase1`: Phase 1 (framework-independent cleanup), Phase 2 (decoupling the retrieval generators from `pages/`), and Phase 3 (the Nextra 4 App Router move itself), plus the build-tooling switch to Turbopack and Pagefind search. It is a spec-and-record artifact for work already on the branch; the normative source is the commits (`417f152562..HEAD`, 36 commits).

## What Changes

**Phase 1 — cleanup (no framework risk)**
- Delete mdbook-era post-process glue (`post_process.sh`, `scripts/post-process/`), `backup-pages/`, and the stripped duplicate `scripts/cmdrun/api_targets.py`.
- `python-prebuild.sh`: drop the duplicate `nym-node-cli-install-help.md` write; remove the broken, unused `nym_vpn` path from `api-scraping/api_targets.py`.
- `autodoc`: drop nym-cli and both client dumps (dead output); build only `nym-node`/`nym-api`/`nymvisor`; drop the `git checkout master` + in-script commit/push; add `ci-docs-autodoc.yml` (path-filtered CI that owns the commit). Later switched the autodoc build to **release** so `build-info` does not publish `Profile: debug`.
- `next.config.js`: remove 17 byte-identical duplicate redirect rules (216 -> 199, behaviour-preserving) and resolve 3 conflicting-destination redirects (-> 196, all sources unique).
- Reduce the `cargo build --workspace --release` step in `ci-docs.yml`/`cd-docs.yml` to the three needed packages.

**Phase 2 — retrieval seam**
- `PAGES_DIR` (shared by `generate-index`/`generate-llms-txt`/`generate-page-markdown`) honours `DOCS_CONTENT_DIR`, so the content-dir move is an env change. Routed `generate-typedoc-meta.mjs` through it too. Verified the index builds from an arbitrary dir.

**Phase 3 — Nextra 4 App Router**
- Dependency bump: `nextra`/`nextra-theme-docs` 2 -> 4.6.1, `react`/`react-dom` 18 -> 19, drop `@coreui/react` (unused). MUI 5.18 already supports React 19 (no MUI major bump); drop `@nextui-org` (deprecated) and port its one accordion to MUI.
- Content: `pages/` -> `content/` (269 MDX); 74 `_meta.json` -> `_meta.js`; remove 5 stale `_meta` keys. Keep `pages/api/mcp.ts` as a hybrid pages-router API route.
- App Router: `next.config.js` -> Nextra 4 wrapper (`require('nextra').default`); `mdx-components.jsx`; `app/layout.tsx` (bare root) + `app/(docs)/layout.tsx` (docs chrome) + `app/(docs)/seo.ts` (metadata + full JSON-LD) + `app/(docs)/[[...mdxPath]]/page.tsx` (catch-all), mirroring the codex reference; delete `theme.config.tsx` and `pages/_app.tsx`.
- `"use client"` on ~27 interactive components; redoc and the demo `dynamic(ssr:false)` components moved into `"use client"` wrappers/barrels; inline MDX snippets moved into a `"use client"` module; `PageActions` switched from `next/router` to `next/navigation`.
- tsconfig `moduleResolution: bundler`; pin `outputFileTracingRoot`; `strictNullChecks: false` (match develop).
- **The blocking fix**: pin `zod` to `4.1.12` via `pnpm-workspace.yaml` overrides. `nextra-theme-docs@4.6.1`'s `<Layout>` strips `children` then validates against a schema that still requires it; `zod@4.6.5` rejected the missing field (every page), `4.1.12` tolerates it.

**Build / search / UX**
- Pagefind wired into the build (Nextra 4 search). Turbopack for dev, then **Turbopack as the default `build`** (webpack kept as `build:webpack`), after verifying the Railgun demo runs end-to-end under Turbopack with single-instance `ethers`/`@railgun-community/shared-models` aliases. Right-side TOC restored (removed a leftover `.nextra-toc` hide); "Use with AI" hidden on the landing page.

## Impact

- **Affected**: `documentation/docs/**` (framework, app/, content/, components/, config), `documentation/scripts/**`, `documentation/autodoc/`, `.github/workflows/{ci-docs,cd-docs,ci-docs-autodoc}.yml`, docs dependencies and lockfile.
- **Build/deploy**: default build is Turbopack (~47s vs ~80s); CI (`pnpm run build`) and the Vercel deploy (runs the `build` script) follow. `move-to-dist.sh` stays (it feeds the Vercel `--prebuilt` deploy and the S3 preview, not a standard flow).
- **Verified**: `next build` green (272/272 pages) on both Turbopack and webpack; `tsc` clean; Railgun demo runs end-to-end under Turbopack; Pagefind indexes 269 pages.
- **Non-goals / deferred**: see `tasks.md` "Remaining". The `zod` pin is a workaround pending an upstream nextra-theme-docs fix.

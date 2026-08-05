# AI assistant + MCP: worklog & test runbook

Working scratchpad for the docs AI chat + developer MCP server. Design lives in
`ai-assistant-mcp-plan.md`; this file tracks what's been built and how you test
it. Branch: `max/docs-ai-assistant-mcp` (off `max/docs-threat-model-overhaul`).

## Features (draft for the docs README)

User-facing summary of what this branch adds. Four features over one shared
retrieval backend.

- **Ask AI (in-docs chat).** A floating assistant on every docs page. It answers
  from the documentation using retrieval-augmented generation: your question is
  embedded, the most relevant doc sections are retrieved, and Claude answers from
  those with inline `[n]` citations, streamed token by token. It only answers from
  the docs corpus, and says so when it cannot find something rather than guessing.

- **MCP server for AI agents.** A single endpoint, `https://nym.com/docs/api/mcp`,
  that developers point their coding agent at (Claude Code, Cursor, and others).
  It exposes the docs and live network state as structured tools, so an agent can
  search the docs and read current network data without leaving its editor.
  Eight tools:
  - `search_docs`, `get_section`: semantic search over the docs with deep-link
    citations, and full-section fetch.
  - `network_summary`, `circulating_supply`, `chain_status`, `list_gateways`,
    `get_gateway`: live data from the Nym APIs at call time (the thing a static
    `llms.txt` cannot give you).
  - `validate_sdk_config`: checks a mix-tunnel / mix-fetch config for typos,
    type errors, and privacy tradeoffs before you write it into code.

- **Per-page Markdown.** Every docs page is fetchable as clean Markdown by
  appending `.md` to its URL (e.g. `nym.com/docs/developers/mcp.md`), so any agent
  or script can ingest a page without scraping HTML. Together with the existing
  `llms.txt` / `llms-full.txt`, this makes the docs machine-readable end to end.

- **Retrieval backend (shared).** A build-time semantic index over the docs
  (Voyage embeddings, cosine search, no vector database). Built during the docs
  deploy and served from the app; both the chat and the MCP server read it. A
  content-hash cache means only changed sections re-embed on each build.

## What's built (chronological)

1. **Phase 0 spike** - `scripts/next-scripts/generate-index.mjs`. Chunks the docs
   corpus; proved 1394 chunks / ~5.5MB index / ~$0.006 per re-index.
2. **Phase 0.1 retrieval core** - `lib/retrieval/`:
   - `chunker.mjs` - heading-scoped, fence-aware, `-1/-2` anchor de-dup, hard-cap
     splitter, skips generated fig-spec noise.
   - `retrieval.ts` - cosine search with a source filter, plus `getSection`.
   - `embed.mjs` - pluggable provider (Voyage default), content-hash cache so only
     changed chunks re-embed.
   - Generator now imports the shared chunker; fixed a `stripMdx` bug that deleted
     `import` lines inside code examples.
3. **Live Nym API client** - `lib/nym-api/client.ts` (+ `live-check.mjs`). Supply,
   chain status, network summary, gateway list + by-id. Endpoints verified against
   production.
4. **MCP tool layer** - `lib/mcp/tools.ts`. 8 tools (search_docs, get_section, 5
   live, validate_sdk_config) with error-wrapping. `validate-config.ts` checks a
   SetupMixTunnelOpts object: field types (errors), unknown/typo'd keys (warnings,
   never errors, since the field list is a one-version snapshot), and privacy
   tradeoffs. The Streamable HTTP transport is under `lib/mcp/scaffold/` (needs
   the SDK; excluded from build + tests): `build-server.ts` (reusable core),
   `mcp-route.ts` (pages-router API route, recommended), `standalone.ts` (separate
   Node server, alternative). Verified against @modelcontextprotocol/sdk 1.30.0
   types: the low-level `Server` passes our JSON-Schema `inputSchema` through
   verbatim, so it needs only the SDK (no zod, no mcp-handler).
5. **Chat logic** - `lib/chat/`: `context.ts` (retrieval -> context + citations,
   docs-only by default), `prompt.ts` (system prompt + guardrails). Route + widget
   are scaffold under `lib/chat/scaffold/` (need the AI SDK; unverified).
6. **Docs bug fix** - corrected stale API example paths in `pages/apis/nym-api.mdx`
   and `ns-api.mdx` (`/v1/mixnodes/active` -> `/v1/nym-nodes/rewarded-set`;
   `/api/v1/gateways` -> `/v2/gateways`).
7. **MCP live route** - `pages/api/mcp.ts` (Streamable HTTP) + core in
   `lib/mcp/build-server.ts`. Validated live: `tools/list` returns 8 tools,
   `tools/call` round-trips (validate + live network tools). Only `@modelcontextprotocol/sdk`
   (no zod). `validate_sdk_config` tool added (`validate-config.ts`).
8. **Deploy wiring (2c)** - `build` now runs `generate-index.mjs` +
   `generate-page-markdown.mjs` before `next build`; `next.config.js` traces
   `docs-index.json` into the `/api/mcp` lambda. Needs `VOYAGE_API_KEY` in Vercel
   + one deploy to confirm the traced read (not verifiable from the sandbox).
9. **Per-page markdown export** - `scripts/next-scripts/generate-page-markdown.mjs`
   emits `public/<path>.md` per page (fence-aware strip). Verified: 190 files, 0
   stray top-level imports, code fences preserved. Output gitignored.
10. **Code retrieval index** - `lib/retrieval/code-chunker.mjs` (boundary-aware,
    tested) + `scripts/next-scripts/generate-code-index.mjs`. Separate index over
    curated source (sdk/, wasm/, examples, `common/nymsphinx`, `common/smol-core`)
    built with `voyage-code-3`; exposed as the `search_code` MCP tool with GitHub
    deep links. 348 files -> 6023 chunks (~12 MB vectorless; ~80 MB vectored, so
    int8 quantization is a future lever). Route loads it optionally + a code
    embedder. Fixed a hard-split infinite loop on single lines over the char cap.

Test coverage: 71 tests across `lib/**`, all passing (includes the privacy-model
tests from the base branch). Every SDK-coupled file now lives under
`lib/**/scaffold`, which tsconfig excludes from `next build`, so the branch keeps
building without the extra deps. (Earlier `lib/mcp/server.ts` was NOT under
scaffold/ and would have broken the build; it was split into the three scaffold
files above.)

## How to test (when you have time)

All commands run from `documentation/docs/`.

### 1. Unit tests (no network, no keys)
```
node_modules/.bin/vitest run lib/
```
Expect 71 passing. This covers the chunker, code chunker, retrieval, embed cache, Nym client
(hermetic), MCP tool logic (incl. validate_sdk_config), and chat context/prompt.

### 2. Live Nym APIs (network, no keys)
```
node lib/nym-api/live-check.mjs
```
Hits the real supply / summary / gateway endpoints. Confirms the MCP live tools
still map to production. Fails loudly if an endpoint moved.

### 3. Build a real index (needs Voyage key)
```
VOYAGE_API_KEY=xxx node ../scripts/next-scripts/generate-index.mjs
```
Writes `public/docs-index.json` (~5.5MB, gitignored) with vectors + an embed cache
under `.cache/`. Re-run: only changed chunks re-embed. Without the key it writes a
vectorless index and warns.

### 4. Chat route + widget (needs AI SDK + keys) - scaffold rewritten for v7

The scaffold under `lib/chat/scaffold/` is now written for **ai@7 + @ai-sdk/react@4**
(verified against the installed type defs), not the old v4 API. To wire it live:
```
pnpm add ai @ai-sdk/anthropic @ai-sdk/react
```
- Move `lib/chat/scaffold/chat-route.ts` -> `pages/api/chat.ts`. Fix its relative
  imports the same way `pages/api/mcp.ts` did (`../../lib/chat/...`, `../../lib/retrieval/...`).
- Move `lib/chat/scaffold/ChatWidget.tsx` -> `components/ChatWidget.tsx`, and
  mount `<ChatWidget />` in `pages/_app.tsx` inside the ThemeProvider.
- `/api/chat` is already in `next.config.js` `outputFileTracingIncludes` (reads the
  same index).
- Set `ANTHROPIC_API_KEY`, `VOYAGE_API_KEY`; optional `CHAT_MODEL`
  (default `claude-haiku-4-5`; set `claude-opus-4-8` for quality).
- `pnpm run dev`, open the docs, click "Ask AI".
- Untestable in the sandbox (needs deps + build). Verify: streaming renders via
  `pipeUIMessageStreamToResponse` + `useChat`; message text comes from
  `message.parts`. Citations-as-links are a follow-up (v7 transport hides response
  headers; the model cites `[n]` inline for now).

### 5. MCP server - VALIDATED WORKING (2026-08-04)

Just the SDK: the low-level `Server` takes JSON Schema, so no zod / mcp-handler.
`build-server.ts` is the reusable core (now in `lib/mcp/`); `pages/api/mcp.ts` is
the live route; `scaffold/standalone.ts` is the separate-deploy alternative.

Setup that was run (dev server, one dep):
```
pnpm add @modelcontextprotocol/sdk
mkdir -p pages/api               # mv will NOT create the parent dir
# route imports reach into lib/ from pages/api/ (../../lib/mcp/...); fixed on move
pnpm run dev
```
Smoke tests (note the dual Accept header, else 406):
```
curl -sS -X POST http://localhost:3000/docs/api/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```
Results:
- `tools/list` -> all 8 tools with correct JSON Schema. The `@hono/node-server`
  wrapped-`res` risk did NOT materialise; the transport works as a pages route.
- `tools/call validate_sdk_config` -> error + typo-warning + privacy note.
- `tools/call network_summary` -> live counts from production.
- Reply arrives as SSE framing (`event: message` / `data:`), the Streamable HTTP
  streaming path.
- Not yet tested: `search_docs` / `get_section` (need `VOYAGE_API_KEY` + a
  vectored index). Standalone alt (separate deploy): `tsx lib/mcp/scaffold/standalone.ts`.
- Still open: rate-limiting/abuse policy on the public endpoint (plan D3).

## Key handling (secrets)

Keys live only in two external stores, never in the repo or a working file. The
code reads them via `process.env.*`; no value is ever committed or handed to a
local agent.

- `VOYAGE_API_KEY` (build-time index embedding): a **GitHub Actions secret**
  (repo Settings -> Secrets and variables -> Actions). Referenced as
  `${{ secrets.VOYAGE_API_KEY }}` in `.github/workflows/cd-docs.yml`. Without it
  the build writes a vectorless index (build still passes; `search_docs` returns
  nothing).
- `VOYAGE_API_KEY` + `ANTHROPIC_API_KEY` (runtime: query embedding + Claude
  generation): **Vercel project env vars** (Project -> Settings -> Environment
  Variables; scope Preview for staging, Production later). Pulled by
  `vercel pull` at deploy. `ANTHROPIC_API_KEY` is runtime-only (not needed at build).

Local dev caveat: `.env` / `.env*.local` are gitignored, so a key there is never
committed, but any file under the repo tree is still readable by tooling with
filesystem access. For local testing prefer exporting in your shell for the
session (`VOYAGE_API_KEY=... pnpm run dev`) or keep the env file outside the repo
(e.g. `~/.config/nym-docs.env`) and source it. Better: exercise keys only through
the staging preview, where they stay in GitHub/Vercel.

## Sequenced backlog (agreed 2026-08-04)

Get it working, then make it nice, then widen the corpus. In order:

1. **Functional validation (in progress).** Chat streaming end to end locally.
   MCP already validated (tools/list, tools/call, live tools, search once vectors
   exist). Gotcha found: `next dev` snapshots env at start, so `VOYAGE_API_KEY` /
   `ANTHROPIC_API_KEY` must be in the shell that launches dev (or inline:
   `VOYAGE_API_KEY=... ANTHROPIC_API_KEY=... pnpm run dev`); exporting after the
   server is up does nothing.
2. **UI polish (after 1).** Productionise `ChatWidget` (currently a placeholder):
   - Open as a right-hand **sidebar drawer** (reference: LangChain docs
     `use-these-docs`), full height, slide-in, rather than the bottom-right box.
   - Add an **"Ask AI" trigger in the top navbar**, left of the search bar. This
     needs `theme.config.tsx` navbar customisation; verify what the installed
     `nextra-theme-docs` actually supports for injecting left of search before
     wiring (may need a search-component override or CSS ordering).
3. **Index the codebase - DONE (machinery; needs a keyed build to go live).** A
   separate `voyage-code-3` code index, exposed as the `search_code` MCP tool, chat
   stays docs-only. Scope (the `ROOTS` list in `generate-code-index.mjs`): sdk/,
   wasm/, examples, `common/nymsphinx`, `common/smol-core`. Chunker + generator +
   tool + route wiring built and tested; run `generate-code-index.mjs` with a key
   to build the vectors. Follow-ups: int8 quantization for the ~80 MB vectored size;
   widen/trim scope; a `search_code` handler test.
4. **Validate docs against the indexed code - PROTOTYPE BUILT (sizes only).**
   Because we index the source alongside the prose, we can turn retrieval around and
   use the code as an oracle to catch docs that have drifted from it. Cross-check
   factual claims (constant values, packet/buffer sizes, API signatures, config
   fields, endpoint paths, CLI flags) against the actual source and flag
   contradictions.

   Motivating case, now confirmed against source: `packet-anatomy.mdx` said Sphinx
   packets are "2000 bytes". Verified via `common/nymsphinx` +
   `sphinx-packet =0.6.0` (re-exported by `nym_sphinx_types`):
   `REGULAR_PACKET_SIZE = 2*1024 + HEADER_SIZE(348) + PAYLOAD_OVERHEAD_SIZE(17)` =
   **2413 bytes**, of which the plaintext payload is **2048** (2 KB).

   IPR layering (Max caught a first-pass mistake here). Mixnet mode tunnels IP, so
   the 2048 plaintext holds: fragmentation header (7 B) + IP-packet-router framing
   (2 B length prefix per packet + ~5 B bincode-varint `IpPacketRequest` wrapper,
   `common/ip-packet-requests`) + the IP packet + padding. The IPR caps its bundle
   at `MAX_PACKET_SIZE = 1500` (codec.rs), NOT 2048, because the Sphinx packet also
   carries the SURB/MixAck ("can't just use 2kb"). So usable IP bytes/packet is
   **~1498**, and ~548 B of every plaintext is reserved-plus-padding. First pass
   wrongly deleted the IPR framing segment and bumped usable to 2041; the old
   "~1570 usable" was actually roughly right. Corrected model keeps both the
   fragmentation header AND the IPR framing, caps the chunk at 1498, on-wire stays a
   constant 2413 (+8 WS). Fixed: prose (two sites), `packets.ts` model (every size
   now traces to `common/nymsphinx` or `common/ip-packet-requests`), component
   legend/summary. Cross-check bonus: `network/cryptography/sphinx.md` already said
   "2048 bytes" and was correct all along - two pages, one right, one drifted, code
   settles it.

   Prototype: `scripts/next-scripts/validate-docs-vs-code.mjs` (+ `--selftest`,
   tested in `lib/retrieval/validate-docs-vs-code.test.ts`). Deterministic: extracts
   byte/KB size claims (binding each number to the noun it modifies, e.g. "2 KB
   payload", not nearest-keyword), diffs against a small source-anchored oracle
   (`SIZE_FACTS`), gates on a same-sentence "sphinx" context to avoid false
   positives on common nouns (LP-frame fields, WireGuard MTU). Scans
   `pages/**` + `lib/privacy-model` only (not `.tsx`, so `PacketAnatomy.tsx`'s
   hardcoded legend/summary strings stay unguarded); 12 claims agree, 0 drift.

   Oracle now DERIVED from source (closed the hand-typed gap): `deriveConstants`
   reads the in-repo Rust consts (`packet_sizes.rs` REGULAR_PACKET_SIZE = 2*1024 +
   ...) with a tiny +/*/parens evaluator, and the two external sphinx-packet leaves
   (HEADER_SIZE 348, PAYLOAD_OVERHEAD_SIZE 17) are pinned with a version check that
   throws if `Cargo.toml`'s `sphinx-packet =0.6.0` moves. Fails loud on missing/
   renamed source, never validates against a stale value. `--show-oracle` prints
   the derivation.

   Oracle now DIMENSIONED (bytes + time), 6 facts, all source-derived: Sphinx
   geometry (2413/2048/348/17), IPR bundle cap `DEFAULT_IPR_TUN_MTU`=1500
   (network-defaults), reply-key age `DEFAULT_MAXIMUM_REPLY_KEY_AGE`=86400 s
   (config-types, read out of `Duration::from_secs(24*60*60)`). Dimensions never
   cross-match (a "512 bytes" cannot satisfy a time fact). Skipped `validity_epochs`
   (a struct field, 24/12/24 across configs, no single canonical const). Scan: 14
   claims agree, 0 drift.

   Survey (3 Opus agents over network/developer/API docs) found real drift to fix
   independent of tooling: D1 tunnel state names (docs+types.ts say disconnecting/
   disconnected, wasm `state.rs` serde emits shutting_down/shutdown - HIGH, breaks
   `state==='disconnected'`); D2 `SetupMixTunnelOpts` missing `preferredGateway`
   (stale TypeDoc @ commit 8ea9a230, off-by-one anchors, 4 copies); D3 `TunnelState.
   reason` typed string but runtime serialises an object; A1 nym-api openapi.json URL
   drops the `/api` prefix its sibling swagger link uses. Root cause of D1-D3: the
   committed TypeDoc `api/` pages are stale. Next validators (ranked): more in-repo
   constants (started); TypeDoc regen-and-diff CI check (retires D2/D3 class);
   OpenAPI path-existence vs served `/api-docs/openapi.json` (catches A1);
   wasm-serde-to-TS-union enum parity (catches D1); version/dist-tag; scan `.tsx`.

## PR final checks (before merge)
Do these before the PR merges; deferred out of the main work.

1. Republish the four TS SDK packages (mix-tunnel/fetch/dns/websocket) so the
   corrected `TunnelState` type (D1 names + D3 `reason` object) actually ships to
   npm. Runbook: root `ts-sdk-publishing.md`. Until this lands, the published
   `@nymproject/mix-tunnel@^0.1.0` still has the old type.
2. After republish: bump `documentation/docs/package.json` `@nymproject/mix-tunnel`
   to the new version, and restore `MixPlayground.tsx` to the discriminated-union
   access (`final.state === 'failed' ? final.reason.kind : ''`) it was reverted from.
3. Regenerate the committed doc artefacts fresh and commit: `pnpm run docs:typedoc`
   (needs `wasm/smolmix/pkg/` built + global typedoc) then `pnpm run build` in
   documentation/docs (regenerates llms-full.txt + indexes).
4. Dependency minimisation sweep (see the dep table below): remove
   `copy-webpack-plugin` (dead, commented out in next.config.js); confirm
   `raw-loader` / `vm-browserify` are still used; check if a later pnpm/nextra hoists
   gray-matter/github-slugger so the direct dep can drop.
5. Add the TypeDoc regen-diff gate (now that CI no longer regenerates): a check that
   `docs:typedoc` output equals the committed `api/` tree, so stale docs fail CI.
6. Add the two docs-validation workflows as branch-protection required checks (the
   linkcheck + any new gate) so they block merges.

## Dependencies added on this branch (for the end-of-branch minimisation sweep)
Every dep added on `max/docs-ai-assistant-mcp`, with why, so we can minimise at the
end. All in `documentation/docs/package.json` unless noted.

| Dep | Used by | Rationale | Minimisation note |
|---|---|---|---|
| `@modelcontextprotocol/sdk` | `pages/api/mcp.ts`, `lib/mcp/*` | MCP server (official SDK; replaced our hand-rolled server) | keep (core) |
| `ai` | `pages/api/chat.ts` | AI SDK core (`streamText`) for the chat | keep |
| `@ai-sdk/anthropic` | `pages/api/chat.ts` | Anthropic provider for the chat | keep |
| `@ai-sdk/react` | `components/ChatWidget.tsx` | `useChat` hook | keep |
| `vitest` | `*.test.ts` | unit tests | keep (dev-only) |
| `next-sitemap` | build | sitemap.xml / robots.txt | keep |
| `d3-scale` + `@types/d3-scale` | `components/threat-model/NetworkDiagram.tsx` | diagram scales | keep (viz) |
| `raw-loader` | `next.config.js` | import files as raw strings | verify still used |
| `vm-browserify` | `next.config.js` | webpack `vm` polyfill | verify still needed |
| `copy-webpack-plugin` | `next.config.js` (COMMENTED OUT) | was for copying wasm assets | **REMOVE candidate: unused/commented** |
| `gray-matter` | 3 generators via `lib/retrieval/mdx.mjs` (WIRED) | real YAML frontmatter parsing; replaced 3 non-head-anchored regex copies; fixes body-`---` eating content | keep |
| `github-slugger` | `chunker.mjs` (WIRED) | retrieval anchors match Nextra's rendered ones (via rehype-slug); replaced hand-rolled slugify + dedup Map | keep |

NOT added (pre-existing in develop, only version-bumped): `eslint`, `eslint-config-next`,
`typescript`. gray-matter + github-slugger are transitive deps of nextra but NOT
hoisted, so a direct entry is required to import them; at sweep time, check if a later
pnpm/nextra hoists them and the direct dep can drop.

## Validation workstream status board (living)
Single place to see what is done and what is outstanding for the docs-vs-code /
docs-rot work. Branch `max/docs-ai-assistant-mcp`. Keep this current.

### DECISION: custom checkers PROTOTYPED then REMOVED (prefer tools)
The custom drift-checkers were the Phase-1 AUDIT (find existing rot), not a permanent
guard. They did their job (found the "2000 bytes" size drift + the TunnelState name
mismatch) and were then removed in favour of generation/existing tools. Removed:
`validate-docs-vs-code.mjs`, `validate-enum-parity.mjs`, their two `.test.ts`, and
`.github/workflows/ci-docs-validation.yml`. The thinking is preserved in the wiki
(`docs-for-ai/checking-vs-projecting`). Prevention is now tools-only:
- wasm-boundary types -> tsify (done for TunnelState).
- plain shared Rust types -> ts-rs (common/types).
- SDK API reference -> TypeDoc regen (+ a regen-diff CI gate, still to add).
- links -> lychee (existing `ci-docs-linkcheck.yml`).
- hand-written prose facts -> no tool; rely on careful authoring (survey showed it
  holds; drift was in hand-typed constants + stale generated docs, not prose).

### Drifts found (3-agent survey) + status
- D1 TunnelState names (HIGH): FIXED at source (types.ts -> shutting_down/shutdown
  + tsify generation). Ships on SDK republish.
- D2 SetupMixTunnelOpts missing preferredGateway: source already has it. TypeDoc
  regen done for mix-tunnel's OWN api (preferredGateway now present). BUT the 3
  downstream copies (mix-fetch/mix-dns/mix-websocket) re-export the type via
  `workspace:*` and resolve it from mix-tunnel's BUILT `dist/esm/*.d.ts`, which is
  STALE (still has disconnecting/reason:string). Fix: build mix-tunnel
  (`scripts/build.sh`) BEFORE regenerating TypeDoc, then re-run the regen so the
  downstream copies pick up the fresh type. Gate implication: the TypeDoc
  regen-diff gate must build the SDKs before running typedoc, else it reproduces
  the stale-downstream state consistently (passes but docs cross-package-stale).
- D3 TunnelState.reason string -> FailureReason object: FIXED at source (types.ts
  discriminated union). Ships on republish.
- A1 nym-api openapi.json URL missing `/api` prefix: RESOLVED, not a drift. Curl
  confirmed both `/api-docs/openapi.json` and `/api/api-docs/openapi.json` return
  200, so the docs link works. Spec paths are `/v1`-relative; `/api` is a
  deployment prefix the docs correctly prepend.

### Code changes made (UNVERIFIED here; need user build/publish)
- `wasm/smolmix/src/state.rs` + `lib.rs`: tsify on TunnelState/FailureReason/
  TaskName; getTunnelState returns typed. Commit b79bda154f. VERIFIED: wasm build
  succeeded, `pkg/smolmix_wasm.d.ts` emits the correct discriminated union
  (`getTunnelState(): TunnelState`), matching the SDK's inline type. tsify handles
  the internally-tagged enum fine (the risk is cleared).
- `sdk/typescript/packages/mix-tunnel/src/types.ts`: TunnelState -> discriminated
  union (D1/D3). NEEDS tsc/build.
- `documentation/docs/components/playground/MixPlayground.tsx`: REVERTED to the
  published mix-tunnel type (`reason` as string). The docs build type-checks against
  published `@nymproject/mix-tunnel@^0.1.0` (npm), not source, so it cannot use the
  new `reason.kind` shape until mix-tunnel is republished and the docs dep bumped.
  This is the publish boundary: SDK source fixes only reach the docs via a republish.

### Outstanding USER actions (I cannot run these here)
1. Build smolmix wasm (`cd wasm/smolmix && make build-debug`), verify tsify emits
   `TunnelState` in `pkg/smolmix_wasm.d.ts`; fix any compile error (likely tsify).
2. Build/tsc the TS SDKs.
3. Regenerate TypeDoc (fixes D2, picks up D1/D3): `generate-typedoc.sh`.
4. Republish the four TS SDK packages (runbook: root `ts-sdk-publishing.md`).
5. Curl to settle A1 (which openapi.json URL 200s).
6. `pnpm test` in documentation/docs (vitest suites).

### Considered and dropped
- Source-link existence checker: DROPPED. Link resolution is lychee's job
  (`ci-docs-linkcheck.yml`). A custom local-tree check has branch-skew false
  positives (checks the working tree, not the linked ref). NOTE: CI lychee is
  `scheme = ["file"]`, so it only checks local links and SKIPS external github
  source links, those ~46 are currently unverified. If we want them checked, it's
  a lychee config toggle (add `https` to `scheme`), not custom code, with a real
  speed/flakiness/rate-limit tradeoff (likely why it's off), so probably a separate
  periodic CI job. Lesson: link RESOLUTION is lychee's; our niche is claim SEMANTICS.

### Outstanding VALIDATOR work (Phase 2; not built)
- OpenAPI path-existence vs served spec: DEPRIORITISED. The `apis/*.mdx` pages
  already embed the live spec via `<RedocStandalone>`, so endpoints are already a
  projection; only a few hand-written curl paths aren't, and they check out. Low
  marginal value. (If built: fetch spec, strip the `/api` deployment prefix, diff
  doc-quoted paths against `.paths` keys.)
- Custom-checker CI gate: REMOVED (see the DECISION section above). Prevention is
  tools, not bespoke checkers.
- TypeDoc regen-and-diff gate (NEXT tool, post republish): blocking gate that runs
  the ordered `build SDKs -> generate-typedoc.sh` then fails on a non-empty `git diff`
  of the `api/` tree; retires the stale-generated-docs class (D2/D3). MUST build the
  SDKs before typedoc (cross-package re-exports resolve from built `dist`, not source),
  else it reproduces the cross-package-stale state and passes while docs are wrong.
- Build-ordering fix (encode in a command, not memory). CRITICAL discovery: the mix
  packages (mix-tunnel/fetch/dns/websocket) are NOT in the static `pnpm-workspace.yaml`
  (they depend on the unpublished `@nymproject/smolmix-wasm`); they are injected only
  by `pnpm dev:on` (`dev-mode-add.mjs`), which also needs `wasm/smolmix/pkg/` to exist.
  So `build:ci:sdk` fails with `EFILTER No packages` unless `dev:on` ran first. The
  correct order is: build wasm -> `dev:on` -> `pnpm install` -> `build:types` ->
  `build:ci:sdk` -> `generate-typedoc.sh` -> `dev:off`. `ci-docs.yml` currently runs
  typedoc BEFORE any of this (and never `dev:on`s), so its downstream API docs are
  cross-package-stale every run. Fix: a `docs:typedoc` root script wrapping that full
  order (mirror `prebuild:ci`/`postbuild:ci` which already do dev:on/install/dev:off),
  and move the `ci-docs.yml` typedoc step to after the SDK build.
- version/dist-tag check; scan `.tsx`; more in-repo constants.

### Docs / wiki
- `documentation/README.md` "Validating docs against the code": cycle, generate-vs-
  check, tsify example.
- Wiki `~/dev/wiki/src/docs-for-ai/checking-vs-projecting.md` (new) + SUMMARY/MOC/
  rung-5 cross-links.

### References / inputs to fold into the framework
- Cloudflare "Engineering Standards Enforcement" (Codex): RFC-2119 MUST/SHOULD
  statements extracted to JSON with stable slug IDs; **approved -> enforced**
  lifecycle (advisory, then blocking); agent + linter + CLI verification paths.
  Directly informs (a) graded enforcement for our checks (warn before gate) and
  (b) stable-ID-per-claim = the contracts idea.
  https://blog.cloudflare.com/engineering-standards-enforcement/

## De-handrolling backlog (audit: replace hand-rolling with existing tools)
Opus audit of the retrieval/generator tooling. IMPORTANT correction to the audit:
`gray-matter` and `github-slugger` are transitive deps of nextra but NOT hoisted or
resolvable (`require.resolve` throws from documentation/docs), so adopting them needs
a direct `package.json` dep + `pnpm install` (fragile on this repo), not free.

Ranked:
1. `gray-matter` (frontmatter) + ONE shared strip helper across the three generators.
   DONE. New shared `lib/retrieval/mdx.mjs` (parseFrontmatter/pageTitle/
   pageDescription/stripMdx); all three generators wired to it; the three duplicated
   copies removed. Fixed both bugs: non-head-anchored frontmatter (gray-matter is
   head-anchored) and `import` lines stripped inside code fences (generate-llms-txt
   now uses the fence-aware strip: 35 code-example imports preserved in llms-full.txt,
   was 0). All three generators run clean under node (191 pages, 1406 chunks, 192
   page-markdown files); tests in `lib/retrieval/mdx.test.ts`. Note: `docs-index.json`
   chunk count moved 1403->1406 (frontmatter/anchor fixes), regenerated on build.
2. `github-slugger` for `chunker.mjs` slugify + dedup counter. REPLACE. Makes
   retrieval deep-link anchors exactly match Nextra's rendered anchors (they can drift
   today). S / Low, needs the dep add. Changes `docs-index.json` anchors: regen +
   spot-check deep links.
3. code-chunker doc-comment/attribute grouping. DONE (no dependency). Rust `///` docs
   and `#[...]` attributes now group with the item they describe instead of the
   previous chunk; node-verified + test added. Changes `code-index.json` boundaries
   (regenerated on the next build).
4. `remark`/`unified` MDX->markdown (library-grade JSX strip). CONSIDER / DEFER. M /
   Med, changes output broadly, leans on non-guaranteed transitive remark versions.
   Only if the line-based stripper proves too lossy after #1.

KEEP (justified hand-rolling, audit-confirmed): token estimate (`chars/4`; Voyage has
no JS tokeniser), content-hash cache (stdlib), Voyage batching/halving (bespoke to the
120k-token cap), cosine + linear scan (deliberately simple, zero infra), the markdown
chunker (carries anchor/heading metadata a generic splitter discards), the Nextra
`pages/` + `_meta.json` walk.

## Open decisions (see plan D1-D4)
- D1 embeddings provider (defaulted to Voyage dim 1024).
- D2 generation model + budget (defaulted to Haiku 4.5; abuse protection TODO).
- D3 MCP hosting (subdomain vs /docs/mcp) + rate limiting.
- D4 Confluence sanitise step + re-index cadence.

## Not started
- Phase 1 chat wired live (needs deps + keys).
- Confluence adapter.
- Rate limiting / abuse protection / feedback capture on the chat.

# Nym Docs v2

This is v2 of the nym docs, condensed from various mdbooks projects that we had previously.

These docs are hosted at [nym.com/docs](https://nym.com/docs).

## Doc projects
`docs/pages/` contains several subdirs, each hosting a subsection of the docs:
* `network` contains key concepts, cryptosystems, architecture.
* `developers` contains key concepts for developers, required architecture, and Rust/Typescript SDK docs.
* `operators` contains node setup and maintenance guides.

## Local development

### Dependencies
Our `prebuild` script relies on the following:
- `python`
- `pip`
- [`pandas`](https://pandas.pydata.org/)
- [`tabulate`](https://pypi.org/project/tabulate/)
- `jq`

Otherwise make sure to have `node` and `rust` installed.

### Link checking (optional)
We use [lychee](https://github.com/lycheeverse/lychee) to check for broken links. Install via your package manager or `cargo install lychee`, then run:
```sh
lychee documentation/docs/ --config lychee.toml --root-dir documentation/docs/pages/
```

### Serve Local (Hot Reload)
```sh
pnpm i
pnpm run dev
```

Open `http://localhost:3000`.

## Build
```sh
pnpm run build
```

## Contribution
* If you wish to add to the documentation please create a PR against this repo, with a `patch` against `develop`.

## Scripts
* `generate:commands`: generates command output files for clients and binaries. This script runs the `autodoc` rust binary, moves the files to their required places, and then if there is an update, commits them to git. We commit the files as our remote deployments pull from a git repo. **Only run this script on branches where you want to push e.g. the build info of a binary to production docs**; it will build the monorepo binaries and use their command output for the produced markdown files.
* `generate:tables`: generates various information tables containing some repo-wide variables and information about ISPs.

### Autodoc
`autodoc` is a script that generates markdown files containing commands and their output (both command and `--help` output). For the moment the binaries and their commands are manually configured in the script.

> **Only run this script on branches where you want to push e.g. the build info of a binary to production docs**; it will build the monorepo binaries and use their command output for the produced markdown files.

## CI/CD
- **Link checking**: Runs on every push to `documentation/docs/` via `.github/workflows/ci-docs-linkcheck.yml`

## SEO & Structured Data
### Frontmatter
Every `.mdx` page supports frontmatter fields that control meta tags, Open Graph, and JSON-LD schema:
```yaml
---
title: "Page Title for Search Engines"
description: "Unique meta description for this page."
schemaType: "TechArticle"    # TechArticle (default), HowTo, or FAQPage
section: "Operators"          # Operators, Developers, Network, APIs
lastUpdated: "2026-02-11"    # Feeds dateModified schema
breadcrumbLabel: "Custom Label" # Optional, overrides URL slug in breadcrumbs
---
```

### Sitemap
```bash
npx next-sitemap
```
Outputs `sitemap.xml` and `robots.txt` to `/public`.

### Environment Variable
Set in production:
```
NEXT_PUBLIC_SITE_URL=https://nym.com/docs
```

### Schema Types
| Type | Use When |
|------|----------|
| TechArticle | Reference docs, config guides, overviews (default) |
| HowTo | Step-by-step install/setup guides |
| FAQPage | Question-answer pages |

## AI assistant, MCP server & machine-readability
The docs are built to be consumed by AI agents and LLMs, not just read.

- **How it works**: `docs/pages/developers/mcp/architecture.mdx` (published at
  `/docs/developers/mcp/architecture`) covers the build-time retrieval pipeline,
  the chunker, the embedding cache, and how the MCP route and chat backend serve
  from a static index with no vector database. Start there before changing
  anything under `lib/retrieval/`, `lib/mcp/` or `scripts/next-scripts/generate-*`.
- **Tool reference**: `/docs/developers/mcp` for the tool catalogue and client setup.
- **Consumer-facing guide**: `/docs/use-with-ai`.

Working notes (`docs/ai-assistant-mcp-plan.md`, `docs/ai-assistant-mcp-scratch.md`)
are gitignored and local to the author's tree, so do not rely on them being present.

- **Ask AI**: an in-docs chat (right-hand sidebar) that answers from the
  documentation with citations, powered by retrieval plus Claude.
- **MCP server** at `/docs/api/mcp` (Streamable HTTP). Point a coding agent at it
  for docs search (`search_docs`, `get_section`), source-code search (`search_code`,
  over selected SDK / wasm / Sphinx / smolmix crates), live network tools
  (`network_summary`, `list_gateways`, `circulating_supply`, `chain_status`,
  `get_gateway`), and `validate_sdk_config`. Reference: `/docs/developers/mcp`.
- **Per-page Markdown**: append `.md` to any docs URL, or use the page's Copy
  button, to fetch it as clean Markdown.

### llms.txt
`llms.txt` and `llms-full.txt` are generated in the build, following
[Cloudflare's approach](https://developers.cloudflare.com/style-guide/how-we-docs/ai-consumability/).
Local: `http://localhost:3000/docs/llms.txt` and `.../llms-full.txt`. Production:
[https://nym.com/docs/llms.txt](https://nym.com/docs/llms.txt) and
[https://nym.com/docs/llms-full.txt](https://nym.com/docs/llms-full.txt).

### Retrieval & keys
Two build-time indexes, no vector database: a docs index (`voyage-3-large`) and a
code index (`voyage-code-3`), built during `pnpm run build` and gitignored.
Embeddings use Voyage; generation uses Anthropic Claude.

**Where each key must be set.** The two are needed at different stages, so they do
not go in the same place. Setting only one of them fails in a way that looks
unrelated to the key, so check both before debugging anything else.

| Key | GitHub Actions | Vercel project env | Why |
|-----|----------------|--------------------|-----|
| `VOYAGE_API_KEY` | **required** | **required** | The build embeds the corpus, and `/api/chat` and `/api/mcp` embed each incoming query at request time. Both stages call Voyage, so it is needed in both places. |
| `ANTHROPIC_API_KEY` | not needed | **required** | Generation happens only at runtime. Nothing in the build calls Claude. |

`ANTHROPIC_API_KEY` never appears in our source: the `@ai-sdk/anthropic` provider
reads it from the environment itself, so grepping for it finds nothing and it is
easy to forget when provisioning. Symptoms of each being missing:

- **No `VOYAGE_API_KEY` at build**: `generate-index.mjs` fails and the deploy stops.
- **No `VOYAGE_API_KEY` at runtime**: the build succeeds, then every chat and every
  MCP `search_docs` call fails when it tries to embed the query.
- **No `ANTHROPIC_API_KEY`**: retrieval works and the MCP server is fully
  functional, but the chat widget returns an error on every question. The MCP
  server does not need this key at all: it hands retrieved sections back to the
  calling agent and lets that agent's own model do the generating.

Keys live in GitHub Actions and Vercel secrets, never in the repo. `VOYAGE_API_KEY`
is already wired into `.github/workflows/cd-docs.yml`.

### Tunables
Everything below has a working default; none of it is required to run. Runtime
values can be changed in Vercel without a redeploy of the code.

| Variable | Default | Effect |
|----------|---------|--------|
| `CHAT_MODEL` | `claude-sonnet-5` | Which model answers in the widget. The tier matters here: the job is synthesising an answer from retrieved sections and citing them, and Haiku was visibly weaker at it. |
| `CHAT_MIN_SCORE` | `0.3` | Cosine-similarity floor for retrieval. **Starting value, not a measured one.** Below this a chunk is not treated as a source. Set too low, every question returns a full set of "sources" however unrelated, and the "not covered in the docs" answer can never fire; set too high, real questions lose their citations. Tune it by asking something deliberately off-topic and confirming it comes back with no sources. |

Compile-time constants, changed in source rather than the environment:

| Constant | Where | Default | Effect |
|----------|-------|---------|--------|
| `MAX_MESSAGES` | `pages/api/chat.ts` | `40` | Longest accepted conversation. A bound on what one caller can push through the paid calls. |
| `MAX_TOTAL_CHARS` | `pages/api/chat.ts` | `32_000` | Largest accepted request body, counted over text parts. |
| `topK` | `pages/api/chat.ts` (`buildContext` call) | `10` | How many chunks are retrieved per question, before `CHAT_MIN_SCORE` filters them. |
| `MAX_CHARS` | `lib/retrieval/chunker.mjs` | `2400` | Hard cap on a chunk. Changing it changes chunk boundaries, so it needs a full re-index. |
| `INPUT_MAX_HEIGHT` | `components/ChatWidget.tsx` | `140` | How far the chat textarea grows before it scrolls internally. |

<!-- prettier-ignore -->
> [!IMPORTANT]
> The query embedding and the index embedding must come from the same model.
> Changing the provider or model in `lib/retrieval/embed.mjs` requires a full
> re-index: querying a `voyage-3` index with `voyage-3-large` vectors returns
> meaningless neighbours rather than an error.

### Keeping docs honest against the code
The docs assert facts the source can settle: constant values, sizes, types, API
signatures, endpoint paths. The goal is that those never drift from the code. The
principle here is: **generate or project the fact from its single source, and
lean on existing tools, rather than shipping bespoke drift-checkers.** A checker keeps
two copies and diffs them; generation keeps one copy and makes drift impossible.

Where each fact class gets its guard:

| Fact class | Guard (existing tool) |
|------------|-----------------------|
| Wasm-boundary types (e.g. `TunnelState`) | **tsify** generates the TS type from the Rust serde in the wasm build |
| Plain shared Rust types | **ts-rs**, as `common/types` does (host `cargo test` export) |
| SDK API reference | **TypeDoc**, regenerated from the SDK source (a regen-and-diff CI gate keeps it fresh) |
| REST endpoints | the `apis/*.mdx` pages embed the **live OpenAPI spec** via Redoc |
| Links (incl. source links) | **lychee** (`ci-docs-linkcheck.yml`) |

`TunnelState` is the worked example of generation. Because it crosses the wasm
boundary, its TS type is generated by [tsify](https://github.com/madonoharu/tsify)
(the generator the crate already uses for `SetupOpts`): `state.rs` derives `Tsify`
under `#[cfg_attr(target_arch = "wasm32", ...)]` (so tsify stays out of any host
build) and `getTunnelState` returns the typed `TunnelState`, so the wasm build emits
the correct `.d.ts`:

```ts
type TunnelState =
  | { state: "connecting" } | { state: "ready" }
  | { state: "shutting_down" } | { state: "shutdown" }
  | { state: "failed"; reason: FailureReason };
```

That removes the drift at the source (it also corrected `reason`, which the SDK type
had as `string` where the runtime emits a `FailureReason` object). The SDK keeps its
copy of the type inline for now (to avoid a dependency on the unpublished
`smolmix-wasm`); a later step can bridge the generated `.d.ts` to the SDK directly so
even that copy is a projection.

For hand-written prose that states a fact (e.g. "a Sphinx packet is 2413 bytes"),
there is no generator, prose cannot be projected. We rely on careful authoring, which
a survey of the docs found holds up well: the drift concentrated in hand-typed
constants and stale generated docs, not in careful prose.

## Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:

* <p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://nym.com/docs">Nym Documentation</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://nym.com">Nym Technologies</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

* Nym applications and binaries are [GPL-3.0-only](https://www.gnu.org/licenses/)

* Used libraries and different components are [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.html) or [MIT](https://mit-license.org/)

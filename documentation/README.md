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
The docs are built to be consumed by AI agents and LLMs, not just read. Design
detail lives in `docs/ai-assistant-mcp-plan.md`; the build/test worklog in
`docs/ai-assistant-mcp-scratch.md`. The consumer-facing guide is `/docs/use-with-ai`.

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
Embeddings use Voyage; generation uses Anthropic Claude. Keys (`VOYAGE_API_KEY`
at build and runtime, `ANTHROPIC_API_KEY` for the chat at runtime) live in GitHub
Actions and Vercel secrets, never in the repo. See the worklog's key-handling notes.

### Validating docs against the code
We index the monorepo source alongside the prose (the `voyage-code-3` code index
above). That index exists for agent and chat search, but it also lets us turn
retrieval around: use the code as an oracle to catch prose that has drifted from it.
This is the strongest argument for indexing the code at all. The docs stay honest
because the code is ground truth; prose rots, constants do not.

The cycle:

```
  monorepo source ──index──▶ code index ──oracle──▶ docs claims ──diff──▶ drift report
        ▲                    (search_code,                                     │
        │                     the chat)                                        │
        └──────────────── fix the drifted page ◀───────────── review ◀─────────┘
```

A claim is any falsifiable statement the source can settle: a constant value, a size,
an API signature, a config field, an endpoint path, a CLI flag. When a page and the
source disagree, the source wins and the page is the bug.

**Run the size-drift check** (the first, deterministic slice of the cycle):

```sh
# from documentation/docs (the script lives in the sibling documentation/scripts)
node ../scripts/next-scripts/validate-docs-vs-code.mjs            # scan the docs
node ../scripts/next-scripts/validate-docs-vs-code.mjs --selftest # fixtures only
```

- `validate-docs-vs-code.mjs` walks `pages/**` and `lib/privacy-model`, extracts
  byte/KB size claims, binds each number to the noun it modifies ("2 KB payload",
  not nearest-keyword), and diffs against the oracle.
- The oracle is **derived from source**, not hand-typed: in-repo constants are read
  and evaluated (`REGULAR_PACKET_SIZE = 2*1024 + HEADER_SIZE + PAYLOAD_OVERHEAD_SIZE`),
  and the two leaves that live in the external `sphinx-packet` crate are pinned with
  a version check that throws if the crate is bumped in `Cargo.toml`. So the oracle
  cannot silently rot: change the in-repo maths and the derivation follows; bump the
  dependency and it fails loud asking you to re-verify.
- `--show-oracle` prints the derived constants and facts; `--selftest` runs the
  built-in fixtures (including the exact bug this was built to catch).
- Exit code is non-zero when a drift candidate is found, so it can gate CI later.

It already caught a real bug: the packet anatomy page said Sphinx packets are "2000
bytes" when `common/nymsphinx` defines `2*1024 + 348 + 17 = 2413` (a 2 KB payload).
A cross-check confirmed the mechanism: `network/cryptography/sphinx.md` independently
said "2048 bytes" and was right, so two pages disagreed and the code settled it.

Scope and next steps: the oracle is derived from source but still covers only Sphinx
sizes, and the scan does not read `.tsx`, so component-level strings are unguarded.
Widening beyond sizes (API signatures, config fields, endpoint paths, CLI flags,
version strings), and an LLM-judged pass for claims a regex cannot express, are the
next steps. Wire it into CI as a drift warning once coverage is broad enough. Covered
by `docs/lib/retrieval/validate-docs-vs-code.test.ts`.

## Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:

* <p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://nym.com/docs">Nym Documentation</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://nym.com">Nym Technologies</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

* Nym applications and binaries are [GPL-3.0-only](https://www.gnu.org/licenses/)

* Used libraries and different components are [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.html) or [MIT](https://mit-license.org/)

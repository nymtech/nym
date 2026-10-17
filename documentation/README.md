# Nym Docs v2

These docs are hosted at [nym.com/docs](https://nym.com/docs). The site is a Next.js 15 app with Nextra 4, in `documentation/docs/`.

## Layout

All paths are under `documentation/docs/`.

| Path | Content |
|------|---------|
| `content/` | The pages, one `.mdx` or `.md` file each: `network/` (key concepts, cryptosystems, architecture), `developers/` (SDKs, clients, tools), `operators/` (node setup and maintenance guides), `apis/` (API reference), and `use-with-ai.mdx`. The sidebar order is in the `_meta.js` file of each directory. |
| `.nym-codex/snippets/` | Snippets: the sections that pages import (see [Snippets and records](#snippets-and-records)). |
| `.nym-codex/manifest.json` | The sites that import snippets, with the page root and the components of each. |
| `components/` | React components, and the `.mdx` partials that pages import (operator partials are in `components/operators/snippets/`). `components/outputs/` holds generated files: do not edit them. |
| `data/csv/` | The CSV files that the prebuild turns into tables. |
| `app/` | The Next.js app router: the layout and the SEO metadata (`app/(docs)/seo.ts`). |
| `pages/api/mcp.ts` | The MCP server route. |
| `lib/privacy-model/` | The threat model as typed data: actors, vectors, properties, scenarios, and the worked examples. |
| `lib/` | Build and check code: `retrieval/` and `mcp/` (MCP server), `snippets/` (snippet tools), `archetypes/` (runbook lint and anchor check). |
| `mdx-components.jsx` | The components that every page and snippet can use with no import. |

## Local development

### Dependencies

You need `node` (22.18+ or 23.6+) and `pnpm`. `pnpm dev` runs the prebuild script (`documentation/scripts/next-scripts/python-prebuild.sh`) first. The prebuild reads the mainnet APIs, so it needs network access, and it needs:

- `python3`
- [`pandas`](https://pandas.pydata.org/)
- [`tabulate`](https://pypi.org/project/tabulate/)
- `jq`

Rust is only for `generate:commands` (see [Scripts](#scripts)). `pnpm dev` does not build Rust.

### Serve local (hot reload)

```sh
cd documentation/docs
pnpm i
pnpm dev
```

Open `http://localhost:3000/docs`. The site has the base path `/docs`, so `http://localhost:3000` shows a 404.

## Build

```sh
pnpm build
```

The build first builds the threat-model records (see [The threat-model step of the build](#the-threat-model-step-of-the-build)). Then it runs the generators (`llms-full.txt`, the search index, the code index, and the Markdown copy of each page), then `next build --turbopack`, then Pagefind and `next-sitemap`. `pnpm build:webpack` does the same with webpack.

## Snippets and records

The text of most pages is in snippets, not in the page files. A page file holds its title, its frontmatter, and a list of imports. The other sites (codex, and the MCP search index) read the same snippets, so you change a section in one file and each site that shows it gets the change.

### The structure

```text
documentation/docs/
├── content/                     pages: title, frontmatter, and snippet imports
├── .nym-codex/
│   ├── snippets/                one .mdx file for each section, one .mmd file for each diagram
│   ├── manifest.json            the sites that import snippets, and the components each allows
│   ├── edges.json               edges for records that have no frontmatter
│   ├── README.md                how to change, add, and split snippets
│   └── build/model.jsonl        threat-model records (written by the build, ignored by git)
├── lib/privacy-model/           the threat model as typed TypeScript data
└── lib/snippets/                the tools that build and check the records
```

A page imports its sections in order:

```mdx
import UseIt from '@snippets/nym/developers/anon-rpc/use-it.mdx';

<UseIt />
```

The `@snippets/nym/*` alias is in `tsconfig.json`. The path of a snippet is its id: `.nym-codex/snippets/developers/anon-rpc/use-it.mdx` is `nym/developers/anon-rpc/use-it`. A link from one snippet to another uses `snippet:<id>`, and each site writes its own route and anchor for it.

### Records

The tools turn the content into records: one JSON object for each unit, with an id, a kind, a body, and its edges to other records. There are two kinds of source:

| Kind | Source | What makes the records |
|------|--------|------------------------|
| Snippet | An `.mdx` file in `.nym-codex/snippets/` | `lib/snippets/records.mjs`, which also checks each page, import, and `snippet:` link |
| Model | The typed data in `lib/privacy-model/`: actors, vectors, properties, scenarios, variants, and the worked examples | `lib/snippets/threat-model.mjs`, which writes `.nym-codex/build/model.jsonl` |

A snippet does not import components or data. It cites a record, and the site renders it:

| In the snippet | On the page |
|----------------|-------------|
| `<Snippet id="nym/threat-model/scenario/mixnet" view="diagram" />` | The diagram of one scenario. The other views are `assessment` and `card`. |
| `<Snippet id="nym/threat-model/actor" view="reference" />` | The table of a whole kind: `actor`, `vector`, or `property` with `view="reference"`, or `scenario` with `view="matrix"` |
| `<Snippet id="nym/<path of an .mmd file>" />` | A Mermaid diagram |
| `<Version name="RUST_MSRV" />` | A version from `components/versions.ts` |
| `<Stat name="nodes" />` | A network count from the prebuild output |
| `<Demo name="EnsDemo" />` | An interactive demo, from the registry in `components/demos/demo.tsx` |

`components/snippet.tsx` reads `model.jsonl` at build time, so a page does not import the model modules, and the text of the tables is in the static HTML that Pagefind indexes. To change a table, a diagram, or a verdict, edit the data in `lib/privacy-model/`, not a page.

The procedures (change a section, add a section, split a page) are in [`docs/.nym-codex/README.md`](docs/.nym-codex/README.md). The design is in `documentation/NYM-CODEX-SNIPPETS-PLAN.md`.

### The threat-model step of the build

`predev`, `build`, and `build:webpack` run `node lib/snippets/threat-model.mjs --out .nym-codex/build/model.jsonl` first. The step builds the records and checks the data. It prints one line for each problem, then a count:

```text
warn   scenario vpn: 2 matrix verdicts are not in the per-actor assessment
90 model records: 6 actor, 56 claim, 6 invariant, 14 scenario, 2 property, 3 variant, 3 vector
0 integrity errors, 10 warnings
wrote .nym-codex/build/model.jsonl
```

An `error` line stops the build: the step writes no `model.jsonl` and exits with code 1. A `warn` line is a gap in the data, and the build continues.

A scenario states its verdicts in two places:

- `matrix`: one cell for each pair of property and actor, for example `p1L3G` (P1, request-identity unlinkability, against L3G, the global network observer). The comparison matrix on `choose-config` shows these cells.
- `actorAssessment`: one entry for each actor, with what that actor sees and cannot see, and the optional fields `p1` and `p2`. The `assessment` view of a scenario shows these entries, and the `card` view shows a P1 or P2 chip for each actor from `p1` and `p2`.

The step compares the two. When both give a verdict for the same pair, the verdicts must agree, or the step stops with an error. When the matrix gives a verdict and the assessment of that actor has no `p1` or `p2` field, the step counts a warning. So `scenario vpn: 2 matrix verdicts are not in the per-actor assessment` means: the `vpn` scenario in `lib/privacy-model/examples/generic.ts` has the matrix cells `p1L3L` and `p1L3G`, and its L3L and L3G assessments have no `p1` field. The matrix on `choose-config` shows the two verdicts, and the card of the scenario shows no P1 chip for L3L and L3G. To clear the warning, the threat-model owner adds `p1` to those two assessments, with the same value as the matrix cell.

| Message | Level | Meaning |
|---------|-------|---------|
| `N matrix verdicts are not in the per-actor assessment` | warn | N matrix cells have no verdict in the assessment of their actor |
| `no <actor> assessment, on the exception list: <reason>` | warn | The scenario has no assessment for a spine actor (L2, L3L, L3G), and `ASSESSMENT_EXCEPTIONS` in `threat-model.mjs` allows it, with the reason |
| `<property> against <actor> is "x" in the matrix and "y" in the assessment` | error | The two places give different verdicts |
| `no assessment for spine actor <actor>, and not on the exception list` | error | A generic scenario has no assessment for L2, L3L, or L3G |
| `matrix key <key> names no property and actor` | error | A matrix key is not of the form `p<n><actor>`, or it names an id that does not exist |
| `assessment of actor <a>, which does not exist` | error | An assessment names an unknown actor |
| `vector <id>: actor <a> does not exist` | error | A vector names an unknown actor |
| `scenario variant <id>: base <b> is not a generic scenario` | error | A variant in `examples/variants.ts` names an unknown base scenario |
| `<record>: edge to <id>, which has no record` | error | An edge points to a record that no source makes |

A line that starts with `example/<name>` is about a worked example (`examples/browsing.ts`, `messaging.ts`, `wallet.ts`). The worked examples have no spine rule, so the exception warning and the spine error do not apply to them.

The 10 warnings in the current data are all of these two warn kinds. Each one is a gap for the threat-model owner, not a fault in the build.

### Tools

Run from `documentation/docs`:

| Command | What it does |
|---------|--------------|
| `node lib/snippets/records.mjs` | Builds the snippet records and checks each page, import, and `snippet:` link. |
| `node lib/snippets/compile.mjs` | Compiles each snippet alone, so a snippet does not depend on the page around it. |
| `node lib/snippets/threat-model.mjs` | Builds the threat-model records and prints the check, as the build does. Add `--out <file>` to write them. |
| `node lib/snippets/split.mjs --site docs --dir <dir>` | Splits the pages of a directory into snippets. |
| `node lib/snippets/standalone.mjs --list` | Lists snippets that refer to text around them ("the table below"). |

## Runbook form

A page with `diataxis: how-to` or `diataxis: tutorial` in its frontmatter must follow the runbook (or tutorial) form. `pnpm lint:runbooks` checks it:

- Put each procedure in a `<Steps>` element, under a heading of its own or in a tab panel of its own.
- Write each step as a heading one level below its section (depth 3 under a `##`, depth 4 under a `###`), with no number in the text and an explicit anchor: `### Create a service file [#create-a-service-file]`. `Steps` adds the numbers.
- Give each code block a language from `lib/archetypes/runbook.json` (`sh`, not `shell`; `text` for output).
- Write placeholders with the names in `lib/archetypes/placeholders.json`, for example `<PUBLIC_IP>` and `<ID>`.

## Checks

Run from `documentation/docs`:

| Command | What it checks |
|---------|----------------|
| `pnpm test` | The Vitest unit tests. |
| `pnpm lint:runbooks` | The runbook form of each how-to and tutorial page. |
| `pnpm typecheck:tools` | The strict type check of the build and check tools that are in TypeScript (see [Tools in TypeScript](#tools-in-typescript)). |
| `node lib/archetypes/anchors.mts --list` | That each `#anchor` link names an id on its target page. Run it before and after you rename a heading: the count must not go up. |
| `lychee documentation/docs/ --config lychee.toml --root-dir documentation/docs/content/` | Broken links (from the repo root). Install [lychee](https://github.com/lycheeverse/lychee) with your package manager or `cargo install lychee`. |

## Tools in TypeScript

The build and check tools are moving from plain JavaScript (`.mjs`) to TypeScript (`.mts`). `lib/archetypes/` is in TypeScript now. `lib/snippets/` is next. `lib/retrieval/` and `scripts/next-scripts/` stay in JavaScript until the shared index replaces them, and the shared index is written in TypeScript.

| Item | Rule |
|------|------|
| File extension | `.mts`. The docs `package.json` has no `"type": "module"`, so `.mts` tells Node and `tsc` that the file is an ES module. |
| How a tool runs | `node lib/archetypes/lint.mts`. Node removes the types and runs the code, with no build step. This needs Node 22.18+ or 23.6+. CI uses Node 24. |
| Type check | `pnpm typecheck:tools` runs `tsc -p tsconfig.tools.json`: `strict`, `noUncheckedIndexedAccess`, and `noImplicitReturns`. CI runs it in `ci-docs-test`. The Next app keeps its own `tsconfig.json`. |
| Syntax | Only syntax that Node can remove: no `enum`, no `namespace`, and no parameter properties (`erasableSyntaxOnly`). Import a type with `import type`, and write the extension in each relative import (`./inline.mts`). |
| A JavaScript module that a tool imports | `tsc` reads its types from the JavaScript (`allowJs`), and does not check it. |

## Contribution

To add to the documentation, open a PR against `develop` in this repo.

## Scripts

- `generate:commands`: builds `nym-node`, `nym-api`, and `nymvisor`, and writes their command output (`--help` and build info) to `components/outputs/command-outputs/`. The `ci-docs-autodoc` workflow runs it and commits the files, because the deployments pull from git. Run it locally only to check the output; the script does not commit.
- `generate:tables`: the prebuild script. It writes the tables from `data/csv/` and the values it reads from the mainnet APIs to `components/outputs/`.
- `generate:llms`: writes `public/llms-full.txt`.

### Autodoc

`documentation/autodoc/` is the Rust crate that `generate:commands` uses to write each command and its output to Markdown. The binaries and their commands are set by hand in the crate.

## CI/CD

| Workflow | When it runs | What it does |
|----------|--------------|--------------|
| `ci-docs` | A push that changes `documentation/docs/` or the TypeScript SDK sources | Builds the site. |
| `cd-docs` | By hand | Builds and deploys the site. |
| `ci-docs-test` | A push that changes `documentation/docs/` | Runs `pnpm test` and `pnpm lint:runbooks`. |
| `ci-docs-linkcheck` | A push that changes `documentation/docs/` | Runs lychee. |
| `ci-codex-snippets` | A PR that changes the snippets, the pages, the snippet tools, or Rust code | Builds the snippet records, compiles each snippet, and comments on the PR with each cited code excerpt that changes. |
| `ci-docs-autodoc` | A change to `nym-node`, `nym-api`, `nymvisor`, or the autodoc crate, and release tags | Runs `generate:commands` and commits the result. |
| `ci-docs-typedoc-fresh` | A change to the TypeScript SDK sources | Fails when the committed TypeDoc reference differs from a fresh build. |

## SEO and structured data

### Frontmatter

Each `.mdx` page supports frontmatter fields that control the meta tags, Open Graph, and the JSON-LD schema:

```yaml
---
title: "Page Title for Search Engines"
description: "Unique meta description for this page."
schemaType: "TechArticle"    # TechArticle (default), HowTo, or FAQPage
section: "Operators"          # Operators, Developers, Network, APIs
lastUpdated: "2026-02-11"    # Feeds dateModified schema
breadcrumbLabel: "Custom Label" # Optional, overrides URL slug in breadcrumbs
diataxis: "how-to"            # tutorial, how-to, reference, or explanation; how-to and tutorial pages get the runbook lint
---
```

### Sitemap

`pnpm build` runs `next-sitemap` at the end. It writes `sitemap.xml` and `robots.txt` to `public/`.

### Environment variable

Set in production:

```
NEXT_PUBLIC_SITE_URL=https://nym.com/docs
```

### Schema types

| Type | Use when |
|------|----------|
| TechArticle | Reference docs, config guides, overviews (default) |
| HowTo | Step-by-step install and setup guides |
| FAQPage | Question-answer pages |

## MCP server and machine-readability

The docs are built for AI agents and LLMs as well as for people. There is an MCP server at `/docs/api/mcp` for coding agents, a Markdown copy of each page (add `.md` to any docs URL), and generated `llms.txt` and `llms-full.txt`. The **Use with AI** button on each page shows how to connect an agent to the server.

[MCP-SERVER.md](MCP-SERVER.md) documents all of it: the retrieval pipeline, chunking and the embedding cache, how the route serves from a static index with no vector database, where to set the API key, each tunable, and how to test a deployment. Read it before you change anything under `docs/lib/retrieval/`, `docs/lib/mcp/`, `docs/pages/api/`, or `scripts/next-scripts/generate-*`.

For readers rather than contributors: `/docs/developers/mcp` is the tool catalogue and client setup, and `/docs/use-with-ai` is the guide for users.

## Licensing and copyright information
This is a monorepo and components that make up Nym as a system are licensed individually, so for accurate information, please check individual files.

As a general approach, licensing is as follows this pattern:

* <p xmlns:cc="http://creativecommons.org/ns#" xmlns:dct="http://purl.org/dc/terms/"><a property="dct:title" rel="cc:attributionURL" href="https://nym.com/docs">Nym Documentation</a> by <a rel="cc:attributionURL dct:creator" property="cc:attributionName" href="https://nym.com">Nym Technologies</a> is licensed under <a href="http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1" target="_blank" rel="license noopener noreferrer" style="display:inline-block;">CC BY-NC-SA 4.0<img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/cc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/by.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/nc.svg?ref=chooser-v1"><img style="height:22px!important;margin-left:3px;vertical-align:text-bottom;" src="https://mirrors.creativecommons.org/presskit/icons/sa.svg?ref=chooser-v1"></a></p>

* Nym applications and binaries are [GPL-3.0-only](https://www.gnu.org/licenses/)

* Used libraries and different components are [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.html) or [MIT](https://mit-license.org/)

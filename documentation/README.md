# Nym Docs v2

These docs are hosted at [nym.com/docs](https://nym.com/docs). The site is a Next.js 15 app with Nextra 4, in `documentation/docs/`.

## Layout

All paths are under `documentation/docs/`.

| Path | Content |
|------|---------|
| `content/` | The pages, one `.mdx` or `.md` file each: `network/` (key concepts, cryptosystems, architecture), `developers/` (SDKs, clients, tools), `operators/` (node setup and maintenance guides), `apis/` (API reference), and `use-with-ai.mdx`. The sidebar order is in the `_meta.js` file of each directory. |
| `.nym-codex/snippets/` | Snippets: the sections that pages import (see [Snippets](#snippets)). |
| `.nym-codex/manifest.json` | The sites that import snippets, with the page root and the components of each. |
| `components/` | React components, and the `.mdx` partials that pages import (operator partials are in `components/operators/snippets/`). `components/outputs/` holds generated files: do not edit them. |
| `data/csv/` | The CSV files that the prebuild turns into tables. |
| `app/` | The Next.js app router: the layout and the SEO metadata (`app/(docs)/seo.ts`). |
| `pages/api/mcp.ts` | The MCP server route. |
| `lib/` | Build and check code: `retrieval/` and `mcp/` (MCP server), `snippets/` (snippet tools), `archetypes/` (runbook lint and anchor check). |
| `mdx-components.jsx` | The components that every page and snippet can use with no import. |

## Local development

### Dependencies

You need `node` and `pnpm`. `pnpm dev` runs the prebuild script (`documentation/scripts/next-scripts/python-prebuild.sh`) first. The prebuild reads the mainnet APIs, so it needs network access, and it needs:

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

The build runs the generators (`llms-full.txt`, the search index, the code index, and the Markdown copy of each page), then `next build --turbopack`, then Pagefind and `next-sitemap`. `pnpm build:webpack` does the same with webpack.

## Snippets

A page can import its sections from `.nym-codex/snippets/` instead of holding the text itself:

```mdx
import UseIt from '@snippets/nym/developers/anon-rpc/use-it.mdx';

<UseIt />
```

The `@snippets/nym/*` alias is in `tsconfig.json`. A snippet is one section of a page, with its heading. To change the text of a split page, edit the snippet, not the page. A link from one snippet to another uses `snippet:<id>`, and the site turns it into the page route and anchor.

The tools are in `lib/snippets/`. Run them from `documentation/docs`:

| Command | What it does |
|---------|--------------|
| `node lib/snippets/records.mjs` | Builds the snippet records and checks each page, import, and `snippet:` link. |
| `node lib/snippets/compile.mjs` | Compiles each snippet alone, so a snippet does not depend on the page around it. |
| `node lib/snippets/split.mjs --site docs --dir <dir>` | Splits the pages of a directory into snippets. |
| `node lib/snippets/standalone.mjs --list` | Lists snippets that refer to text around them ("the table below"). |

The design is in `documentation/NYM-CODEX-SNIPPETS-PLAN.md`.

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
| `node lib/archetypes/anchors.mjs --list` | That each `#anchor` link names an id on its target page. Run it before and after you rename a heading: the count must not go up. |
| `lychee documentation/docs/ --config lychee.toml --root-dir documentation/docs/content/` | Broken links (from the repo root). Install [lychee](https://github.com/lycheeverse/lychee) with your package manager or `cargo install lychee`. |

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

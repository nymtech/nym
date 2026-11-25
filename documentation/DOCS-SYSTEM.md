# The Nym docs system: snippets, records, checks and the MCP server

This file explains how the build turns the Nym docs into surfaces for people and for agents, and how CI keeps those surfaces true to their sources. It is for contributors who change the docs tooling.

The public page [How these docs are checked](docs/content/about/how-these-docs-are-checked.mdx) gives the reader view of the same system.

Other files have the details, and this file does not repeat them:

| File | What it has |
|---|---|
| [`README.md`](README.md) | The structure, the tools and the messages of the build |
| [`docs/.nym-codex/README.md`](docs/.nym-codex/README.md) | The procedures and the snippet rules |
| [`MCP-SERVER.md`](MCP-SERVER.md) | The index build, the embedding, the MCP tools and their checks |
| [`MCP-STORE.md`](MCP-STORE.md) | The public retrieval store that the MCP route reads |

## The problem

The docs have three kinds of reader. A person reads a page in a browser. A coding agent asks the MCP server and reads the sections that it returns. A second site, such as codex, shows the same content in a different structure.

Documentation fails in two ways, and each failure needs a different fix:

| Failure | Example | Fix |
|---|---|---|
| The content does not arrive | Definitions inside a React component. The search index reads the page source and does not see them, so agents answer from other pages | Put each fact in a form that each reader gets: a snippet, a record, a chunk |
| The content arrives and is false | A hand copy of a Rust struct stays as it was when the struct gets a new field | Generate the surface from its source, or record the link to the source and check it in CI |

Drift collects in constants that a person typed by hand, and in generated docs that nobody generated again (`MCP-SERVER.md`, "Keeping docs honest against the code"). So the system does not try to check prose. It removes hand copies, and it checks the links that remain.

## The rules behind the design

| Rule | What it means in practice |
|---|---|
| Generate before you check | A generated surface cannot drift from its source. A checker keeps two copies and compares them. Use a checker only where generation is not possible |
| Put each check in CI | A check that a person runs once finds drift once. A check in CI finds it on each pull request |
| A check must be able to fail | Before a check guards anything, show that it fails on a planted fault |
| Give each fact one home | A snippet, a model entity or a code item has one id. Each site reads it by id. Nobody copies it |
| Put the structure in the source, and keep the interface plain | The MCP server is a ranked search over sections. It has no knowledge graph. The structure is in the snippets, the records and the typed models |
| Record what a record commits the project to | Each record has an authority label. A requirement or a threat-model entity is `normative`. A description of the code is `descriptive` |

## The parts

```
 authored by people                          generated or checked by tools
 +---------------------------------+
 | .nym-codex/snippets/*.mdx, .mmd | --+--> records.mts -----> records (id, kind, body, edges)
 | content/*.mdx (imports only)    |   |                          |
 | lib/privacy-model/ (typed data) | --+--> threat-model.mts --> model.jsonl ---> <Snippet> views
 | openspec/specs/*/spec.md        | --+                          |
 | Rust source                     | --+--> extractor ----------> code excerpts, PR comment
 +---------------------------------+                              |
                                       coverage.mts: code -> requirement -> threat model
                                                                  |
 surfaces:  docs site pages   .md mirror   llms.txt   llms-full.txt   MCP index
```

### 1. Snippets and pages

A snippet is the smallest piece of content that has an id: one section of prose, one Mermaid diagram, one code item, or one entity of a typed model. A person writes a prose snippet once, in `documentation/docs/.nym-codex/snippets/`. A split page in `content/` holds its frontmatter, its title and a list of imports, and no prose of its own.

Some pages stay whole: the TypeDoc reference, the autodoc command output, and the pages that `sites.docs.whole` in `manifest.json` lists.

The path of a snippet is its id. To change an id, move the file with `git mv`. A link from one snippet to another names the target id (`snippet:nym/developers/rust/stream/intro`), and each site writes its own route. A link to a snippet that no page imports stops the docs build.

A snippet must read alone, because a search result or a second site shows it with no page around it. `standalone.mts --list` finds a pointer to text outside the snippet. The standalone lint is a warning.

A split must not change the text output. `text-outputs.mts` saves three outputs before and after a split (`llms-full.txt`, the `.md` mirror and each index chunk) and compares them.

`lib/snippets/benchmark.mts` compares the retrieval quality of two indexes on the labelled queries in `lib/snippets/benchmark-queries.json` (hit@1, hit@6, MRR@20). Use it before a change to the chunking.

### 2. Records and typed models

The tools turn each snippet into a record: one JSON object with an id, a kind, a body, a hash, and typed edges to other records. The record is the contract between a source and each consumer. It holds no URL and no page title, because those depend on the site.

The edges use a closed list of relations: `cites`, `relates-to`, `depends-on`, `realises`, `specialises`, `observes`, `mitigates`, `assessed-against`. A relation that is not on the list fails the check. With a free vocabulary, authors write three names for one meaning, and a query then misses two of them.

The threat model is a typed model. Its data is in `lib/privacy-model/`: actors, vectors, properties, scenarios and worked examples. `threat-model.mts` turns it into records in `.nym-codex/build/model.jsonl`, and `pnpm dev` and `pnpm build` run it first. An integrity error (a verdict in the matrix that contradicts the per-actor assessment, an edge to an unknown id) stops the build. A gap in the data is a warning.

A snippet cites a model entity and names a view: `<Snippet id="nym/threat-model/scenario/mixnet" view="diagram" />`. At build time, the site renders the diagram, the matrix or the assessment from the record. So the text of the tables is in the static HTML, and Pagefind reads it. The MCP index reads a text projection of the same data (`lib/retrieval/projections.mjs`).

Each record has an `authority`, which the tools set by kind: `normative` for an OpenSpec requirement and a threat-model entity, `descriptive` for specifications, explanations and code, `illustrative` for how-to and tutorial prose. The label tells an agent that a description of what the code does is not a rule about what it must do. No tool applies a precedence between the labels.

### 3. Code cited by symbol

A snippet can cite a Rust item by file and symbol: `<Snippet id="nym/common/network-defaults/src/network/v1.rs#NymNetworkDetails" />`. The extractor finds the item with tree-sitter. If the symbol is gone or matches two items, the check fails and names the snippet. So a rename in Rust fails CI in the same pull request.

The repository commits no extracted code. When a pull request changes a cited excerpt, `ci-codex-snippets` posts one comment with the excerpt diff and the pages that show it. The workflow runs only on its path filter, and it posts nothing when no cited excerpt changes. A Rust contributor runs no Node tool.

The docs site does not render a code citation. The Rust tutorials keep their example code in `sdk/rust/nym-sdk/examples/tutorial_*.rs`: `ci-build` builds those files, and `tutorial-examples.test.mts` fails when a tutorial and its example differ.

### 4. Runbook and tutorial form

A page with `diataxis: how-to` or `diataxis: tutorial` must follow an archetype. The archetype is data: the sections of the page, the Markdown syntax nodes of each section, and the constraints. `pnpm lint:runbooks` checks each such page and the partials that it imports. A page with no `diataxis` field gets no check.

| Rule | Why |
|---|---|
| Each step is a heading one level below its section, with no number and an explicit anchor | `Steps` adds the number. A number in the text gives two numbers on screen, and a link to a numbered anchor breaks when an author adds a step above it |
| Each code block has a language from a closed list (`sh`, not `shell`) | One language with three names splits search and highlighting |
| Each placeholder is a name from `lib/archetypes/placeholders.json` | One value with many names (the public IPv4 address of a node) confuses a reader who follows two pages |

The placeholder table on `operators/variables.mdx` renders from `placeholders.json`, so the site and the lint read one list. `lib/archetypes/anchors.mts` checks that each link with a `#anchor` names an id on its target page. The lint checks the form of a page. It does not check that a command works.

### 5. Machine-readable surfaces and the MCP server

The build writes three surfaces for agents: a Markdown copy of each page (add `.md` to the URL), `llms-full.txt`, and the search index. `public/llms.txt` is a hand-written list of pages.

The MCP server at `/docs/api/mcp` has nine tools: `search_docs` and `get_section` over the docs index, `search_code` over the code index, five live network tools (`network_summary`, `list_gateways`, `get_gateway`, `circulating_supply`, `chain_status`), and `validate_sdk_config`. The server returns sections, and the calling agent writes the answer. So the server needs no LLM key. It needs `VOYAGE_API_KEY`, and a search call costs one Voyage embedding.

The index is in a Postgres store. Producer CI writes it, and the route searches it with a cosine scan in SQL. The chunking, the embedding and the store come from the shared package `@nymproject/retrieval-indexer`, so other Nym repositories index with the same code and write their own shards. [`MCP-STORE.md`](MCP-STORE.md) explains the store.

The server applies no similarity floor, because no threshold separates on-topic from off-topic questions: an off-topic question can score higher than a correct on-topic one.

| Test | What it checks |
|---|---|
| `scripts/check-mcp-server.sh <deployment>` | HTTP checks against a deployment: transport, retrieval, a `get_section` round trip, code search, index coverage, the live network tools, and error shapes |
| `lib/retrieval/index-coverage.test.ts` | Each root in `ROOTS` has a coverage probe |
| [`agent-scenarios.md`](agent-scenarios.md) | A sandboxed agent with only the MCP server as a source answers set questions. A person reads the answer and the transcript. It tests retrieval and scope honesty, which a script cannot judge |

### 6. Requirements, the threat model, and the chain between them

A reader can go from a piece of code to the requirement that it keeps, and from the requirement to the threat-model property that it serves:

```
 code item  --realises-->  requirement  --realises / mitigates-->  property or vector (threat model)
```

Each OpenSpec requirement in `openspec/specs/` is a record (`lib/snippets/openspec.mts`). The links from a requirement to the threat model are authored edges in `.nym-codex/edges.json`, and the threat-model owner reviews each one. `coverage.mts` reports, for each requirement, the threat-model entities it links to and the code that realises it, so a reader sees where the chain is empty.

### 7. The CI checks

| Workflow or step | Runs on | Fails on |
|---|---|---|
| `build:data` (threat-model step, in `predev` and `build`) | Each build | An integrity error in the threat model |
| `ci-docs` | A push to the docs or the TypeScript SDK sources | A site that does not build. A snippet link with no target stops the build |
| `ci-docs-test` | A push to the docs | A unit test (the index coverage test with them), the strict type check of the tools, or a runbook lint error |
| `ci-codex-snippets` | A pull request that changes snippets, pages, the snippet tools, specs or any Rust file | A record error, a broken import or `snippet:` link, a code citation that does not resolve, or a snippet that does not compile alone. Posts the excerpt diff when a cited excerpt changes |
| `ci-docs-linkcheck` | A push to the docs | A broken link to a local page or file (lychee, `scheme = ["file"]`). It does not check external links |
| `ci-docs-typedoc-fresh` | A change to the TypeScript SDK sources | A committed TypeDoc reference that differs from a fresh build |
| `ci-docs-autodoc` | By hand only (`workflow_dispatch`) | It does not check the docs. It writes the `--help` output of each binary and commits it |
| `cd-retrieval-store` | A push to develop | An index build or a store write that fails. It writes the nym docs and code shards |

Some facts have a generator, so they need no checker:
- tsify generates the TypeScript types that cross the wasm boundary.
- ts-rs generates the plain shared Rust types.
- Redoc renders each REST reference page from the live OpenAPI spec.
- A snippet reads a version from `components/versions.ts` with `<Version>`.
- A snippet reads a network count with `<Stat>`, from the committed output of `python-prebuild.sh`.

`predev` runs `python-prebuild.sh`, which needs `pandas`, `tabulate` and `jq`. `pnpm build` does not run it and uses the committed outputs.

`records.mts` prints errors and warnings. An error fails `ci-codex-snippets`. A warning (a snippet over the 2,400-character chunk cap, a threat-model data gap, a missing inline source path) does not.

## Where the parts are

| Part | Path |
|---|---|
| Snippet tools | `documentation/docs/lib/snippets/` |
| Runbook lint and anchor check | `documentation/docs/lib/archetypes/` |
| Threat model data | `documentation/docs/lib/privacy-model/` |
| Retrieval and MCP | `documentation/docs/lib/retrieval/`, `documentation/docs/lib/mcp/`, `documentation/docs/pages/api/mcp.ts` |
| The shared indexer package | [`nymtech/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer) |
| Design of the snippets, the store and the coverage gate | `openspec/changes/` |
| Writing style | `documentation/docs/STE-STYLE.md` |

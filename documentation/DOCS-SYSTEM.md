# The Nym docs system: snippets, records, checks and the MCP server

This file explains how the build turns the Nym docs into surfaces for people and for agents. It gives the state of each part: merged, on a branch, or planned. It is for contributors who change the docs tooling, and for anyone who must decide what to build next.

The public page [How these docs are checked](docs/content/about/how-these-docs-are-checked.mdx) gives the reader view of the same system, with no task ids and no owners.

Three files have the details, and this file does not repeat them. [`README.md`](README.md) has the structure, the tools and the messages of the build. [`docs/.nym-codex/README.md`](docs/.nym-codex/README.md) has the procedures and the snippet rules. [`MCP-SERVER.md`](MCP-SERVER.md) has the retrieval pipeline.

Each part has one of three states:

| State | Meaning |
|---|---|
| develop | Merged. The figures come from `develop` at `ad62d6c7d5` (2026-11-17), unless a row gives a different source. |
| branch | On the named branch, not merged. |
| planned | In an OpenSpec change, with a task id, and not built. |

## The problem

The docs have three kinds of reader. A person reads a page in a browser. A coding agent asks the MCP server and reads the sections that it returns. A second site, such as codex, will show the same content in a different structure.

Documentation fails in two ways, and each failure needs a different fix:

| Failure | Example from these docs | Fix |
|---|---|---|
| The content does not arrive | The threat-model definitions were in a React component. The search index read the page source and did not see them, so agents answered from other pages. | Put each fact in a form that each reader gets: a snippet, a record, a chunk. |
| The content arrives and is false | A hand copy of a Rust struct in codex stays as it was when the struct gets a new field. | Generate the surface from its source, or record the link to the source and check it in CI. |

`MCP-SERVER.md` ("Keeping docs honest against the code") records where drift collects: in constants that a person typed by hand, and in generated docs that nobody generated again. So the system does not try to check prose. It removes hand copies, and it checks the links that remain.

## The rules behind the design

| Rule | What it means in practice |
|---|---|
| Generate before you check. | A generated surface cannot drift from its source. A checker keeps two copies and compares them. Use a checker only where generation is not possible. |
| Put each check in CI. | A check that a person runs once finds drift once. A check in CI finds it on each pull request. |
| A check must be able to fail. | Before a check guards anything, show that it fails on a planted fault. The Tracey gate plants three faults first, and fails if it does not find each one. |
| Give each fact one home. | A snippet, a model entity or a code item has one id. Each site reads it by id. Nobody copies it. |
| Put the structure in the source, and keep the interface plain. | The MCP server is a ranked search over sections. It has no knowledge graph. The structure is in the snippets, the records and the typed models. |
| Record what a record commits the project to. | Each record has an authority label. A requirement or a threat-model entity is `normative`. A description of the code is `descriptive`. |

## The parts

```
 authored by people                          generated or checked by tools
 +---------------------------------+
 | .nym-codex/snippets/*.mdx, .mmd | --+--> records.mts -----> records (id, kind, body, edges)
 | content/*.mdx (imports only)    |   |                          |
 | lib/privacy-model/ (typed data) | --+--> threat-model.mts --> model.jsonl ---> <Snippet> views
 | openspec/specs/*/spec.md        | --+                          |
 | Rust source                     | --+--> extractor ----------> code excerpts, PR comment
 +---------------------------------+   |                          |
                                       +--> Tracey (branch) ----> code and test links per requirement
                                                                  |
                                       coverage.mts: code -> requirement -> threat model
                                                                  |
 surfaces:  docs site pages   .md mirror   llms.txt   llms-full.txt   MCP index   codex (planned push)
```

### 1. Snippets and pages (develop)

A snippet is the smallest piece of content that has an id: one section of prose, one Mermaid diagram, one code item, or one entity of a typed model. A person writes a prose snippet once, in `documentation/docs/.nym-codex/snippets/`. A split page in `content/` holds its frontmatter, its title and a list of imports, and no prose of its own.

| Fact | Value |
|---|---|
| Prose snippets | 1,171 `.mdx` files |
| Diagram snippets | 11 `.mmd` files |
| Docs pages that import snippets | 194 of 270 |
| Pages that stay whole | The TypeDoc reference, the autodoc command output, and the 14 pages in `sites.docs.whole` of `manifest.json`: the changelog (until the end of 2026) and 13 operator pages that wait for a manual restructure |
| Snippets with no standalone lint fault | 1,171 of 1,171 (`standalone.mts`). Four pointers in the Swiss legal opinion are exceptions, because they are the counsel's words. The lint is a warning, and the read by a person (tasks 0.3, 4b.2 to 4b.4) is not done |

The path of a snippet is its id. To change an id, move the file with `git mv`. A link from one snippet to another names the target id (`snippet:nym/developers/rust/stream/intro`), and each site writes its own route. A link to a snippet that no page imports stops the docs build.

A snippet must read alone, because a search result or a second site shows it with no page around it. `standalone.mts --list` finds a pointer to text outside the snippet.

The split did not change the text output, with two small exceptions. For each batch, `text-outputs.mts` compared three outputs before and after the split: `llms-full.txt`, the `.md` mirror and each index chunk. They were identical apart from the network counts and the link targets of the shared performance sections (tasks 10.7, 10.10).

One question for the split was retrieval quality: is one vector per snippet worse than the old capped chunks? Task 0.2 records a first baseline on 24 labelled queries (`lib/snippets/benchmark-queries.json`) for the capped chunks: hit@1 79%, hit@6 96%, MRR@20 0.873. `lib/snippets/benchmark.mts` compares the two indexes. The first comparison run (2026-10-16) showed no difference beyond the noise of 24 queries. The repository holds the tool and the queries, not the result of that run.

### 2. Records and typed models (develop)

The tools turn each snippet into a record: one JSON object with an id, a kind, a body, a hash, and typed edges to other records. The record is the contract between a source and each consumer. It holds no URL and no page title, because those depend on the site.

The edges use a closed list of relations: `cites`, `relates-to`, `depends-on`, `realises`, `specialises`, `observes`, `mitigates`, `assessed-against`. A relation that is not on the list fails the check. With a free vocabulary, authors write three names for one meaning, and a query then misses two of them.

The threat model is the first typed model. Its data is in `lib/privacy-model/`: actors, vectors, properties, scenarios and worked examples. `threat-model.mts` turns it into records in `.nym-codex/build/model.jsonl`, and `pnpm dev` and `pnpm build` run it first. An integrity error (a verdict in the matrix that contradicts the per-actor assessment, an edge to an unknown id) stops the build. A gap in the data is a warning.

A snippet cites a model entity and names a view: `<Snippet id="nym/threat-model/scenario/mixnet" view="diagram" />`. At build time, the site renders the diagram, the matrix or the assessment from the record. So the text of the tables is in the static HTML, and Pagefind reads it. The MCP index reads a text projection of the same data (`lib/retrieval/projections.mjs`).

| Model | State |
|---|---|
| Threat model | develop. The README example run gives 90 model records and 0 integrity errors. |
| The four deep-dive animations (`PacketAnatomy` first) as typed data | planned (`add-codex-snippets` 6d.1 to 6d.3) |
| A motivation model: stakeholders, drivers, goals, principles | planned (6f.1 to 6f.4). It has no entity types and no owner yet. |

Each record has an `authority`, which the tools set by kind (`specs/codex-snippets/spec.md` of `add-codex-snippets`): `normative` for an OpenSpec requirement and a threat-model entity, `descriptive` for specifications, explanations and code, `illustrative` for how-to and tutorial prose. The label tells an agent that a description of what the code does today is not a rule about what it must do. No tool applies a precedence between the labels yet.

### 3. Code cited by symbol (develop: extraction and the check; planned: rendering)

A snippet can cite a Rust item by file and symbol: `<Snippet id="nym/common/network-defaults/src/network/v1.rs#NymNetworkDetails" />`. The extractor finds the item with tree-sitter. If the symbol is gone or matches two items, the check fails and names the snippet. So a rename in Rust fails CI in the same pull request.

The repository commits no extracted code. When a pull request changes a cited excerpt, `ci-codex-snippets` posts one comment with the excerpt diff and the pages that show it. The workflow runs only on its path filter, and it posts nothing when no cited excerpt changes. A Rust contributor runs no Node tool.

No site renders a code citation yet (tasks 2.6, 10.6). On `develop`, three citations exist, all on the codex pilot page `network-details-model` in `.nym-codex/pages/`, which nothing publishes yet. The Rust tutorials are the next users. Their example files (`sdk/rust/nym-sdk/examples/tutorial_*.rs`) exist, `ci-build` builds them, and `tutorial-examples.test.mts` fails when a tutorial and its example differ. The open part of task 9.15 is the citation by symbol, after the site renders citations.

### 4. Runbook and tutorial form (develop)

A page with `diataxis: how-to` or `diataxis: tutorial` must follow an archetype. The archetype is data: the sections of the page, the Markdown syntax nodes of each section, and the constraints. `pnpm lint:runbooks` checks each such page and the partials that it imports. A page with no `diataxis` field gets no check.

| Rule | Why |
|---|---|
| Each step is a heading one level below its section, with no number and an explicit anchor | `Steps` adds the number. A number in the text gave two numbers on screen, and a link to a numbered anchor broke when an author added a step above it. |
| Each code block has a language from a closed list (`sh`, not `shell`) | Shell had three names on the operator pages. |
| Each placeholder is a name from `lib/archetypes/placeholders.json` | The public IPv4 address of a node had at least six names. |

Task 9.12 moved 292 step headings to the new form. Today all 33 how-to and tutorial pages pass every rule. The lint checks the form of a page. It does not check that a command works.

### 5. Machine-readable surfaces and the MCP server (develop)

The build writes three surfaces for agents: a Markdown copy of each page (add `.md` to the URL), `llms-full.txt`, and the search index that the MCP route at `/docs/api/mcp` serves. `public/llms.txt` is a hand-written list of pages.

The MCP server has nine tools: `search_docs` and `get_section` over the docs index, `search_code` over the code index, five live network tools (`network_summary`, `list_gateways`, `get_gateway`, `circulating_supply`, `chain_status`), and `validate_sdk_config`. The server returns sections, and the calling agent writes the answer. So the server needs no LLM key. It needs `VOYAGE_API_KEY`, and a search call costs one Voyage embedding.

The index is two static JSON files that the build writes and that the `/api/mcp` function loads at cold start. A search is a cosine scan in memory. The server applies no similarity floor. A measurement showed why: "What is the capital of France?" scored 0.523, higher than a correct on-topic question at 0.504, so no threshold separates on-topic from off-topic questions.

Two tests check the server:

| Test | What it checks |
|---|---|
| `scripts/check-mcp-server.sh <deployment>` | 37 HTTP checks against a deployment: transport, retrieval, a `get_section` round trip, code search, index coverage, the live network tools, and error shapes |
| [`agent-scenarios.md`](agent-scenarios.md) | A sandboxed agent with only the MCP server as a source answers set questions. A person reads the answer and the transcript. It tests retrieval and scope honesty, which a script cannot judge |

The work on branches and in plans:

| Part | State | What it changes |
|---|---|---|
| A shared indexer package, `@nymproject/retrieval-indexer` | branch `max/shared-mcp`. The package is public at v0.1.1, and the branch imports it | The chunkers, the tree-sitter item finder, the embedder and the store move out of `lib/retrieval/`, so other repositories (`nym-vpn-client` first) can index with the same code |
| A Postgres and pgvector store for the public index | branch `max/shared-mcp` (`max/shared-mcp:documentation/MCP-STORE.md`). The cutover is planned: tasks 5.0 to 5.3 | The index leaves the function bundle. `MCP-SERVER.md` measures it at about 114 MB, and `MCP-STORE.md` estimates about 120 MB at the chunk counts of 2026-11-13. Each producer repository writes its own shard from its own CI |
| A private store, separate from the public one | planned | Private content goes only to a self-hosted store with a self-hosted embedding model. No query through the public server can return it, because the data is not in the public store |
| A coverage gate for code index roots | branch `max/shared-mcp` (`add-code-index-coverage-gate`, 5 of 8 tasks) | A test fails when a root in `ROOTS` has no probe. 8 of the 20 roots had none |
| The indexer reads snippet records | planned (`add-codex-snippets` 6.0 to 6.3) | The indexer makes no cut of its own. A snippet is a chunk |
| Context prefix, edge expansion, `get_related` | planned (6.5 to 6.7) | Each one stays off unless a labelled measure shows a gain. Only the plain vector baseline of task 0.2 exists |

### 6. Requirements, the threat model, and the chain between them

The design wants a reader to go from a piece of code to the requirement that it keeps, and from the requirement to the threat-model property that it serves:

```
 code item  --realises-->  requirement  --realises / mitigates-->  property or vector (threat model)
```

On `develop`, the tools exist and the chain is empty. Each of the 492 OpenSpec requirements is a record. `.nym-codex/edges.json` is empty, and `coverage.mts` counts 0 complete chains.

The main finding of the snippets work was this gap: no OpenSpec spec defined the three mechanisms that give the mixnet its privacy properties. These are the Sphinx packet format, client cover traffic, and the delay that a mix node adds to each packet. So for the code that matters most, no requirement existed to link to.

The branch `max/mixnet-core-specs` closes it:

| Part | State on the branch |
|---|---|
| `add-mixnet-core-specs` | 21 requirements: `sphinx-packet-format` (7), `client-cover-traffic` (8), `mix-node-delay` (6). Each describes the code as it is, with the conditions under which a protection is absent (cover off by a setting, zero hop delays, no mix hops). The design note lists what the specs do not promise |
| Proposed threat-model links | 13 requirements have a proposed `realises` or `mitigates` link, and 8 state that they serve none. The threat-model owner must review each link before it goes into `edges.json`. A trial run on 2026-11-04, with the specs and hand-written code edges, gave 12 complete chains from 0 |
| Tracey ids and markers | Each requirement has an id line such as `r[sphinx.size-classes]`. The Rust code has 59 `// r[impl ...]` lines and 78 `// r[verify ...]` lines |
| Version suffixes | When the meaning of a requirement changes, the author adds a version suffix to its id (`r[sphinx.size-classes+2]`). The gate then fails on each marker that names the old version, and `tracey query stale` lists them. A change of wording with the same meaning needs no suffix |
| `ci-spec-trace` | Installs Tracey 1.4.0, checks its SHA-256, plants three faults, and fails if it does not find each one. Then it fails a pull request on an unknown id, a stale version, a duplicate id, or a requirement with no code or no test. The branch log records it green at `cdd8b3c166`, with all 21 requirements covered by code and by a test |
| A lefthook pre-commit job | Runs the same gate. It prints the result and does not fail the commit |
| `coverage.mts` | Asks Tracey for the code links, and reads the threat-model links from `edges.json`. The hand-written code edges are gone, so the code link has one source |

[Tracey](https://github.com/bearcove/tracey) reads only ids and comments. Code can change its behaviour under a valid marker. Only a `verify` test that fails when the behaviour changes guards the behaviour. Tracey shows that the test exists and names the correct requirement. `max/mixnet-core-specs:documentation/TRACEY.md` has the full explanation.

The work on the specs found five places where the code and its comments disagree, or where a setting has no effect. It also found one docs claim that needs a condition. `max/mixnet-core-specs:openspec/changes/add-mixnet-core-specs/design.md` records them:

1. The replay filter treats a tag as not seen when the node has a key but no filter for its rotation. The comment in `batch_check_and_set` says the opposite.
2. No packet builder reads `use_legacy_sphinx_format`.
3. No client or SDK code reads `surb_mix_hops`.
4. A comment by `DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY` gives a mean delay of 50 ms. The client default is 15 ms.
5. Hop delays come from the thread random generator, so `deterministic_route_selection` fixes the route and not the delays.
6. The page "What is a Nym client" says the client uses the same bandwidth idle as under load. The `client-cover-traffic` spec shows that named settings turn one or both cover streams off. The sentence needs that condition when the specs merge.

What remains in `add-spec-trace-gate`:
- A required check on `develop` (3.4, which needs a repository admin).
- Tracey ids and markers for the SDK stream spec (section 4, 12 requirements).
- Tracey ids and markers for the three Network Monitor V3 specs (section 5, 49 requirements).
- The archive of the specs (section 7).

The config reads 3 specs today.

### 7. The CI checks

| Workflow or step | Runs on | Fails on | State |
|---|---|---|---|
| `build:data` (threat-model step, in `predev` and `build`) | Each build | An integrity error in the threat model | develop |
| `ci-docs` | A push to the docs or the TypeScript SDK sources | A site that does not build. A snippet link with no target stops the build | develop |
| `ci-docs-test` | A push to the docs | A unit test, the strict type check of the tools, or a runbook lint error | develop |
| `ci-codex-snippets` | A pull request that changes snippets, pages, the snippet tools, specs or any Rust file | A record error, a broken import or `snippet:` link, a code citation that does not resolve, or a snippet that does not compile alone. Posts the excerpt diff when a cited excerpt changes | develop |
| `ci-docs-linkcheck` | A push to the docs | A broken link to a local page or file (lychee, `scheme = ["file"]`). It does not check external links | develop |
| `ci-docs-typedoc-fresh` | A change to the TypeScript SDK sources | A committed TypeDoc reference that differs from a fresh build | develop |
| `ci-docs-autodoc` | By hand only (`workflow_dispatch`) | It does not check the docs. It writes the `--help` output of each binary and commits it | develop |
| `ci-spec-trace` | Each pull request | A broken marker, or a traced requirement with no code or no test | branch `max/mixnet-core-specs` |
| Code index coverage test | A change to `ROOTS` or the probes | A root with no probe | branch `max/shared-mcp` |

Some facts have a generator, so they need no checker:
- tsify generates the TypeScript types that cross the wasm boundary.
- ts-rs generates the plain shared Rust types.
- Redoc renders each REST reference page from the live OpenAPI spec.
- A snippet reads a version from `components/versions.ts` with `<Version>`.
- A snippet reads a network count with `<Stat>`, from the committed output of `python-prebuild.sh`.

The placeholder table on `operators/variables.mdx` renders from `placeholders.json`, so the site and the runbook lint read one list (task 9.14). `predev` still runs `python-prebuild.sh`, which needs `pandas`, `tabulate` and `jq`. `pnpm build` does not run it and uses the committed outputs. Tasks 12.1 to 12.4 of `add-codex-snippets` plan to remove the Python. The script still writes one CSV table, the network numbers and two `--help` captures.

`records.mts` gives 0 errors and 48 warnings: 38 snippets over the 2,400-character chunk cap, and 10 threat-model data gaps. A path check that reads the local build state can add one more. The next step is to fix each warning and then make warnings fail CI, with no exemption list.

## What snippets make possible

These parts are planned, unless a row says otherwise. Each one depends on the records, the ids and the typed edges above.

| Idea | What it does | Where it is planned |
|---|---|---|
| A push to codex | CI on `develop` pushes the snippets, the pages and the records to codex. codex stops being a hand copy, and deletes its older copy of the threat model | `add-codex-snippets` 4.2 to 4.4 (4.1, the PR check, is built) |
| More sites from one pool | A site has its own pages and imports the same snippet files. The docs site already does this (task 10.1). `nym-vpn-client` is the next candidate | `add-shared-retrieval-index` (the NymVPN docs site, a later stage) |
| Statements | Each OpenSpec requirement and each matrix cell is a statement with a `level` (MUST, SHOULD) and a `status`. The fields exist (7.1b), and no statement is enforced. An `approved` statement is reported. An `enforced` one blocks a merge | 7.1c: who promotes a statement, and the rule |
| Viewpoints and views | A viewpoint file names a reader (an auditor, an operator, an SDK developer), the questions that the reader asks, the records to select and the edges to follow. A generator makes the pages. The first test is to make one hand-made audit page from a viewpoint and compare the two | Task section 8 (design D16) |
| Generated prose with provenance | Where a view needs connecting text, the prompt is in git, the output is a record with the input ids and their hashes, and the text changes only when an input changes | Task section 8 (design D16) |
| A warrant on each record | `checked`, `entailed`, `asserted` or `narrative`. `authority` says what a record commits the project to. `warrant` says what backs it | `add-codex-assurance` A1 (branch `max/mixnet-core-specs`) |
| A witness and a canary before `enforced` | A statement becomes `enforced` only when a mutation of its source turns its witness test red | A2 |
| Obligations with an owner | An accepted contradiction, or a gap on a `normative` record, becomes a record with an owner and a close condition. An obligation with no owner fails the check | A3 |
| An assurance view | One page with each guarantee, its assumptions, its open gaps, and what is enforced against what is only approved | A4 |
| A sufficiency report | For each question in a viewpoint, whether the view has a record that answers it. Fidelity to the source does not show that a page answers its reader | A5 |
| Verdicts from rules | The threat-model verdicts come from rules over the capabilities of each configuration, and a CI test fails when a computed verdict and a typed verdict differ. The rules exist on a local branch that is not pushed | A6 |
| A coherence agent | It reads a new proposal and judges it against the statements, and follows edges to the reason behind each one. `add-codex-assurance` defines what happens to its verdicts. The agent itself has no design in the repository yet | A3, and the proposal of `add-codex-assurance` |

`add-codex-snippets` (design, "The measures give two points where the design changes") states two results that change its course:

- A retrieval mechanism shows no gain in stage 6. Then it stays off.
- The standalone share stays far below 90% after the rewrite. Then the page becomes the unit of authoring again, and the tools derive the records from headings.

The record format stays the same in both cases, so no consumer changes.

## Decisions that wait for an owner

| Task | Decision |
|---|---|
| 0.0, 6e.1, 9.0, 10.0 | Owners of the retrieval baseline, the requirement links, the archetypes and the docs site split |
| 6b.8 | CODEOWNERS for `documentation/docs/lib/privacy-model/` |
| Threat-model owner | Review of the 13 proposed links on `max/mixnet-core-specs` |
| Code owners of `common/nymsphinx`, `common/client-core`, `nym-node` | Review of the three specs, the markers in their code, and the five findings |
| 7.1c | Who promotes a statement to `enforced`, and the rule |
| 8.1, 8.2 | The facet vocabulary, and the first viewpoint |
| 6f.1 | The entity types and the owner of the motivation model |
| 4.4 | The write credential for the push to codex |

## Where the parts are

| Part | Path or branch |
|---|---|
| Snippet tools | `documentation/docs/lib/snippets/` |
| Runbook lint and anchor check | `documentation/docs/lib/archetypes/` |
| Threat model data | `documentation/docs/lib/privacy-model/` |
| Retrieval and MCP | `documentation/docs/lib/retrieval/`, `documentation/docs/lib/mcp/`, `documentation/docs/pages/api/mcp.ts` |
| Snippets design and tasks | `openspec/changes/add-codex-snippets/` |
| Mixnet core specs, Tracey gate, assurance | `openspec/changes/add-mixnet-core-specs/`, `add-spec-trace-gate/`, `add-codex-assurance/` on `max/mixnet-core-specs` |
| Shared index and store | `openspec/changes/add-shared-retrieval-index/`, `add-code-index-coverage-gate/` on `max/shared-mcp` |
| Writing style | `documentation/docs/STE-STYLE.md` |

# Nym codex snippets: plan

Status: draft for review, 2026-10-13. Owner: TBD(Max). Reviewers: TBD(Max). The implementation spec is the OpenSpec change `openspec/changes/add-codex-snippets/`. The reasons for the plan, the readers it serves, and the findings of an independent review are in `NYM-CODEX-SNIPPETS-CONTEXT.md`.

## Purpose

Today the same content exists as hand-made copies. The codex repository holds 24 Rust excerpts that a person pasted from this monorepo. It also holds 474 specification pages that a person copied in by hand. The docs site, the codex site, and the retrieval index each cut that content into pieces with their own code.

This plan makes one unit, the snippet, and one path for it. People write snippets in a `.nym-codex/` directory in each source repository. A CI job pushes them to codex. A page is a list of snippet ids, so a site is a set of lists.

The plan has four goals:

1. Remove the hand copies. A snippet cites code by symbol, and a tool does the copy.
2. Give the retrieval index one input format for this content. A snippet is a retrieval chunk, so the indexer makes no cut of its own for content that is in `.nym-codex/`. Content that is not there yet (the docs site pages, and the broad code index) keeps the present chunkers.
3. Build a documentation site from a selection of snippets, with no copy of the prose.
4. Make the content a graph. Each snippet says which question it answers (why, what, how). Typed edges connect a piece of code to the requirement it carries out and to the threat that motivates it.

## Terms

| Term | Meaning |
|---|---|
| Snippet | The smallest unit of content that has an id. It has one of four kinds: prose, code, diagram, model. |
| Prose snippet | One section of text in its own MDX file. A person writes it. |
| Diagram snippet | One Mermaid diagram in its own `.mmd` file. A person writes it. |
| Model snippet | One entity of a typed data model, for example one threat-model actor or one goal in the motivation model. A person writes the data, and a tool writes the text form. |
| Code snippet | One named item from a source file: a function, method, struct, enum, trait, or module. A tool extracts it. |
| Recipe | A JSON file that defines one page: a title and an ordered list of snippet ids. |
| Record | The typed data for one snippet: its body and its metadata. |
| `.nym-codex/` | The directory in a source repository that holds its snippets and recipes. |
| codex | The public aggregation repository. It receives pushes from public source repositories. |
| Private target | A private aggregation repository. It holds all of codex plus the content of private repositories. |

## Flow

```
SOURCE REPOSITORY (nym)                     AGGREGATION                 CONSUMERS

 authored by people
 +------------------------------+
 | .nym-codex/snippets/  .mdx   |  prose
 | .nym-codex/snippets/  .mmd   |  diagrams
 | .nym-codex/pages/     .json  |  recipes
 | source code  (.rs, .ts, ...) |
 +---------------+--------------+
                 |
                 | 1. extract (local run, and CI on each PR)
                 |    resolve each code citation by symbol
                 v
 +------------------------------+
 | PR comment: each changed     |  nothing generated is committed;
 | excerpt and the pages it     |  the reviewer sees the excerpt
 | changes                      |  change in the source PR
 +---------------+--------------+
                 |
                 | 2. push (CI, on merge to develop)
                 |    replace the repository's two directories
                 |    in codex, record the source commit once
                 v
                         +--------------------------+
                         | codex (public)           |      +--> codex site: materialise recipes to pages
                         |  snippets/nym/*.jsonl    | -----+--> another site: its own recipes, same records
                         |  pages/nym/**.json       |      +--> public index (Voyage)
                         +------------+-------------+
                                      |
                                      | 3. sync (one direction)
                                      v
 private repository      +--------------------------+
 .nym-codex/  --push---> | private target           | --------> private index (self-hosted model)
                         |  codex + private content |
                         +--------------------------+
```

The flow has three rules:

1. A source repository owns `snippets/<repo>/` and `pages/<repo>/` in the target and no other path. The push job replaces both in full, and git computes the diff. A renamed or deleted snippet disappears from codex with no extra logic.
2. A private repository pushes to the private target and holds no credential for codex. Content moves from codex to the private target and does not move back.
3. Do not edit pushed content in codex by hand. To change it, change the source and let CI push.

## Form

### The directory in a source repository

```
.nym-codex/
  manifest.json            authored: the repository name and its GitHub URL
  edges.json               authored: edges from records that have no frontmatter (code, adapters)
  snippets/**/<name>.mdx   authored: one prose snippet in each file
  snippets/**/<name>.mmd   authored: one diagram snippet in each file
  pages/**/<page>.json     authored: one recipe for each page
  pages/**/_meta.json      authored: sidebar order as data; the site build writes _meta.js from it
```

### A prose snippet

This is the first section of the codex page `technical-details/discovery-urls/network-details-model`, as a snippet. The text is the page text today. The citation takes the place of the Rust fence that the page holds now.

File: `.nym-codex/snippets/specifications/nym-network/technical-details/discovery-urls/network-details-model/nymnetworkdetails-v1.mdx`

```mdx
---
title: "`NymNetworkDetails` (v1)"
aspect: design
---

`NymNetworkDetails` (`common/network-defaults/src/network.rs`) is the value every binary in the workspace builds at startup:

<Snippet id="nym/common/network-defaults/src/network.rs#NymNetworkDetails" />
```

The file is MDX and not plain Markdown, because a body can hold a `<Snippet>` citation. The title uses YAML quotes, because it holds inline code.

Rules for the file:

| Rule | Reason |
|---|---|
| The body has no heading. The `title` is the heading, and the recipe sets its level. | The same snippet can sit at level 2 on one page and level 3 on a second one. |
| The body is clear alone: no "as described above". A link names its target by id (see Links). | A recipe can put it on a page next to any other snippet, and the index returns it with no page around it. |
| The body can cite code and diagram snippets. It cannot cite a prose snippet. | No nesting, so no cycles. A page is the one place that orders prose. |
| A body over 2,400 characters gets a warning. | That is the size at which the indexer splits a chunk. Today 59 of the 2,198 sections in the Nym network pages are over that size. |
| An empty body is legal. | A heading can group the sections below it and hold no text of its own. Four headings in the Nym network pages do. |

### A diagram snippet

A `.mmd` file is a Mermaid file with no frontmatter, so it opens in any Mermaid tool and renders as it does today. Its title in the record is its file name. `split` names each file from the slug of its section and a number. The one diagram in `bootstrapping-and-overrides` sits in the section "Loading order in one call", so its file is `loading-order-in-one-call-d1.mmd`.

### Model snippets

A model is a named set of typed entities that reference each other by id. Prose explains a thing in sentences. A model states the parts of a domain as data, so that a tool can check the data, draw it, and project it to text. The `model` kind covers each such model.

Each model has the same parts:

| Part | Content |
|---|---|
| Entity types | The kinds of thing in the model, as TypeScript types. |
| Data | The entities, in typed modules that the type checker covers. |
| Adapter | A small module that evaluates the data and writes one record for each entity. The record holds the id, a text projection in `body`, the data in `meta.data`, the edges in `refs`, and the aspect. |
| Integrity rules | Checks that run in CI, for example that each referenced id exists. |
| Views | The components that render an entity on a page, selected by `view` in a citation. |

The record ids have the form `<repo>/<model name>/<entity type>/<id in the data>`. Edges between two models use the same relation list as all other edges, so the graph has one vocabulary.

Snippetisation makes it possible to write down why the network exists, and not only how it works. The threat model states the adversaries and the protections. A second model states the motivation: who has a stake in the network, what drives the work, the goals, and the principles. Edges then connect the two models and the rest of the content: a threat-model property `realises` a goal, and a requirement `realises` a principle. A reader or an agent can then go from a piece of code to a requirement and then to a property. From the property, the next step is the goal behind it.

| Model | State | Source |
|---|---|---|
| Threat model | Data exists | `documentation/docs/lib/privacy-model/` |
| Motivation model | New. Entity types and content are TBD(Max). The ArchiMate motivation layer is the inspiration: stakeholder, driver, goal, principle. | TBD(Max): a new directory under `.nym-codex/models/` is the default |
| Deep-dive animations | The content is inside component code today (see the end of this section) | `documentation/docs/components/threat-model/` |
| Other models | TBD(Max) | |

The motivation model has no content yet. Its text states why the network exists, so the people who own that reason write it, and a tool does not. Its records have the aspect `motivation`.

#### The first model: the threat model

The threat model is the base for the other content. It states who the adversaries are, what each one observes, and which configuration defends against which one. The model is typed data in `documentation/docs/lib/privacy-model/`.

| Entity | Count | Id in the data |
|---|---|---|
| Actor | 3 in the spine, 1 more in each worked example | `L2`, `L3L`, `L3G`, `L1` |
| Vector, with its countermeasures | 3 | `V1` to `V3` |
| Property | 2 | `P1`, `P2` |
| Scenario | 8 generic, 6 in the worked examples | `mixnet`, `dvpn-single`, ... |
| Invariant | A list in each worked example | Per example |

An animated network diagram is a view of a scenario. The scenario also holds the verdict matrix and, for each actor, what it sees, what it cannot see, and the residual risk. So the diagram and the claims it illustrates are one entity, and the plan keeps them together.

Each threat-model entity becomes one record of kind `model`:

| Part | Content |
|---|---|
| Source | The TypeScript modules, unchanged. The type checker covers them, and some topologies come from builder functions. The record builder evaluates the modules (`lib/retrieval/ts-data.mjs` does this today) and takes the result. |
| `id` | From the id in the data: `nym/threat-model/actor/L2`, `nym/threat-model/scenario/mixnet`, `nym/threat-model/example/wallet/scenario/mixnet-ipr`. |
| `body` | A text projection of the entity. The projection of a scenario holds the summary, each route as an ordered list of hops with its mode, and each verdict. For each actor, it also holds the sees, cannot-see, and residual lists. |
| `meta.data` | The evaluated entity as JSON. A site renders the diagram, the matrix, and the assessment panel from it. |
| `refs` | The edges that the data holds. An actor `observes` each vector that lists it. A scenario is `assessed-against` each actor and property. A worked-example scenario `specialises` the generic one. |
| `aspect` | `motivation`. An invariant has `requirement`. |

A prose snippet cites an entity and names the view: `<Snippet id="nym/threat-model/scenario/mixnet" view="diagram" />`. The views are the present components (`NetworkDiagram`, `MetadataPanel`, `GenericScenarioView`, `ConfigMatrix`, the reference cards). The citation replaces the `export const scenario = requireGenericScenario(...)` lines in pages.

The docs copy is the one source. The codex repository holds a second copy today (`codex/lib/privacy-model/threat-model.ts`), and the two copies differ. For L2, the codex copy says it cannot observe "Only what the client never sends it". The docs copy gives a different list with a separate mixnet list. The codex repository deletes its copy and reads the records.

Three guards protect the threat model. The motivation model gets the same three when it exists:

| Guard | What it does |
|---|---|
| Integrity check in CI | Each referenced id exists. Each scenario has an assessment for each spine actor, or is on a short list of stated exceptions. A verdict has one source. Today 16 verdicts are in the matrix and are absent from the per-actor assessment. The `end-to-end` scenario has no L2 assessment (this change notes it and does not fix it). |
| Owner review | A CODEOWNERS entry on the model directory. The PR comment lists each entity whose record changes. |
| Provenance | Each record names its source in `meta`: the note "Network privacy" by Claudia Diaz, sections 2, 3, and 5. |

The index does not see these items today, and gets them from the records:

| Gap | Cause |
|---|---|
| The 6 worked-example scenarios, their L1 actors, and their invariants | `lib/retrieval/mdx.mjs` matches `requireGenericScenario` alone, and `projections.mjs` loads `examples/generic.ts` alone. |
| The second scenario on a page that shows two | The match takes the first one. |
| The comparison matrix | `ConfigMatrix` has no projection. |
| The route each diagram draws | The projection prints the caption. It does not print the hops. |

Four animations are not scenarios and keep their text inside component code: `PacketAnatomy`, `MixnetDeepDive`, `DvpnCoverTraffic`, `SurbDirectory` (about 1,500 lines of TSX). For each one, move the content into a typed data module, then add it as a `model` entity. `PacketAnatomy` is first. Its sizes come from constants in the Rust code (`common/nymsphinx`, `common/ip-packet-requests`), so a check against the code is possible.

### How a snippet cites code

A prose snippet names the file and the symbol. It does not hold the code.

```mdx
<Snippet id="nym/common/network-defaults/src/network.rs#NymNetworkDetails" />
```

A method uses the type as a qualifier: `nym/nym-signers-monitor/src/test_result.rs#TestResult::quorum_unavailable`. The full qualifier rules are not fixed yet: TBD(task 2.3). The open cases are two trait impls with one method name, `cfg` variants, constants, and items that a macro generates. Task 2.3 fixes the rules against the real cases, and the code id form is provisional until then.

The extractor finds each citation and finds the item with tree-sitter. The manifest has no list of symbols: a citation is the request to extract. If the symbol is gone or matches two items, the extractor fails and names the snippet that cites it. That failure is the drift check. A rename in Rust code fails CI in the same PR, before a wrong excerpt reaches a reader.

The source repository commits no extracted code. On each PR, the check runs the extractor and posts one comment: each excerpt that changes, as a diff, and the pages that show it. A reviewer sees the excerpt change in the same PR as the code change, and the PR holds no generated file. A Rust contributor runs no Node tool. The check fails on a citation that does not resolve. The push job extracts the bodies again at the merged commit, and `code.jsonl` in codex keeps their history. The shared index walk (`collect.mjs`) excludes `.nym-codex/`, because its plain Markdown walk reads `.mdx` prose snippets as pages.

### A recipe

```json
{
  "title": "The network details model",
  "sections": [
    { "id": "nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/intro" },
    { "id": "nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/nymnetworkdetails-v1", "level": 2 },
    { "id": "nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/constructors", "level": 2 }
  ]
}
```

The path of the recipe under `pages/` is the path of the page on the site. Materialise writes the `title` as the `# ` line of the page. A section with no `level` renders with no heading. A section with a `level` renders the snippet `title` as a heading at that level.

A site that is not codex writes its own recipes in its own repository and reads the same records. The first such site: TBD(Max). Each snippet is in use in its own repository: a recipe lists it, or a prose snippet that a recipe lists cites it. So codex shows each snippet that a second site can use. Recipes that select by metadata ("all snippets where `audience` is `operator`") are a later step. The first version lists ids.

### Aspect: the question a snippet answers

Each record has an `aspect` from a closed list. The aspect separates why a thing exists from what it must do and from how the code does it. A flat set of chunks cannot make that separation.

| Aspect | Question | Content | Who sets it |
|---|---|---|---|
| `motivation` | Why does it exist? | Threat-model actors, vectors, properties, scenarios | The model adapter |
| `requirement` | What must hold? | Threat-model invariants, OpenSpec requirements | The adapters |
| `design` | How is it built? | The specification pages | The author, in frontmatter. `split` takes it from the heading. "Why it exists" gives `motivation`, and "Code" gives `implementation`. A heading with no rule gives `design` and a mark for review. |
| `implementation` | What is the code? | Code snippets | The extractor |
| `operation` | How does a person run it? | Operator guides | The author |

The full rule table from heading to aspect: TBD(stage 2 owner). Two rules exist.

A prose snippet with no `aspect` fails the check. The aspect is a second axis next to the Diataxis type of a page. Diataxis says what form a text has, and the aspect says which question it answers.

The aspects and the `realises` edges form a chain. A code item realises a requirement, and a requirement realises a property. A property stands against an actor and a vector. A reader, or an agent, can go from a piece of code to the reason it exists.

### Authority

TBD(Max). A sense-making agent has to know which source wins if two sources disagree. A proposed field has three values: `normative` (the threat model, the OpenSpec requirements), `descriptive` (a specification page), `illustrative` (a guide, an example). The field is not in the record type or the spec until Max makes this decision.

### Links

The 363 Nym network pages hold 1,845 links that start with a codex route (`](/specifications/...)`). Of those links, 120 have an anchor. The pages also hold 11 links to an anchor on the same page. A codex route is wrong on a second site, and it has no meaning in the index.

A link in a snippet names a target by id and holds no route:

```md
See [the constructors](snippet:nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/constructors).
```

Materialise resolves each `snippet:` link for the site it builds. It finds the page whose recipe lists the target and writes that page route with the anchor. A target that is on no page of that site fails the build. The record keeps the `snippet:` form, so the index holds a reference that is correct for each site. The tools also add a `relates-to` edge for each link.

The anchor of a section is the last part of its id (the file name). Materialise writes the anchor with the Nextra custom id syntax: `## Constructors [#constructors]`. A change to the `title` changes the heading text and keeps the anchor. A link to a section with no `level` goes to the top of its page.

`split` converts each link in a page to the `snippet:` form. A link to a page that is not split yet stays a route until its target moves.

### Id rules

| Kind | Id | Example |
|---|---|---|
| Prose | `<repo>/<path under snippets/, no extension>` | `nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/constructors` |
| Diagram | The same rule | `nym/specifications/nym-network/technical-details/discovery-urls/bootstrapping-and-overrides/loading-order-in-one-call-d1` |
| Model | `<repo>/<model name>/<entity type>/<id in the data>` | `nym/threat-model/actor/L2` |
| Code | `<repo>/<source file path>#<symbol>` | `nym/common/network-defaults/src/network.rs#NymNetworkDetails` |

The id of an authored snippet is its file path, so a `git mv` is the one way to change it, and git shows the rename. A change to the `title` does not change the id. A code id has a `#` and a file extension, and the other two kinds have neither, so the id spaces do not collide.

### The record

Each snippet becomes one record. The record is the contract between a source repository and each consumer.

```ts
export type SnippetKind = "prose" | "code" | "diagram" | "model";

// A directed edge from this snippet to another one.
export type RefRel =
  | "cites" // the body embeds the target with <Snippet id>
  | "relates-to" // the default for a reference with no relation, and for a link
  | "depends-on" // the reader needs the target to understand this snippet
  | "realises" // this snippet carries out the target: code a requirement, a requirement a property
  | "specialises" // this snippet is a particular case of the target
  | "observes" // model: an actor, to a vector it can observe
  | "mitigates" // this snippet reduces the target vector
  | "assessed-against"; // model: a scenario, to an actor or a property

// The question a snippet answers.
export type Aspect =
  | "motivation" // why does it exist: adversaries, vectors, properties, assessments
  | "requirement" // what must hold: invariants, SHALL statements
  | "design" // how is it built: specifications
  | "implementation" // what is the code
  | "operation"; // how does a person run it

export interface SnippetRef {
  id: string;
  rel: RefRel;
}

export interface Snippet {
  id: string; // unique across all repositories
  kind: SnippetKind;
  aspect: Aspect;
  repo: string; // "nym"
  path: string; // the snippet file, or for code the source file
  lines?: [number, number]; // code: 1-indexed, inclusive, in `path`
  title: string; // prose, diagram: frontmatter title. code: the symbol
  lang?: string; // code: "rust". diagram: "mermaid"
  body: string; // prose, diagram, code: the source text. model: the text projection
  hash: string; // sha256 of `body`
  refs: SnippetRef[]; // typed edges to other snippets, sorted by id
  meta: Record<string, unknown>; // other frontmatter, passed through
}
```

A record holds content and no fact about a site. A URL, a page title, and the source commit depend on the site or on the push, so they are outside the record:

| Value | Where it comes from |
|---|---|
| Source commit | One `manifest.json` for each repository in codex. With a commit in each record, each push changes each line of each file. |
| GitHub permalink for code | The consumer builds it from the manifest commit, `path`, and `lines`. |
| Page URL and page title | The consumer finds them in the recipes of the site it serves. The first recipe in path order that lists the snippet gives both. |

The present retrieval `Chunk` (`documentation/docs/lib/retrieval/types.ts`) has `id`, `source`, `title`, `heading`, `url`, `text`, `tokensEst`. The indexer builds one from a record and the recipes of one site. It keeps `id` as it is. It takes `source` from `repo` and `kind` (`nym-docs`, `nym-code`, as the spike does), and `heading` from the snippet `title`. It takes `title` and `url` from the recipe, and `text` from `body`. In `text`, the indexer replaces each citation tag with the title of its target. The indexer makes no cut of its own, apart from the split of an oversized body into `id~0`, `id~1` parts that it does today.


### Edges between snippets

`refs` makes the records a graph. Each entry is a directed edge from this snippet to a target, with a relation.

| Source of the edge | Relation | Who writes it |
|---|---|---|
| A `<Snippet id>` citation in a body | `cites` | The tools |
| A reference between model entities | `observes`, `assessed-against`, `specialises` | The tools, from the data |
| A `snippet:` link in a body | `relates-to` | The tools |
| A `refs` list in the frontmatter of a prose snippet | The one the author gives, or `relates-to` for a plain id | The author |

A code record and a record from an adapter have no frontmatter. Edges from those records are in `.nym-codex/edges.json`, a list of `{ from, to, rel }` that a person writes and the check validates. A `realises` edge from a code item to a requirement goes in this file.

An author can write a plain id or an object, and the two forms can be in one list:

```yaml
refs:
  - nym/threat-model/actor/L2
  - { id: nym/specifications/nym-network/technical-details/rust-sdk/sending-and-receiving/surbs, rel: depends-on }
```

In a record, each entry has the object form, so a consumer handles one shape. The field name is `rel`. The name `kind` is not available, because `kind` is the kind of the snippet in the same record.

The list takes its ideas from the ArchiMate 3.2 relationship set and from the UML relationships. The two agree on a core: realisation, specialisation, dependency, association. The list does not copy either one. `observes` is ArchiMate's access relation for the one case this content needs. `mitigates` is ArchiMate's influence relation with a negative sign. `assessed-against` has no match in either one and comes from the threat-model data. Composition and flow are absent: a recipe groups the snippets, and the hops of a route are inside one scenario record.

The relations are a closed list in the `RefRel` type. A relation that is not on the list fails the check. A new relation is a change to the type, with a line in this plan that says what it means. With a free vocabulary, authors write `depends-on`, `dependsOn`, and `requires` for one meaning. A query on the graph then misses two of the three.

An edge to an id with no snippet fails the check. The source repository cannot check an edge to a snippet in a second repository, so the codex build checks it.

### The layout in codex

```
codex/
  snippets/<repo>/prose.jsonl     one record on each line, sorted by id
  snippets/<repo>/code.jsonl
  snippets/<repo>/diagram.jsonl
  snippets/<repo>/model.jsonl
  snippets/<repo>/manifest.json   the source commit of the last push
  pages/<repo>/**/<page>.json     recipes, and the _meta.json files
  content/                        generated at build time from the two directories above
```

One record on each line gives a git diff of one line for each changed snippet.

## How each consumer reads snippets

| Consumer | Reads | Mechanism |
|---|---|---|
| codex site | Records and recipes | A build step, materialise, writes one `.mdx` page for each recipe into `content/`. The page holds the headings and the prose bodies. It holds each code citation as a fenced block with a link to its GitHub permalink. It holds each diagram as a `mermaid` block and each model citation as its component. Materialise removes each generated directory before it writes, so no stale page stays. Nextra then builds as it does today. |
| A second site | Records, and its own recipes | The same materialise step. |
| Public index | The four `.jsonl` files | The indexer embeds one record as one chunk. |
| Private index | The same files from the private target | The same indexer with the private model. |

Materialise writes real `.mdx` files because Nextra needs files at compile time. From those files, Nextra builds the table of contents, the sidebar, the search index, and the syntax highlight. A component that renders a recipe at request time has to rebuild each of those. That component stays possible later, because the records are plain data.

## Authored snippets: the costs and the answers

A person writes each prose unit as its own file. This form gives the third goal: a site is a set of recipes over one set of snippets. The form has two costs.

| Cost | Answer in the plan |
|---|---|
| The present pages are whole files: 363 Nym network pages in codex for this repository, about 840 pages across codex and the docs. | A migration tool, `split`, cuts each page at its `##` to `####` headings into snippet files and writes the recipe. The work is mechanical. There are two checks. The first check is a round trip: materialise the split result and compare it with the original page in codex with `git diff`. The second check compiles each snippet alone as MDX. The reason is that a cut through a JSX element gives broken parts that still join to a correct page. The 363 Nym network pages hold no JSX element today. Each page has its title as a `# ` line, and `split` moves the title to the recipe. |
| A reviewer reads fragments, and the order is in a different file. | A local command prints the materialised page for a recipe. A preview of the codex site for each PR in this repository is possible later, as a push to a preview branch in codex. The pilot does not build it. |

`split` makes the files. It does not make each section clear alone. Stage 3 of the plan does that work, with its own measure. A page that is split and not yet rewritten renders the same as before, so the site loses nothing in the meantime.

Some sources stay whole files because a different tool owns their format: `openspec/specs/*/spec.md` (the OpenSpec CLI), the TypeDoc output, the autodoc command output. An adapter derives records from those in a later phase. Do not split them by hand.

## Stages: how the data gets ready

The corpus is not in the target form today, and the plan does not need it to be. Most sections are not clear alone. The requirements do not name the threat-model property they serve. If a tool sets the aspect today, each section gets the same one. Each stage changes the data one step, and each has a measure that says how far the data is. Expect a low number at the start of a stage.

Each stage runs on the pilot section first (`technical-details/discovery-urls`, 5 pages), then on the other 358 Nym network pages.

| Stage | Change to the data | Who does it | Measure | Today |
|---|---|---|---|---|
| 0. Baseline | None. Label a set of "why" questions and a set of developer questions against the chunks that exist now. Record recall for plain search, a keyword hybrid, and expansion along the links that the pages already hold. | People write the labels | Recall at 6 for each question set | No labelled set exists |
| 1. Move and split | The pages move into `.nym-codex/` as snippet files and recipes. The text does not change. | The `split` tool | Round trip: each page renders as before. Per-snippet compile passes. | 0 of 363 pages |
| 2. Mechanical enrichment | Links become snippet ids. The aspect comes from the heading template. A tool checks each inline source path. Each code fence becomes a citation by symbol. A tool removes each section that holds only a list of pages, because the recipes and the sidebar data give that list. | Tools, with a person for the fences | Links resolved, of 1,845. Source paths that exist, of 1,085. Fences replaced, of 24. Sections with an aspect from a rule, of 2,199. | 0 in each |
| 3. Standalone pass | Each section gets a rewrite where needed, so that it is clear with no page around it. The result has a title that says what the section is about, and no reference to text above or below. | A model proposes, a person reviews | Share of sections that pass a standalone read by a person who did not write them. Target: 90%. | About 20% in an 11-page sample |
| 4. Threat model | Each entity becomes a record. Each cell of the comparison matrix gets an id as one claim. The integrity check runs in CI. The codex repository deletes its copy. | The adapter | Entities with a record. Integrity faults. | 2 copies that differ. 16 verdicts stated in one place and not the other. 1 missing assessment |
| 5. Requirement linkage | Each requirement states the property, actor, or vector it serves, or states that it serves none. Code items get `realises` edges to requirements. People write specs for the areas that have none (Sphinx, cover traffic, mixing delay). | People, with model proposals | Share of the 492 requirements with a stated link. Count of complete chains from code to motivation. | 0 requirements with a link. 1 weak chain found by hand |
| 6. Index and MCP | The indexer reads records. The context prefix, edge expansion, and `get_related` become active, each behind a switch. | Tools | Recall at 6 on the stage 0 sets, with each mechanism on and off | The stage 0 numbers |

| Stage | Owner | Target date | Result |
|---|---|---|---|
| 0. Baseline | TBD(Max) | TBD(Max) | Recall at 6, "why" set: plain TBD, hybrid TBD, link expansion TBD. Developer set: plain TBD, hybrid TBD, link expansion TBD. Standalone share: TBD |
| 1. Move and split | TBD(Max) | TBD(Max) | TBD |
| 2. Mechanical enrichment | TBD(Max) | TBD(Max) | TBD |
| 3. Standalone pass | TBD(Max) | TBD(Max) | TBD |
| 4. Threat model | TBD(Max) | TBD(Max) | TBD |
| 5. Requirement linkage | TBD(Max) | TBD(Max) | TBD |
| 6. Index and MCP | TBD(Max) | TBD(Max) | TBD. A mechanism stays on at a gain of TBD(stage 6 owner) or more |

Stages 0 and 4 do not depend on the split and can start at once. Stage 5 is the largest piece of content work, and no tool can do it alone. A person who knows the system has to say why each requirement exists.

The measures also say when to change the plan. If stage 6 shows no gain from a mechanism, that mechanism stays off and the record keeps the data. If stage 3 stays far below its target after real effort, the page becomes the unit of authoring again. A tool then derives the records from headings. The record format is the same, so no consumer changes.

Stage 1 includes the work to build CI and the push job: the PR check with its excerpt comment, and the push to codex. Both run for each later stage.

The work after these stages covers `nym-vpn-client`, the private target and the private repositories, and the docs site content. It also covers the four deep-dive animations and recipes that select by metadata. `nym-vpn-client` owns the 10 NymVPN threat diagrams that codex holds in `lib/privacy-model/nymvpn-diagrams.ts`.

## Evidence from the current repositories

| Fact | Value | Why it matters |
|---|---|---|
| Rust fences in codex Nym network pages | 24, on 12 pages | The size of the code work in stage 2. |
| What the fences hold | Whole items, methods inside `impl` and `trait` blocks, parts of a function body, and one formula | Top-level detection is not enough, so the extractor needs a parser. The parts of a body need a decision (P21). Task 2.1 counts each class. |
| Links to a codex route in the Nym network pages | 1,845 | `split` has to convert each one. |
| Inline source paths in the Nym network pages | 2,300 mentions of 1,085 distinct paths, on 312 pages | No tool checks them today. A path that no longer exists is drift, and the pages hold far more paths than fences. |
| Headings that repeat across pages | "Code" 77, "Technical details" 64, "Used by" 61, "Why it exists" 60, "Status" 60 | The template gives the aspect by rule. The titles have no meaning without the page name. Stage 3 fixes that. |
| OpenSpec requirements that name a threat-model property, actor, or vector | 0 of 492 | The size of stage 5. |
| `file:NN` line citations in codex | 215, none in Nym network pages | Line citations are a `nym-vpn-client` problem. The pilot does not meet them. |
| Pages in codex | 569: 363 Nym network, 111 NymVPN, 80 openspec, 13 other, 2 index pages | The size of the split in stage 1. |
| `mermaid` blocks in codex | 130, of which 83 in Nym network pages | The diagram snippet count. |
| `code-examples/sdk/typescript/*.mdx` in the docs | 7 files, no importer | Dead files. Delete them, or convert them to snippets. |

## Relation to the shared retrieval index

The shared index design (`add-shared-retrieval-index`, decision 10) moves the code chunker to tree-sitter. The extractor needs the same thing: a function that takes a file and returns its items with name, kind, and line range. Build that function once.

| Output | Items | Purpose |
|---|---|---|
| Code index | All items in the indexed roots | Broad code search. |
| Code snippets | The items that a prose snippet cites | Excerpts in pages, and a curated set for the index. |

Code snippets do not replace the broad code index. The indexer does not embed a code record as a chunk of its own. The broad index holds the same item already, and if the indexer adds a second copy, that copy competes with the item in the results. A code record gives a page its excerpt. The `cites` edge tells the index which items a person chose to explain, and the index can use that edge to rank results.

The shared index change is on the branch `max/shared-mcp` and is not in this branch. Stage 6 needs it.

## Placeholders

Each value below is not known yet. The same marker, `TBD(<who>)`, is at the place in this file or in the OpenSpec change where the value belongs. A search for `TBD(` finds each one.

| # | Placeholder | Who fills it | Needed before |
|---|---|---|---|
| P1 | Owner of this plan, and the reviewers | TBD(Max) | Review by the team |
| P2 | Owner of each stage | TBD(Max) | The stage starts |
| P3 | Target date of each stage | TBD(Max) | Stage 0 starts |
| P4 | Baseline recall at 6: plain search, keyword hybrid, link expansion, for each query set | TBD(stage 0 owner) | Stage 6 |
| P5 | Standalone share in a sample of 100 sections | TBD(stage 0 owner) | Stage 3 |
| P6 | The full rule table from heading to aspect (two rules exist) | TBD(stage 2 owner) | `split` runs on all pages |
| P7 | The qualifier rules for a code id | TBD(task 2.3) | A person commits the first code citation |
| P8 | The meaning of the relation `creates`, and one real edge that uses it. The relation is the example from Mark S and is not on the list. | TBD(Mark S) | It joins the relation list |
| P9 | An authority field: whether it exists, and its values (proposed: normative, descriptive, illustrative) | TBD(Max) | Stage 4 |
| P10 | The metadata vocabulary for selection next to `aspect` (`audience`, `product`, Diataxis type). Recommendation: pass frontmatter through now, and fix the vocabulary when the first recipe needs to select on it. | TBD(Max) | The first recipe that selects by metadata |
| P11 | The second site: its name, its owner, and its first page list | TBD(Max) | A site outside codex uses recipes |
| P12 | Owner of `overview/`, `threat-model/`, and `prior-work/` in codex. No recommendation: they describe more than one product. | TBD(Max) | Those pages are split |
| P13 | The kind of write credential for codex (GitHub App or deploy key), and who creates it | TBD(infra) | Stage 1 CI |
| P14 | The CODEOWNERS names for the threat-model directory | TBD(Max) | Stage 4 |
| P15 | The exception list of the integrity check, with a reason for each entry (one known entry: `end-to-end` has no L2 assessment) | TBD(threat-model owner) | Stage 4 |
| P16 | The specs to write for areas with none (known: Sphinx, cover traffic, mixing delay) | TBD(stage 5 owner) | Stage 5 |
| P17 | The place of materialise, and how other repositories get the tools (a package, or a copy). Recommendation for the pilot: `split` and the extractor in this repository, materialise in codex. Decide on a shared package when `nym-vpn-client` starts. | TBD(Max) | `nym-vpn-client` starts |
| P18 | Whether codex follows `develop` or release tags, and whether the push job commits to the default branch or opens a PR. Recommendation: follow `develop`, push a branch, and merge it when the codex build passes. | TBD(Max) | Stage 1 CI |
| P19 | The name and location of the private target, and its sync from codex | TBD(Max), not in this repository | A private repository adopts the flow |
| P20 | The gain in recall that keeps a retrieval mechanism on | TBD(stage 6 owner) | Stage 6 |
| P21 | Some fences show part of a function body: whether to cite the whole function, or add region markers to product code. Recommendation: cite the whole function, and add markers when a real case needs one. | TBD(Max) | Stage 2 |
| P23 | Which models come after the threat model, and an owner for each. The motivation model is the next one. | TBD(Max) | Stage 4 ends |
| P24 | The entity types, the location, and the content owner of the motivation model | TBD(Max) | The motivation model starts |
| P22 | Whether `.nym-codex/` is the right place for about 2,600 authored files, or a visible directory serves contributors better. Recommendation: keep `.nym-codex/` for the pilot. The tools take the directory as an argument, so a move is one rename. | TBD(Max) | Stage 1 |

## Decisions made

| Decision | Result |
|---|---|
| Unit of authoring for prose | The snippet. A page is a recipe. |
| What the source repository commits | Authored files only. The PR check posts excerpt changes as a comment, and the push job extracts code at the merged commit. Look at this again when the broad design is complete. |
| Sidebar order | JSON next to the recipes. The site build writes `_meta.js`. |
| Models | A `model` kind for each typed data model. The threat model is the first. A motivation model follows. For the threat model, the docs copy in this repository is the one source, and codex mirrors it. |
| Edges | Typed, with a closed list of relations. ArchiMate and UML are the inspiration only. |
| Aspect | Each record must have this field. It has five values. |
| Index | Context in the embedded text, edge expansion at query time, and labelled query sets. Stage 0 measures the baseline before any file moves. |
| The corpus is not ready today | Accepted. The data changes in stages, and each stage has a measure. |
| Distribution | Push from each source repository to codex. The review proposed a pull by codex at build time, and the plan does not take it. |
| codex is not public yet | Accepted. The split does not wait for it. |

## Out of scope

- The private flow in detail. This plan states only the boundary rule.
- The docs site content. It can adopt the same form after the pilot.
- The embedding provider and the store. The shared index change owns both.

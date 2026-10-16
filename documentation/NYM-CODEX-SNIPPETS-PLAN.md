# Nym codex snippets: plan

Status: draft. Owner: TBD(Max). Reviewers: TBD(Max). The implementation spec is the OpenSpec change `openspec/changes/add-codex-snippets/`. The reasons for the plan, the readers it serves, the known risks, and the alternatives are in `NYM-CODEX-SNIPPETS-CONTEXT.md`.

## Purpose

A snippet is the smallest piece of content that has an id. It is one section of prose, one diagram, one code item, or one entity of a model. A person writes it once, in the repository that owns the subject. Each place that needs the content reads the same snippet by its id, and nobody copies it.

The places that read snippets:

| Reader | What it does with a snippet |
|---|---|
| codex | Shows it on a page of the knowledge base. The page imports the snippet file. |
| A documentation site | Builds its own pages, which import the same snippet files. |
| The retrieval index | Embeds each snippet as one chunk, with its id as the key. |
| The MCP server | Gives an agent the snippet, its edges, and the commit it came from. |
| The coherence agent | Checks a new idea against the commitments of the project: it judges the idea against statements and follows edges to the reason behind each one. The agent is designed separately. This plan gives it the records. |
| A view generator | Builds pages for one kind of reader, for example an auditor, from a viewpoint: a selection of records, the edges to follow, and a page template. |

So the plan has six goals:

1. One source for each piece of content. A snippet that shows code cites it by symbol, and a tool takes the text from the source file.
2. One input format for the retrieval index. A snippet is a chunk, so the indexer makes no cut of its own.
3. Any number of sites from one pool of content, with no copy of the prose.
4. A graph of the content. Each snippet says which question it answers (why, what, how), and typed edges connect code to its requirement and to the motivation behind it.
5. Pages for a defined reader, made by a tool from the corpus, with a minimum of prompted text. A viewpoint states who reads and what they ask, and the tool makes the view.
6. Guides that a tool can check. An operator guide or a tutorial follows the archetype of its Diataxis type, and a linter checks the page against it.

The rest of this file describes the form of a snippet and the flow from a source repository to codex. It also gives the stages that bring the present content into that form. The present state and its numbers are in the section "Evidence from the current repositories".

## From code to the reason it exists

**Snippetisation makes it possible to write down why the network exists, and not only how it works.** The threat model states the adversaries and the protections. A second model states the motivation: who has a stake in the network, what drives the work, the goals, and the principles. Edges then connect the two models and the rest of the content: a threat-model property `realises` a goal, and a requirement `realises` a principle.

```
 code item  --realises-->  requirement  --realises-->  property        --realises-->  goal
                                |                      (threat model)                 (motivation model)
                                |
                                +-------realises---->  principle
                                                       (motivation model)
```

**A reader or an agent can then go from a piece of code to a requirement and then to a property. From the property, the next step is the goal behind it.** Today no requirement names a property, actor, or vector (0 of 492), and the motivation model does not exist. Stages 4 and 5 build this chain, and the coverage report shows where it is still empty.

## Where the content stands

The investigation for this plan found one gap that matters more than the others:

**No OpenSpec spec defines the core mixnet mechanisms: the Sphinx packet format, how the client sends cover traffic, and the delay that a mix node adds to each packet.** Specs mention them only as inputs. `nym-network-monitor` builds Sphinx packets for its tests. `sdk-service-provider-example` turns cover traffic on in a client configuration. `nym-swizzle` specifies application-layer delays for wallets, not the delay of a mix node. These mechanisms give the network its privacy properties. So the chain from code to the reason it exists breaks at its most important link: for the code that matters most, no requirement exists to link to. It also leaves the coherence agent with no commitment to check an idea such as "mix nodes should forward each packet at once" against.

The other gaps, each with the stage that closes it:

| Gap | Size | Stage |
|---|---|---|
| No OpenSpec requirement names a threat-model property, actor, or vector | 0 of 492 requirements | 5 |
| The motivation model does not exist | No entity types, no content | After 4 |
| codex holds an older copy of the threat model, and the copies differ | 2 copies. 16 verdicts in the matrix and not in the per-actor assessment. 1 missing assessment | 4 |
| Most sections do not read alone | About 1 in 5 pass, in an 11-page sample | 3 |
| Links name codex routes, not ids | 1,845 links | 2 |
| No tool checks the source paths in the pages | 2,300 mentions of 1,085 paths | 2 |
| No labelled test shows if retrieval returns the right chunk | No relevance labels | 0 |

The section "Evidence from the current repositories" has the full numbers.

## Viewpoints and views

One reason to put the content into snippets is to make pages for a reader that we define, from the same corpus, by a tool. An auditor, an operator, and an SDK developer each ask different questions of the same records. Hand-made audit documentation already does part of this: typed blocks with facets in their frontmatter (`operator`, `area`, `audit-status`), and a component that selects the blocks for one operator. The intro text and the links on those pages are still hand-written.

The plan generalises that pattern with the structure that ISO/IEC/IEEE 42010 and ArchiMate use:

| Term | Meaning here |
|---|---|
| Stakeholder | Who reads: an auditor, an operator, an SDK developer. |
| Concerns | The questions that the stakeholder asks. Example: "Which MUST statements have no code that realises them?" |
| Viewpoint | A data file that answers the concerns: which records to select (aspect, kind, facets), which edges to follow and how far, the order, and the template of each page. |
| View | A page that the generator makes from one viewpoint and the records at one commit. |

The plan says "viewpoint" and not "observer", because the threat model uses "observer" for an adversary, and `observes` is an edge relation.

### Three levels of text

The generator uses a level only where the level before it cannot do the job:

| Level | What it makes | How |
|---|---|---|
| 0. Deterministic | Selection, traversal, order, record bodies, claims, code excerpts, matrices, coverage gaps | A query over the records, and a template |
| 1. Template text | "Credential Proxy: operated by centralised infrastructure. Specification: the credential proxy service page." | Record fields put into a sentence |
| 2. Generated prose | A short introduction, or a summary that connects the sections | A prompt in the viewpoint file, with the input ids listed |

Rules for level 2:

| Rule | Reason |
|---|---|
| The prompt is part of the viewpoint file, in git. Nobody types it in a chat. | The prompt has a history and a reviewer, like code. |
| The output is a prose record. Its `meta.generated` holds the viewpoint, the input ids, and their hashes. | The text has provenance, and the index can tell it from authored text. |
| The generator makes the text again only when an input hash changes. A person reviews each new version once. | A view with no input change has no new text to review. |
| Each generated sentence cites the ids it comes from. A section with no input states that the corpus has no evidence. | A reader, and the coherence agent, must see a gap as a gap. |

### How a view becomes a page

A view uses records, not snippet files. A record `body` has no heading, so the generator writes each heading at the level that its template needs. Hand-made pages import snippet files. Generated views compose records. Both use the same ids, the same `<Snippet id>` citations, and the same link map.

Nextra needs files at compile time, so the generator writes the `.mdx` pages of each view into a generated directory under `content/` in the target site, before `next build`. The directory is not in git. The generated prose records are in git in the target site, so that each new version has a review.

A viewpoint file lives in the target site that shows the view, because a view can take records from more than one repository. A viewpoint that reads private records lives in the private target. This is the case for an auditor viewpoint that reads audit findings.

An example of a viewpoint file. The field names are provisional:

```json
{
  "name": "auditor",
  "stakeholder": "An external security auditor",
  "concerns": ["What does each component do?", "Which requirements have no code that realises them?"],
  "pages": [
    {
      "path": "views/auditor/areas/{area}",
      "forEach": { "facet": "area" },
      "sections": [
        { "title": "Components", "select": { "facets": { "area": "{area}" } } },
        { "title": "Requirements", "select": { "aspect": "requirement", "facets": { "area": "{area}" } },
          "follow": [{ "rel": "realises", "direction": "backward" }] },
        { "title": "Gaps", "coverage": "requirements-without-code" },
        { "title": "Summary", "generate": { "prompt": "prompts/area-summary.md", "inputs": "sections" } }
      ]
    }
  ]
}
```

### What a viewpoint needs from the corpus

| Need | Where the plan gives it |
|---|---|
| A closed facet vocabulary next to `aspect` (`audience`, `operator`, `area`, `product`, `audit-status`) | P10. The facets of the hand-made audit blocks are the first candidates. |
| Sections that read alone, because a view puts a snippet in a new context | Stage 3 |
| Edges, because a view that follows edges is only as good as the graph | Stage 5, and the coverage report for the gaps |
| A measure of how much a tool can make | Stage 7: generate one hand-made audit page from a viewpoint, and compare |

## Terms

| Term | Meaning |
|---|---|
| Snippet | The smallest unit of content that has an id. It has one of four kinds: prose, code, diagram, model. |
| Prose snippet | One section of text in its own MDX file. A person writes it. |
| Diagram snippet | One Mermaid diagram in its own `.mmd` file. A person writes it. |
| Model snippet | One entity of a typed data model, for example one threat-model actor or one goal in the motivation model. A person writes the data, and a tool writes the text form. |
| Code snippet | One named item from a source file: a function, method, struct, enum, trait, or module. A tool extracts it. |
| Page | An MDX file that holds a title and imports snippet files in order. It holds no prose of its own. |
| Viewpoint | A data file that defines a reader, the questions they ask, the records to select, the edges to follow, and a page template. |
| View | A page that the generator makes from a viewpoint and the records. |
| Archetype | The form of the pages of one Diataxis type: the sections, the syntax tree nodes of each section, and the constraints. A linter checks a page against it. |
| Runbook | The archetype of a how-to page, for example an operator guide. |
| Placeholder | A token such as `<HOSTNAME>` in a command, which the reader replaces with their own value. Each token comes from one list. |
| Record | The typed data for one snippet: its body and its metadata. |
| `.nym-codex/` | The directory in a source repository that holds its snippets and pages. In this repository it is `documentation/docs/.nym-codex/` (P22). |
| codex | The public aggregation repository. It receives pushes from public source repositories. |
| Private target | A private aggregation repository. It holds all of codex plus the content of private repositories. |

## Flow

```
SOURCE REPOSITORY (nym)

 authored by people
 +------------------------------+
 | .nym-codex/snippets/  .mdx   |  prose
 | .nym-codex/snippets/  .mmd   |  diagrams
 | .nym-codex/pages/     .mdx   |  pages (imports)
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
                 |    replace the repository's paths in codex,
                 |    record the source commit once
                 v
 +------------------------------+
 | codex (public aggregation)   |      +--> codex site: Nextra builds the pages as they are
 | snippets/nym/  files         |      |
 | snippets/nym/*.jsonl         | -----+--> another site: its own pages, same snippet files
 | content/<owned paths>/       |      |
 +---------------+--------------+      +--> public index (Voyage)
                 |
                 | 3. sync (one direction)
                 v
 +------------------------------+
 | private target               | <--push--- .nym-codex/ in a private repository
 | codex + private content      | ---------> private index (self-hosted model)
 +------------------------------+
```

The flow has three rules:

1. A source repository owns `snippets/<repo>/` in the target, and the page directories under `content/` that its manifest lists. It owns no other path. The push job replaces each of them in full, and git computes the diff. A renamed or deleted snippet disappears from codex with no extra logic.
2. A private repository pushes to the private target and holds no credential for codex. Content moves from codex to the private target and does not move back.
3. Do not edit pushed content in codex by hand. To change it, change the source and let CI push.

## Form

### The directory in a source repository

In this repository the directory is `documentation/docs/.nym-codex/`, inside the docs package, and not at the repository root. The docs site imports the snippet files, and its Turbopack build reads no file outside the docs package: Next.js 15.5 needs `turbopack.root` to equal `outputFileTracingRoot`, and that root is the docs package, because the MCP tracing paths and the Vercel function bundle resolve against it. The tools read Rust source and OpenSpec specs from the repository root as before, because they are Node scripts and not part of the bundle. Each path in this file that starts with `.nym-codex/` is under `documentation/docs/`. A move to the repository root stays one rename, if both roots move to the repository root one day (a case for that is a docs page that imports a monorepo package directly).

```
.nym-codex/
  manifest.json            authored: the repository name, its GitHub URL, a page root for each site, the page directories it owns in codex
  edges.json               authored: edges from records that have no frontmatter (code, adapters)
  snippets/**/<name>.mdx   authored: one prose snippet in each file
  snippets/**/<name>.mmd   authored: one diagram snippet in each file
  pages/**/<page>.mdx      authored: one page in each file; it imports snippets
  pages/**/_meta.json      authored: sidebar order as data; the push job writes _meta.js from it
```

> Prepend `nym-` in repos to avoid name clashes if anyone is using Codex LLM tooling

### A prose snippet

This is the first section of the codex page `technical-details/discovery-urls/network-details-model`, as a snippet. The text is the text of that page. The citation takes the place of the Rust fence that the page holds now.

File: `.nym-codex/snippets/specifications/nym-network/technical-details/discovery-urls/network-details-model/nymnetworkdetails-v1.mdx`

```mdx
---
aspect: design
---

## `NymNetworkDetails` (v1) [#nymnetworkdetails-v1]

`NymNetworkDetails` (`common/network-defaults/src/network.rs`) is the value every binary in the workspace builds at startup:

<Snippet id="nym/common/network-defaults/src/network.rs#NymNetworkDetails" />
```

The file is MDX and not plain Markdown, because a body can hold a `<Snippet>` citation.

Rules for the file:

| Rule | Reason |
|---|---|
| The body has at most one heading, on its first line. The heading holds an explicit anchor that is the file name: `[#nymnetworkdetails-v1]`. A second heading fails the check. | The heading text is the title of the record. The anchor is the last part of the id, so a link to the snippet resolves to the section, and a change to the heading text keeps each link. |
| A body with no heading has a `title` in its frontmatter. A body with a heading has no `title` in its frontmatter. | Each record has one title from one place. The first section of a page has no heading, so its `title` is the page title. The check fails when that `title` differs from the `# ` line of the page that imports it. This check blocks reuse on a second site with a different page title: TBD(Max) (P26). |
| The heading level is part of the snippet. | A page imports the snippet as it is, so the snippet has the same level on each page that imports it. |
| The body is clear alone: no "as described above". A link names its target by id (see Links). | A page can import it next to any other snippet, and the index returns it with no page around it. |
| The body can cite code and diagram snippets. It imports no file and cites no prose snippet. | No nesting, so no cycles. A page is the one place that orders prose. |
| A body over 2,400 characters gets a warning. | That is the size at which the indexer splits a chunk. In the Nym network pages, 59 of the 2,198 sections are over that size. |
| A snippet can hold a heading and no text. | A heading can group the sections below it and hold no text of its own. Four headings in the Nym network pages do. |

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

The section "From code to the reason it exists" near the top of this file states what the models are for. The motivation model is the second model.

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

A prose snippet cites an entity and names the view: `<Snippet id="nym/threat-model/scenario/mixnet" view="diagram" />`. The views are the docs site components (`NetworkDiagram`, `MetadataPanel`, `GenericScenarioView`, `ConfigMatrix`, the reference cards). The citation replaces the `export const scenario = requireGenericScenario(...)` lines in pages.

The codex repository has older copies of some of these components, taken from the docs before later updates. Its `NetworkDiagram` takes a `topology` and tint props, and the docs `NetworkDiagram` takes a scenario. The codex repository has no `MetadataPanel`, `GenericScenarioView`, or `ConfigMatrix`. So both sites use the docs components as one set, shipped to codex as a package or a copy, and codex deletes its older copies. This is part of stage 4.

A view is a client component, and the data reaches it through a small server component. The server component reads `model.jsonl` with `node:fs` (the pattern of `codex/lib/mdx-data.ts`), finds the record by id, and passes `meta.data` to the view. The data then stays out of the generated page.

The docs copy is the one source. The codex repository holds an older copy (`codex/lib/privacy-model/threat-model.ts`), taken from the docs before later updates. The copies now differ. For L2, the codex copy says it cannot observe "Only what the client never sends it". The newer docs copy gives a different list with a separate mixnet list. This is the drift that snippets stop: the codex repository deletes its copy and reads the records.

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

A method uses the type as a qualifier: `nym/nym-signers-monitor/src/test_result.rs#TestResult::quorum_unavailable`. A citation names a unique suffix of the item's path in the file, and a method of a trait impl is `<Type as Trait>::method` (D5 has the rules). Items that a macro generates are not citable. A form for `cfg` variants is still open: TBD(task 2.3).

The extractor finds each citation and finds the item with tree-sitter. The manifest has no list of symbols: a citation is the request to extract. If the symbol is gone or matches two items, the extractor fails and names the snippet that cites it. That failure is the drift check. A rename in Rust code fails CI in the same PR, before a wrong excerpt reaches a reader.

The source repository commits no extracted code. On each PR, the check runs the extractor and posts one comment: each excerpt that changes, as a diff, and the pages that show it. A reviewer sees the excerpt change in the same PR as the code change, and the PR holds no generated file. A Rust contributor runs no Node tool. The check fails on a citation that does not resolve. The push job extracts the bodies again at the merged commit, and `code.jsonl` in codex keeps their history. The shared index walk (`collect.mjs`) excludes `.nym-codex/`, because its plain Markdown walk reads `.mdx` prose snippets as pages.

### A page

A page is an MDX file that imports snippet files in order:

```mdx
---
title: "The network details model: NymNetworkDetails and its constants"
description: "The NymNetworkDetails struct family in common/network-defaults, and the constant tables that fill it in for mainnet and sandbox."
schemaType: "TechArticle"
section: "Specifications"
lastUpdated: "2026-10-14"
---

import Intro from '@snippets/nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/intro.mdx'
import NymNetworkDetailsV1 from '@snippets/nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/nymnetworkdetails-v1.mdx'
import Constructors from '@snippets/nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/constructors.mdx'

# The network details model

<Intro />
<NymNetworkDetailsV1 />
<Constructors />
```

The import path is `@snippets/` plus the snippet id plus `.mdx`, so the id is visible in the page. The path of the page under `pages/` is the path of the page on the site. Nextra compiles each imported file as MDX and adds its headings to the table of contents of the page (`remark-headings` in Nextra 4.6.1 does this for each `.mdx` import). The search index and the syntax highlight work as they do for any page. So codex needs no build step for pages, and an edit to a snippet shows in `next dev` at once.

A page holds frontmatter, one `# ` title line, import lines, and one tag for each import. It holds no prose of its own. The one exception is a wrapper element that the archetype of the page names (see "Archetypes: runbooks and tutorials").

The page frontmatter keeps the SEO fields that the docs pages have today: `title`, `description`, `schemaType`, `section`, and `lastUpdated`. Both sites read `title`, `description`, `schemaType`, and `lastUpdated` in `app/(docs)/seo.ts` for the page title, the meta description, Open Graph, and the JSON-LD article. A page can add more fields. Nextra reads metadata from the page file alone, so the frontmatter of a snippet (`aspect`, `refs`) never reaches the page metadata. `split` copies the frontmatter of a page unchanged. An edit to an imported snippet does not change the page file. So the push job sets `lastUpdated` in the pushed copy of each page to the newest commit date of the page file and the snippet files it imports. The job changes no other frontmatter field, and the source repository commits no change. The Nym network pages in codex have no frontmatter today, so they use the default description, and a later step can add it. A check fails on any other content, because content with no snippet has no id and does not reach the index. The check also fails when two imported snippets on one page have one anchor.

The alias `@snippets` points to `snippets/` in codex. In the source repository, the tools read `@snippets/<repo>/` as `.nym-codex/snippets/`. codex sets the alias as a `tsconfig.json` `paths` entry, which Next applies under both of its bundlers; the docs site does the same. A `turbopack.resolveAlias` key did not reach the subpaths of the alias. The Turbopack build is the one that must pass; a webpack build is a fallback.

A site that is not codex has its own pages, which import the same snippet files. The first such site is the docs site of this repository (decided 2026-10-16, stage 9). Its pages stay in `documentation/docs/content/` and import the snippet files of `.nym-codex/snippets/` through the same `@snippets` alias, so it needs no push. How a site in another repository gets the files (a push, or a package) is part of P11. Each snippet is in use in its own repository: a page imports it, or a prose snippet that a page imports cites it. So codex shows each snippet that a second site can use. Pages that select snippets by metadata ("all snippets where `audience` is `operator`") are a later step.

### Archetypes: runbooks and tutorials

The operator section of the docs (65 `.mdx` pages under `documentation/docs/content/operators/`) holds the procedures for node operators, and 25 of its pages use `Steps`. The Rust SDK tutorials (3 pages, `content/developers/rust/*/tutorial.mdx`) are procedures for developers. A reader follows a procedure in order and runs the commands. Its form has rules that a tool can check. Today no tool checks them:

| Fact | Value |
|---|---|
| Step headings with the number in the text (`###### 1. Create a service file`) | 253, on 24 operator pages: 234 at depth 6, 17 at depth 5, 2 at depth 3. 248 of them are inside `<Steps>`. In Nextra 4.6.1, `Steps` numbers each heading from depth 2 to 6 with a CSS counter (`styles/steps.css`). So the site shows two numbers on each of these headings, and on some of them the two numbers differ. |
| Links to an anchor that holds a step number (`#2-following-steps-for-...`) | 22: 12 in page files and 10 in imported files. At least two are broken today (`nymvisor-configuration.mdx:25` and `routing-conf.mdx:257`). A new step above the target breaks the others. |
| Code blocks in operator pages, from the syntax tree | 630. 43 have no language. Shell has three names: `sh` 435, `bash` 81, `shell` 12. |
| Placeholder tokens in code blocks of operator pages | About 117 distinct. The public IPv4 address of a node has at least six names: `<IPv4_ADDRESS>`, `<PUBLIC_IP>`, `<NODE_IP_ADDRESS>`, `<NODE_IPv4>`, `<HOSTv4>`, `<IPv4>`. `nym-node/setup.mdx` alone uses three of them. |
| A placeholder list | `data/csv/variables.csv` gives a description for each of a set of tokens, and `operators/variables.mdx` shows it. It names the same address `<PUBLIC_IPS>`. |
| Procedures outside the page file | 30 `.mdx` files in `components/operators/` reach the pages. The pages import 20 of them. 5 come through other `.mdx` files, up to three levels deep. 5 come through TSX components (`RunTabs` and `MigrateTabs` in `nym-node/setup.mdx`). |
| Step headings in tutorials | 15, in the form `## Step N: ...`. The operator guides use a different form. |
| Tutorial code with a source file | 0 of 16 Rust fences. The "Complete code" of the mixnet tutorial does what `examples/simple.rs` and `examples/surb_reply.rs` do together, and matches neither. The stream tutorial has a server and a client, and no SDK example matches them. |

An archetype is the form of one Diataxis type. It lists the sections of a page, maps each section to nodes of the Markdown syntax tree (mdast), and gives the constraints. The plan defines two archetypes: `runbook` for a how-to page and `tutorial` for a tutorial page. Reference and explanation pages have no archetype in this plan. A page states its type in its frontmatter (`diataxis: how-to`), and the linter reads it there. A record gets the Diataxis facet of P10 from the page that imports it. So the archetype is the structure of one facet value, and the plan gets no third classification next to the aspect.

The runbook archetype, first version. It takes its sections from the DITA task topic (see the context file):

| Section | Required | mdast | Constraint |
|---|---|---|---|
| Title | Yes | `heading`, depth 1 | States the goal: "Upgrade a node with Nymvisor". |
| Context | No | `paragraph` nodes before the first section | Why and when to do the procedure. No code blocks. |
| Prerequisites | No | `heading`, depth 2, "Prerequisites", then a `list` | Each item is a state, or a link to the runbook that gets there. |
| Steps | Yes | An `mdxJsxFlowElement` with the name `Steps` | One `Steps` for each procedure. A page with more than one procedure puts each one under its own depth 2 heading. Today 25 pages hold 75 `Steps` elements, and the changelog alone holds 19. |
| Step | Yes, one or more | `heading`, depth 3, inside `Steps` | The instruction of the step, as an imperative title. No number in the text. An explicit anchor: `### Create a service file [#create-a-service-file]`. |
| Code block | No | Each `code` node on the page | `lang` from a closed list: `sh`, `bash`, `toml`, `ini`, `json`, `yaml`, `nginx`, `csv`, `xml`, `html`, `rust`, `ts`, `text`, `ascii`. `shell` fails, with `sh` as the fix. `bash` stays, for a script that uses bash syntax. A block with no `lang` fails. Another language (`console` and `cfg` are on the pages today) fails with no fix. Nextra reads `filename` and `copy` from the meta string, and `rehype-pretty-code` reads line highlights. The archetype allows those keys and no others. |
| Placeholder | No | A `<...>` token in a `code` or `inlineCode` node, other than a tag of the language of the block (HTML, XML) | The token is a name in the placeholder list. A token that is an alias in the list fails, with the name as the fix. |
| Troubleshooting | No | `heading`, depth 2, "Troubleshooting" | Each failure, then its fix. |
| Next steps | No | `heading`, depth 2, "Next steps", then a `list` of links | |

The placeholder list is one JSON file, `documentation/docs/lib/archetypes/placeholders.json`. Each entry has a `description`, a `name` (a placeholder such as `<ID>`) or a `flag` (`--write-changes`) or both, and optional `example` and `aliases` (the other names of the same value on the present pages). The linter uses the entries that have a `name`. The first entries come from the 21 rows of `data/csv/variables.csv`, and two of those rows are flags with no placeholder. Later, `operators/variables.mdx` renders its table from the JSON, and the CSV and its prebuild line go. This plan owns that change. `NEXTRA4-DEFERRAL-PLAN.md` also plans to replace the csv2md step, and after this change its replacement covers `isp-sheet.csv` alone. So the site and the linter read one list. Stage 8 chooses the name for each value that has more than one.

The linter lives in `documentation/docs/lib/archetypes/`, next to the retrieval modules (D12):

| File | Content |
|---|---|
| `runbook.json`, `tutorial.json` | The archetypes as data: the sections, the languages, the meta keys |
| `placeholders.json` | The placeholder list |
| `inline.mjs` | The import inliner |
| `rules/*.mjs` | One rule for each checked constraint: a plain function over the mdast |
| `lint.mjs` | The command: `pnpm lint:runbooks` in `documentation/docs`. It prints each fault and a count for each rule, and exits non-zero only on a rule that is an error |
| `*.test.ts`, `fixtures/` | Vitest fixture tests. `pnpm test` runs them, in the existing docs CI job (`ci-docs-test.yml`) |

The tutorial archetype uses the sections that the three tutorials have today, with one name for each. A section that is not in the archetype, such as "When to use the pool", is allowed. Only the required sections are checked by name:

| Section | Required | Constraint |
|---|---|---|
| What you'll learn | No | A `list`. |
| Prerequisites | No | As in a runbook. |
| Steps | Yes | Each step is a heading inside `Steps`, as in a runbook. Replace the `## Step N:` form. |
| How it works | No | Replaces "What's happening underneath" and "How it works internally". |
| Going further | No | One depth 2 heading for each extension. |
| What you've learned | No | A `list`. |
| Complete code | Yes | A citation of an example file by symbol, for example `<Snippet id="nym/sdk/rust/nym-sdk/examples/simple.rs#main" />`. The SDK examples compile in CI, so the tutorial then shows code that compiles, and a rename fails CI. |

The citation needs three things that the docs site does not have yet: the extractor (stage 2), a `<Snippet>` renderer on the docs site (P17), and one example file for each tutorial. Until then, the linter checks only that the section exists. The step fragments of a tutorial show parts of one function, so they are the open case of P21, and they stay as fences.

The linter is the runbook lint. It checks both archetypes, the runbook and the tutorial. It is a set of rules, one for each checked constraint. Each rule is a plain function over the mdast (D17). Six rules check the first version: step heading, steps per procedure, fence language, fence meta, placeholder, and required sections. The other constraints in the tables are guidance for authors, and a rule joins for one of them only with a count. The archetypes and the placeholder list are data files that the rules read. Each rule starts as a warning, and it fails CI when all pages pass it. The linter reads the mdast of a whole page, so it does not need the split, and it runs on the present docs pages at once (stage 8). Before the check, it puts each imported `.mdx` or `.md` file in place of its tag, at each level. `inlineMdxPartials` in `lib/retrieval/mdx.mjs` already resolves nested imports and breaks cycles, but it works on text and loses the file and line of each node. The linter reuses its path resolution and splices the mdast, so each fault names the file and line where it is. A TSX component on a how-to page hides its procedure from the mdast, so the linter lists it as unchecked. An unchecked component is not a fault and does not change the result of the page. D17 in the design has the details that an implementer needs: the data shapes, the token grammar, and how each rule finds its nodes.

Stage 8 moves the step headings to depth 3 and gives each one an explicit anchor. A link to a step then stays correct when a step is added above it, and the split later keeps the same anchors. Each link to an old numbered anchor changes in the same stage. The move to depth 3 also changes how the table of contents shows the steps, so stage 8 checks the result on one page first.

After the split, each step is its own prose snippet, with its heading on the first line and the anchor from the file name. The page puts the step tags inside `<Steps>`:

```mdx
# Upgrade a node with Nymvisor

<Context />

<Steps>
<CreateServiceFile />
<StartService />
</Steps>
```

So the page check has one exception: a page can hold a wrapper element that its archetype names, with only snippet tags inside it. Each site gives `Steps` to MDX in `mdx-components`, so the page needs no import that is not a snippet. `Steps` is the one wrapper in the first version. A step snippet must still read alone (stage 3), so its title says what the step does in the procedure, for example "Create the Nymvisor service file".

### Aspect: the question a snippet answers

Each record has an `aspect` from a closed list. The aspect separates why a thing exists from what it must do and from how the code does it. A flat set of chunks cannot make that separation.

| Aspect | Question | Content | Who sets it |
|---|---|---|---|
| `motivation` | Why does it exist? | Threat-model actors, vectors, properties, scenarios | The model adapter |
| `requirement` | What must hold? | Threat-model invariants, OpenSpec requirements | The adapters |
| `design` | How is it built? | The specification pages | The author, in frontmatter. `split` takes it from the heading. "Why it exists" gives `motivation`, and "Code" gives `implementation`. A heading with no rule gives `design` and a mark for review. |
| `implementation` | What is the code? | Code snippets | The extractor |
| `operation` | How does a person run it? | Operator guides and tutorials | The author |

The full rule table from heading to aspect: TBD(stage 2 owner). Two rules exist.

A prose snippet with no `aspect` fails the check. The aspect is a second axis next to the Diataxis type of a page. Diataxis says what form a text has, and the aspect says which question it answers.

The aspects and the `realises` edges form a chain. A code item realises a requirement, and a requirement realises a property. A property stands against an actor and a vector. A reader, or an agent, can go from a piece of code to the reason it exists.

### Authority

TBD(Max). The coherence agent has to know which records are commitments, and which source wins if two sources disagree. A proposed field has three values: `normative` (the threat model, the OpenSpec requirements), `descriptive` (a specification page), `illustrative` (a guide, an example). The field is not in the record type or the spec until Max makes this decision.

### Statements

This part follows the Cloudflare approach to engineering standards. Cloudflare extracts each RFC 2119 statement (MUST, SHOULD) from its standards into a record with a stable id. Its agents read the statements first and load the full document only when they need more. A statement starts as approved, and it blocks a merge only after someone promotes it to enforced.

Here, a statement is one normative unit. Two sources hold statements today:

| Source | One statement | Id |
|---|---|---|
| OpenSpec specs | One requirement block (`### Requirement: ...`), with its SHALL text and its scenarios | `<repo>/openspec/<spec>/<slug of the requirement title>` |
| The threat model | One cell of the comparison matrix: a configuration, a property, an actor, a verdict, and its condition | `<repo>/threat-model/claim/<scenario>/<property>/<actor>` |

A statement record has the aspect `requirement` for an OpenSpec requirement, and `motivation` for a threat-model claim. Two fields in `meta` follow Cloudflare:

| Field | Values | Who sets it |
|---|---|---|
| `level` | The strongest RFC 2119 keyword in the statement: `MUST`, `SHALL`, `SHOULD`, `MAY`. A threat-model claim has none. | The adapter, from the text |
| `status` | `approved` or `enforced`. Each statement starts as `approved`. | A person, with an explicit list of enforced ids in `.nym-codex/manifest.json`. Who decides: TBD(Max) |

An agent that reviews a change reports each unmet approved statement and does not block the merge. An unmet enforced statement with the level `MUST` or `SHALL` blocks the merge. This change stores `status` and builds no review agent.

Agents read statements first. A search with the aspect `requirement` returns short statement records. `get_section` on a statement id returns the full requirement with its scenarios, or the full scenario for a claim. This is the progressive disclosure that Cloudflare describes, and it needs no new MCP tool.

The id of an OpenSpec statement comes from the requirement title, so a title change changes the id. The adapter fails when two requirements in one spec give one slug. A requirement that holds sentences at two levels stays one statement in the first version.

### Links

The 363 Nym network pages hold 1,845 links that start with a codex route (`](/specifications/...)`). Of those links, 120 have an anchor. The pages also hold 11 links to an anchor on the same page. A codex route is wrong on a second site, and it has no meaning in the index.

A link in a snippet names a target by id and holds no route:

```md
See [the constructors](snippet:nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/constructors).
```

The site resolves each `snippet:` link when it builds the page. It looks up the target in a link map and writes the page route with the anchor. The push job writes the map for each repository, `snippets/<repo>/pages.json`, from the import lines of its pages: snippet id to page path and anchor. codex merges the maps of all repositories, so a link can reach a page of a second repository. A target that is on no page of that site fails the build. The record keeps the `snippet:` form, so the index holds a reference that is correct for each site. The tools also add a `relates-to` edge for each link.

The anchor of a section is the last part of its id (the file name), written in the Nextra custom id syntax: `## Constructors [#constructors]`. A page that imports snippets from two directories can get two sections with one anchor, for example two `code` sections. The page check fails on a duplicate anchor on one page, and the author renames one file. A page that `split` made from one original has no duplicates, because `split` numbers repeated headings. A change to the heading text keeps the anchor. A link to a snippet with no heading goes to the top of its page.

`split` converts each link in a page to the `snippet:` form. A link to a page that is not split yet stays a route until its target moves.

### Id rules

| Kind | Id | Example |
|---|---|---|
| Prose | `<repo>/<path under snippets/, no extension>` | `nym/specifications/nym-network/technical-details/discovery-urls/network-details-model/constructors` |
| Diagram | The same rule | `nym/specifications/nym-network/technical-details/discovery-urls/bootstrapping-and-overrides/loading-order-in-one-call-d1` |
| Model | `<repo>/<model name>/<entity type>/<id in the data>` | `nym/threat-model/actor/L2` |
| Code | `<repo>/<source file path>#<symbol>` | `nym/common/network-defaults/src/network.rs#NymNetworkDetails` |

The id of an authored snippet is its file path, so a `git mv` is the one way to change it, and git shows the rename. A change to the heading text does not change the id. A code id has a `#` and a file extension, and the other two kinds have neither, so the id spaces do not collide.

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
  title: string; // prose: the heading text, or the frontmatter title. diagram: the file name. code: the symbol
  lang?: string; // code: "rust". diagram: "mermaid"
  body: string; // prose: the text below the heading. diagram, code: the source text. model: the text projection
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
| Page URL and page title | The consumer finds them in the pages of the site it serves. The first page in path order that imports the snippet gives both. |

The present retrieval `Chunk` (`documentation/docs/lib/retrieval/types.ts`) has `id`, `source`, `title`, `heading`, `url`, `text`, `tokensEst`. The indexer builds one from a record and the pages of one site. It keeps `id` as it is. It takes `source` from `repo` and `kind` (`nym-docs`, `nym-code`, as the spike does), and `heading` from the snippet `title`. It takes `title` and `url` from the page that imports the snippet, and `text` from `body`. In `text`, the indexer replaces each citation tag with the title of its target. The indexer makes no cut of its own, apart from the split of an oversized body into `id~0`, `id~1` parts that it does today.


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

The list takes its ideas from the ArchiMate 3.2 relationship set and from the UML relationships. The two agree on a core: realisation, specialisation, dependency, association. The list does not copy either one. `observes` is ArchiMate's access relation for the one case this content needs. `mitigates` is ArchiMate's influence relation with a negative sign. `assessed-against` has no match in either one and comes from the threat-model data. Composition and flow are absent: a page groups the snippets, and the hops of a route are inside one scenario record.

The relations are a closed list in the `RefRel` type. A relation that is not on the list fails the check. A new relation is a change to the type, with a line in this plan that says what it means. With a free vocabulary, authors write `depends-on`, `dependsOn`, and `requires` for one meaning. A query on the graph then misses two of the three.

An edge to an id with no snippet fails the check. The source repository cannot check an edge to a snippet in a second repository, so the codex build checks it.

### The layout in codex

```
codex/
  snippets/<repo>/**/<name>.mdx   the prose snippet files, as authored
  snippets/<repo>/**/<name>.mmd   the diagram snippet files, as authored
  snippets/<repo>/prose.jsonl     one record on each line, sorted by id
  snippets/<repo>/code.jsonl
  snippets/<repo>/diagram.jsonl
  snippets/<repo>/model.jsonl
  snippets/<repo>/manifest.json   the source commit of the last push
  snippets/<repo>/pages.json      the link map: snippet id to page path and anchor
  content/<owned path>/**         the pages of the repository, with _meta.json and the _meta.js that the push writes
```

One record on each line gives a git diff of one line for each changed snippet. The snippet files are outside `content/`, so Nextra does not route them as pages.

## How each consumer reads snippets

| Consumer | Reads | Mechanism |
|---|---|---|
| codex site | Pages, snippet files, and records | Nextra builds the pages as they are, with the imported snippet files. One remark plugin (`remarkSnippet`) runs when Nextra compiles each file. It replaces each code citation with a fenced block and a link to its GitHub permalink, each diagram citation with a `mermaid` block, and each model citation with the component of its view. It also resolves each `snippet:` link. Nextra runs the plugins of a site before its own (`compile.js` in Nextra 4.6.1), so its Mermaid, heading id, and table of contents steps see the result. Turbopack passes loader options as data, so a function plugin under the Turbopack build is to verify (task 1.9). The fallback needs no plugin: a server component `<Snippet id>` reads the record and renders the code block, the Mermaid diagram, or the view, and an `a` override in `mdx-components.tsx` resolves `snippet:` links. Citations hold no headings, so the table of contents does not need a compile-time step for them. |
| A second site | Its own pages, the snippet files, and records | The same plugin. |
| Public index | The four `.jsonl` files | The indexer embeds one record as one chunk. |
| Private index | The same files from the private target | The same indexer with the private model. |
| View generator | Viewpoint files, records, and the link map | Writes the `.mdx` pages of each view into a generated directory under `content/` before `next build`. Hand-made pages and views use the same citations and links. |

The page and the snippets are real `.mdx` files, because Nextra needs files at compile time. From those files, Nextra builds the table of contents, the sidebar, the search index, and the syntax highlight. A component that renders content at request time has to rebuild each of those.

## Authored snippets: the costs and the answers

A person writes each prose unit as its own file. This form gives the third goal: a site is a set of pages that import one set of snippets. The form has two costs.

| Cost | Answer in the plan |
|---|---|
| The present pages are whole files: 363 Nym network pages in codex for this repository, about 840 pages across codex and the docs. | A migration tool, `split`, cuts each page at its `##` to `####` headings into snippet files and writes a page that imports them. The work is mechanical. There are two checks. The first check is a round trip: put the text of each snippet in place of its import, put each diagram back as a `mermaid` fence, and write each `snippet:` link back as its route. Then compare the result with the original page in codex with `git diff`. The anchors that `split` adds are the one expected difference. The second check compiles each snippet alone as MDX. The reason is that a cut through a JSX element gives broken parts that still join to a correct page. The 363 Nym network pages hold no JSX element today. Each page has its title as a `# ` line, and the title stays in the page. |
| A reviewer reads fragments, and the page file holds only the order. | A local command prints a page with the text of each snippet in place of its import (the same function as the round trip). A preview of the codex site for each PR in this repository is possible later, as a push to a preview branch in codex. The pilot does not build it. |

`split` makes the files. It does not make each section clear alone. Stage 3 of the plan does that work, with its own measure. A page that is split and not yet rewritten renders the same as before, so the site loses nothing in the meantime.

Some sources stay whole files because a different tool owns their format: `openspec/specs/*/spec.md` (the OpenSpec CLI), the TypeDoc output, the autodoc command output. An adapter derives records from those in a later phase. Do not split them by hand.

## Stages: how the data gets ready

The corpus is not in the target form today, and the plan does not need it to be. Most sections are not clear alone. The requirements do not name the threat-model property they serve. If a tool sets the aspect today, each section gets the same one. Each stage changes the data one step, and each has a measure that says how far the data is. Expect a low number at the start of a stage.

Each stage runs on the pilot section first (`technical-details/discovery-urls`, 5 pages), then on the other 358 Nym network pages. Stages 8 and 9 are the exceptions. Stage 8 runs on the operator guides and the Rust tutorials, and starts with one operator page. Stage 9 runs on the docs site in two steps, with a review between them.

| Stage | Change to the data | Who does it | Measure | Today |
|---|---|---|---|---|
| 0. Baseline | None. Label a set of "why" questions and a set of developer questions against the chunks that exist now. Record recall for plain search, a keyword hybrid, and expansion along the links that the pages already hold. | People write the labels | Recall at 6 for each question set | No labelled set exists |
| 1. Move and split | The pages move into `.nym-codex/` as snippet files and pages that import them. The text does not change. | The `split` tool | Round trip: each page renders as before. Per-snippet compile passes. | 0 of 363 pages |
| 2. Mechanical enrichment | Links become snippet ids. The aspect comes from the heading template. A tool checks each inline source path. Each code fence becomes a citation by symbol. A tool removes each section that holds only a list of pages, because the sidebar data gives that list. | Tools, with a person for the fences | Links resolved, of 1,845. Source paths that exist, of 1,085. Fences replaced, of 24. Sections with an aspect from a rule, of 2,199. | 0 in each |
| 3. Standalone pass | Each section gets a rewrite where needed, so that it is clear with no page around it. The result has a title that says what the section is about, and no reference to text above or below. | A model proposes, a person reviews | Share of sections that pass a standalone read by a person who did not write them. Target: 90%. | About 20% in an 11-page sample |
| 4. Threat model | Each entity becomes a record. Each cell of the comparison matrix gets an id as one claim. The integrity check runs in CI. The codex repository deletes its copy. | The adapter | Entities with a record. Integrity faults. | 2 copies that differ. 16 verdicts stated in one place and not the other. 1 missing assessment |
| 5. Requirement linkage | Each requirement states the property, actor, or vector it serves, or states that it serves none. Code items get `realises` edges to requirements. People write specs for the areas that no spec defines (P16). | People, with model proposals | Share of the 492 requirements with a stated link. Count of complete chains from code to motivation. | 0 requirements with a link. 1 weak chain found by hand |
| 6. Index and MCP | The indexer reads records. The context prefix, edge expansion, and `get_related` become active, each behind a switch. | Tools | Recall at 6 on the stage 0 sets, with each mechanism on and off | The stage 0 numbers |
| 7. First viewpoint | Write the first viewpoint (P27). Generate a view that matches one hand-made audit page, and compare the two. | Tools, with a person for the viewpoint | The share of the hand-made page that levels 0 and 1 make. The claims, specs, and links in one version and not the other. | No viewpoint. Each audit page is hand-made |
| 8. Archetypes | Write the runbook and tutorial archetypes, the placeholder list (`placeholders.json`, from `variables.csv`), and the lint rules. Each operator guide and tutorial states its Diataxis type. Fix each how-to and tutorial page until it passes: step headings at depth 3 with an explicit anchor and no number, each link to an old anchor updated, `sh` in place of `shell`, a language on each code block, and one name for each placeholder. | The linter, then people | How-to and tutorial pages that pass. Distinct placeholder tokens, of about 117 (the first lint run gives the exact count with the token grammar of D17). | No archetype. 253 numbered step headings |
| 9. Docs site split | The docs pages move into snippet files and pages that import them, in two steps (D18). Step 1: `network/` (52 pages) and the hand-written pages of `developers/` (83; the 59 TypeDoc pages stay whole). Then a review. Step 2: `operators/` (68), `apis/` (5), and the 2 root pages. The docs site gets the `@snippets` alias, the components that snippets use in `mdx-components`, and one compose function for `llms-full.txt`, the `.md` mirror, and the docs index. | The `split` tool, then people for the JSX cases | Pages split, of 135 in step 1 and 75 in step 2. Batches whose text outputs are identical before and after the split. JSX cases that need a new `split` rule. | 0 of 210 pages. The `.md` mirror has no partial text today |

| Stage | Owner | Target date | Result |
|---|---|---|---|
| 0. Baseline | TBD(Max) | TBD(Max) | Recall at 6, "why" set: plain TBD, hybrid TBD, link expansion TBD. Developer set: plain TBD, hybrid TBD, link expansion TBD. Standalone share: TBD |
| 1. Move and split | TBD(Max) | TBD(Max) | TBD |
| 2. Mechanical enrichment | TBD(Max) | TBD(Max) | TBD |
| 3. Standalone pass | TBD(Max) | TBD(Max) | Lint baseline, 2026-10-16 (`standalone.mjs`, task 4b.1): 1,476 of the 2,198 codex snippets (67%) have no fault the lint finds; 347 refer to text around them (234 of those say "this document"), 388 have a template title (Code 77, Technical details 64, Used by 61, Why it exists 60, Status 60, Documents in this section 53). The docs step 1 snippets: 332 of 394 (84%). The lint finds text that cannot read alone, so it is an upper bound on the share that a person passes; the human read of task 0.3 stays the measure |
| 4. Threat model | TBD(Max) | TBD(Max) | Adapter run 2026-10-16: 31 entities with a record (6 actors, 3 vectors, 2 properties, 14 scenarios, 6 invariants). Integrity: 0 errors, no verdict that contradicts another, 18 matrix verdicts not in the per-actor assessment (the plan counted 16), and `end-to-end` / L2 on the exception list. Claims (6b.9), views (6b.5), and the codex copy (6b.7) are still open |
| 5. Requirement linkage | TBD(Max) | TBD(Max) | Baseline from the coverage report, 2026-10-16: 0 of 492 requirements link to a threat-model entity, 0 have code that realises them, 0 complete chains, 0 of 42 specs with a link |
| 6. Index and MCP | TBD(Max) | TBD(Max) | TBD. A mechanism stays on at a gain of TBD(stage 6 owner) or more |
| 7. First viewpoint | TBD(Max) | TBD(Max) | TBD |
| 8. Archetypes | TBD(Max) | TBD(Max) | First run, 2026-10-16, with `--assume how-to` on the 68 operator pages and their imports: 0 pass. Faults: step-heading 978 (317 headings hold a number), steps-per-procedure 33, fence-lang 73, fence-meta 0, placeholder 353 (about 190 distinct tokens, from code blocks and inline code), required-sections 55. The 3 Rust tutorials with `--assume tutorial`: 0 pass; fence-lang 5, required-sections 3 (no `Steps`). |
| 9. Docs site split | TBD(Max) | TBD(Max) | Step 1, 2026-10-16: 64 of 135 pages split into 370 snippets; the text outputs (llms-full.txt, the .md mirror, each chunk of the index) are identical before and after; 394 of 394 snippets compile; 0 record errors. 71 pages wait, each with a reason (D18). The Turbopack build passes (273 static pages), and the 26,284 internal page links on the split pages all reach a built page. Review: TBD. Step 2: TBD |

Stages 0 and 4 do not depend on the split and can start at once. Stage 8 can start the lint of the operator guides at once, on the docs pages where they are now. The citation of the complete code of each tutorial needs the extractor, a renderer on the docs site, and an example file for each tutorial. Stage 9 starts after the pilot of stage 1. Its step 1 (`network/` and `developers/`) does not wait for stage 8, apart from the 16 pages that export scenario data, which wait for stage 4. Its step 2 (`operators/`) waits for stage 8, so that the operator guides have their runbook form before the split. Stage 7 needs the facets of P10 and is better after stages 3 and 5, but a first viewpoint over the present records shows the gaps early. Stage 5 is the largest piece of content work, and no tool can do it alone. A person who knows the system has to say why each requirement exists.

The measures also say when to change the plan. If stage 6 shows no gain from a mechanism, that mechanism stays off and the record keeps the data. If stage 3 stays far below its target after real effort, the page becomes the unit of authoring again. A tool then derives the records from headings. The record format is the same, so no consumer changes.

Stage 1 includes the work to build CI and the push job: the PR check with its excerpt comment, and the push to codex. Both run for each later stage.

The work after these stages covers `nym-vpn-client`, and the private target and the private repositories. It also covers the four deep-dive animations and more viewpoints. `nym-vpn-client` owns the 10 NymVPN threat diagrams that codex holds in `lib/privacy-model/nymvpn-diagrams.ts`.

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
| Numbered step headings in operator guides | 253, on 24 of 65 pages | The size of stage 8. 248 of them are inside `Steps`, which numbers them a second time. Each change of a heading changes its anchor, so 22 links change too. |
| Placeholder tokens in operator guides | About 117 distinct in code blocks, and at least six names for the public IPv4 address of a node | The placeholder list of stage 8, `placeholders.json`. Its first entries come from `data/csv/variables.csv`. |
| Rust tutorial code with a source file | 0 of 16 fences in 3 tutorials. No SDK example matches the complete code of the mixnet or the stream tutorial. | The complete code of each tutorial can cite an SDK example by symbol, after each tutorial has one. |

## Relation to the shared retrieval index

The shared index design (`add-shared-retrieval-index`, decision 10) moves the code chunker to tree-sitter. The extractor needs the same thing: a function that takes a file and returns its items with name, kind, and line range. Build that function once.

| Output | Items | Purpose |
|---|---|---|
| Code index | All items in the indexed roots | Broad code search. |
| Code snippets | The items that a prose snippet cites | Excerpts in pages, and a curated set for the index. |

Code snippets do not replace the broad code index. The indexer does not embed a code record as a chunk of its own. The broad index holds the same item already, and if the indexer adds a second copy, that copy competes with the item in the results. A code record gives a page its excerpt. The `cites` edge tells the index which items a person chose to explain, and the index can use that edge to rank results.

Stage 6 depends on the shared index change.

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
| P7 | The qualifier rules for a code id. Decided by task 2.3 (2026-10-16): a citation names a unique suffix of the item path, and a trait-impl method is `<Type as Trait>::method` (D5). Open: a form for `cfg` variants | TBD(task 2.3) for `cfg` | A citation names a `cfg` variant |
| P8 | The meaning of the relation `creates`, and one real edge that uses it. The relation is not on the list. | TBD(Max) | It joins the relation list |
| P9 | An authority field: whether it exists, and its values (proposed: normative, descriptive, illustrative) | TBD(Max) | Stage 4 |
| P10 | The facet vocabulary for selection next to `aspect`, as closed lists. Candidates: `audience`, `operator`, `area`, `product`, `audit-status` (from the hand-made audit blocks). The Diataxis type is decided (2026-10-16): `diataxis` takes `tutorial`, `how-to`, `reference`, or `explanation`. | TBD(Max) | Stage 7 |
| P11 | The second site. Decided 2026-10-16: the docs site of this repository, with the import alias and no push (stage 9, D18). Open: its owner, and how a site in another repository gets the snippet files (a push, or a package) | TBD(Max) | Stage 9 starts; a site in another repository imports snippets |
| P12 | Owner of `overview/`, `threat-model/`, and `prior-work/` in codex. No recommendation: they describe more than one product. | TBD(Max) | Those pages are split |
| P13 | The kind of write credential for codex (GitHub App or deploy key), and who creates it | TBD(infra) | Stage 1 CI |
| P14 | The CODEOWNERS names for the threat-model directory | TBD(Max) | Stage 4 |
| P15 | The exception list of the integrity check, with a reason for each entry (one known entry: `end-to-end` has no L2 assessment) | TBD(threat-model owner) | Stage 4 |
| P16 | The specs to write for the core mechanisms that no spec defines: the Sphinx packet format, client cover traffic, and the mix-node delay (see "Where the content stands") | TBD(stage 5 owner) | Stage 5 |
| P17 | The place of `remarkSnippet`, and how other repositories get the tools (a package, or a copy). Both sites run `remarkSnippet` (stage 9). Recommendation: `split`, the extractor, and `remarkSnippet` in this repository, and a copy of `remarkSnippet` in codex. Decide on a shared package when `nym-vpn-client` starts. | TBD(Max) | Stage 9 starts |
| P18 | Whether codex follows `develop` or release tags, and whether the push job commits to the default branch or opens a PR. Recommendation: follow `develop`, push a branch, and merge it when the codex build passes. | TBD(Max) | Stage 1 CI |
| P19 | The name and location of the private target, and its sync from codex | TBD(Max), not in this repository | A private repository adopts the flow |
| P20 | The gain in recall that keeps a retrieval mechanism on | TBD(stage 6 owner) | Stage 6 |
| P21 | Some fences show part of a function body: whether to cite the whole function, or add region markers to product code. Recommendation: cite the whole function, and add markers when a real case needs one. | TBD(Max) | Stage 2 |
| P22 | Decided 2026-10-16: `documentation/docs/.nym-codex/`, inside the docs package, because the docs Turbopack build reads no file outside it (see "The directory in a source repository"). The name stays `.nym-codex/`. Still open: whether a visible directory serves contributors better. The tools take the directory from one constant (`CODEX_DIR`), so a move is one rename. | Max | Done |
| P23 | Which models come after the threat model, and an owner for each. The motivation model is the next one. | TBD(Max) | Stage 4 ends |
| P24 | The entity types, the location, and the content owner of the motivation model | TBD(Max) | The motivation model starts |
| P25 | Who promotes a statement from `approved` to `enforced`, and the rule for it | TBD(Max) | A review agent uses `status` |
| P26 | The first-section `title` check compares the title with the `# ` line of the importing page. A second site with a different page title then cannot import that snippet. Whether the check stays, applies only to the pages of the source repository, or is removed | TBD(Max) | A second site imports a first-section snippet (P11) |
| P27 | The first viewpoint, its owner, and its target site. Recommendation: the auditor viewpoint in the private target, because the regeneration test of stage 7 needs the hand-made audit pages. | TBD(Max) | Stage 7 |
| P28 | Decided 2026-10-16: the sections are in "Archetypes: runbooks and tutorials". The languages are `sh`, `bash`, `toml`, `ini`, `json`, `yaml`, `nginx`, `csv`, `xml`, `html`, `rust`, `ts`, `text`, `ascii`. The placeholder list is `placeholders.json`, and `variables.mdx` renders from it. Open: who owns the list (TBD(Max)), and the name for each value with more than one (stage 8) | TBD(Max) | Stage 8 |

## Design choices

| Topic | Choice |
|---|---|
| Unit of authoring for prose | The snippet. A page is an MDX file that imports snippets. |
| What the source repository commits | Authored files only. The PR check posts excerpt changes as a comment, and the push job extracts code at the merged commit. This is a detail to look at again when the broad design is complete. |
| Sidebar order | `_meta.json` next to the pages. The push job writes `_meta.js`. |
| Models | A `model` kind for each typed data model. The threat model is the first. A motivation model follows. For the threat model, the docs copy in this repository is the one source, and codex mirrors it. |
| Edges | Typed, with a closed list of relations. ArchiMate and UML are the inspiration only. |
| Aspect | Each record must have this field. It has five values. |
| Statements | Taken from Cloudflare: one record for each normative unit, with `level` and `status` (`approved`, then `enforced`). Agents read statements before the full text. |
| Index | Context in the embedded text, edge expansion at query time, and labelled query sets. Stage 0 measures the baseline before any file moves. |
| Starting point | The present content does not meet the target form. The data changes in stages, and each stage has a measure. |
| Distribution | Push from each source repository to codex, as the codex README defines it. |
| Viewpoints | A viewpoint file defines a reader. A generator makes views from records, with deterministic selection first, template text second, and prompted text last, with provenance. |
| Guides | An archetype for each of two Diataxis types: runbook (how-to) and tutorial. The runbook lint checks each page against its archetype, before and after the split. |
| Docs site | The second site. Its pages stay in `documentation/docs/content/` and import snippets from `.nym-codex/snippets/`. A snippet takes its components from the `mdx-components` of each site. Split in two steps with a review between them. |

## Out of scope

- The private flow in detail. This plan states only the boundary rule.
- A docs page split before its step of stage 9. Step 2 starts only after the review of step 1.
- The embedding provider and the store. The shared index change owns both.

# Nym codex snippets: context

Status: discussion notes, 2026-10-13. This file gives the reasons for `NYM-CODEX-SNIPPETS-PLAN.md` and for the OpenSpec change `openspec/changes/add-codex-snippets/`. The plan says what to build. This file names the readers, the ideas behind the plan, the findings of an independent review, and the open points.

## Who reads the content

The content has three kinds of reader. The needs of the third kind make the plan as large as it is.

| Reader | What it asks | What it needs from the content |
|---|---|---|
| A person on a site | "How does this work?" | Pages in a sensible order. |
| A developer agent | "How do I send a message with the SDK?" | Good search, correct code excerpts, citations that do not move. |
| A sense-making agent | "Is this statement true? Is this feature a good idea, given what we know?" | Claims with ids, a way to tell which source wins, and a path from a piece of code to the reason it exists. |

A developer agent uses the present MCP server today, and that works. The plan must not make it worse. The sense-making agent does not exist yet. Because of that agent, the plan has a graph as well as a better index.

## The hypothesis

The hypothesis came from discussion, and no proof of it exists:

1. An embedding places a piece of text in a vector space by its words. It holds no data about how that text relates to other text.
2. A flat set of chunks gives an agent connectivity between tokens and none between ideas. So an agent can find a passage and still not know why the thing in it exists.
3. In the target form, each unit of content answers one question: why, what must hold, how it is built, or what the code is. If the corpus holds such units and typed edges connect them, retrieval can return an answer together with its reason.
4. A general system prompt does not give this separation. The structure has to be in the corpus.

Two points in the discussion stay open. The first is whether the structure gives better answers. Only a labelled test can show that. The second is how much of the structure a machine can write. A model can propose an aspect and edges for a few thousand snippets. A person has to review them, most of all for the threat model.

## How the structure reaches a vector

An embedding model reads one piece of text. An edge or an aspect changes a vector only if the text contains it. The plan uses the structure in two places:

| Place | Mechanism | Cost |
|---|---|---|
| In the embedded text | The indexer puts context before the body. Today the context is the page title and the heading. The plan adds the aspect and the titles of the linked requirement and motivation snippets. | A change to a linked title makes the vector stale. The cache key for an embedding must be a hash of the full embedded text. A hash of the body alone is not enough. |
| Beside the vector | The store holds `aspect`, `kind`, `repo`, and the edges in their own table. A query can filter on the aspect before the search and follow edges from each hit after it. | Plain database work. It uses no model. |

One record gives one vector, with the snippet id as the key. Prose, diagram, and model records go to the prose model. Code stays in the broad code index with the code model. The public store and the private store hold the same records with the same ids and edges. They differ in the model that embeds them. The search does not compare scores from two models.

## How it relates to the MCP server

The MCP server is the main reader of the records. The present tools keep their shape.

| Change | Effect on an agent |
|---|---|
| Ids change from heading slugs to file paths | An id stays valid across edits and sessions. |
| Each threat-model entity is a record | `get_section` on the id of actor L2 returns the one definition. |
| The source commit is in a manifest | An answer can name the commit of its source, and a code link is a permalink. |
| `aspect` and `expand` parameters on `search_docs` | "Requirements only", or "return the motivation with each hit". |
| One new tool, `get_related(id, rel?, direction?)` | Forward: from a design snippet to the threat it answers. Backward: from a code symbol to each snippet that cites it. |

Plain vector search cannot do the backward lookup. Before an agent changes a struct, it can ask which specifications and requirements name that struct.

The number of tools stays small on purpose. Each tool uses context in each agent session, and an agent chooses worse from a long list. Snippets as MCP resources with a `snippet:` URI are a possible later step, because the plan uses the same form for links.

## What the sense-making agent needs

The agent needs three things. The plan has the first as a task in the threat-model stage and the third as the coverage report. The second, an authority field, is not in the plan. It needs a decision on its values first.

1. **Claims as units.** The threat model has them already. One cell of the comparison matrix is one claim. A claim has a configuration, a property, an actor, a verdict, and the condition under which the verdict holds. Each cell needs an id, so that an agent cites the claim and not a paragraph.
2. **Authority.** If two sources disagree, the agent has to know which one wins. The threat model and the OpenSpec requirements are normative. A specification page describes, and a guide illustrates. The records have an `aspect` and no field for this.
3. **The right to say "no evidence".** An agent that judges must be able to answer that the corpus does not cover a question. That needs a coverage report. The report shows which requirements have an edge from code and which have none. A sparse graph that looks complete is worse than no graph.

The integrity check on the threat model is the most important item here. Today codex and the docs hold two different definitions of the primary adversary.

## Where the vocabulary comes from

The edge relations and the aspects use ideas from two standards and copy neither.

| Source | What the plan takes | What it leaves |
|---|---|---|
| ArchiMate 3.2 relationships | Realisation, specialisation, serving (as `depends-on`), access (as `observes`), influence with a negative sign (as `mitigates`), association (as `relates-to`) | Composition, aggregation, assignment, triggering, flow |
| ArchiMate motivation layer | The idea that drivers, goals, and requirements are separate from the elements that realise them, which gives the `aspect` field | The element types, about 60 of them |
| UML relationships | Agreement on the same core: realisation, generalisation, dependency, association | The whole-part forms |

A colleague proposed typed edges, with a plain id for a simple reference and an object with a kind for a specific one. The plan takes that and stores one object form in each record. It keeps the relations as a closed list, so that one meaning cannot have two spellings in the graph. His example relation `creates` is not on the list until it has a stated meaning and one real edge.

## What an independent review found

An independent reviewer looked for faults in the core idea and measured the corpus. Its verdict was to continue with changes.

| Finding | Evidence |
|---|---|
| Most sections do not stand alone. | The 363 Nym network pages follow templates: "Code" 77 times, "Technical details" 64, "Used by" 61, "Why it exists" 60, "Status" 60. About one section in five of an 11-page sample passes a standalone read. |
| No reuse exists today. | No identical paragraph between codex and the docs, no second site, no recipe that reuses a snippet. |
| The chain from code to motivation has almost no content. | 492 OpenSpec requirements, and none names a threat-model property, actor, or vector. No spec covers Sphinx, cover traffic, or mixing delay. A search by hand found one weak chain and two partial ones. |
| The plan has no place for `realises` edges. | Tools generate the code records and the OpenSpec records. Authored edges exist only in prose frontmatter. |
| The retrieval acceptance test cannot detect a failure. | `check-retrieval-scores.mjs` has no relevance labels and reads the docs site index, which the change does not alter. |
| `split` writes one aspect for all sections. | At least 197 sections under "Why it exists" and "Code" get the wrong aspect at creation. |
| Nothing checks the inline source paths. | 2,300 mentions of 1,085 distinct paths on 312 pages. The 24 Rust fences are the smaller drift problem. |

The review compared the design with known failures. Topic-based authoring (DITA) tends to give topics with generic titles and no context, and maps that drift from topics. In DITA projects, teams plan reuse and seldom use it. Knowledge graphs for retrieval tend to start with an ontology, get sparse hand-made edges, and have no evaluation. The design repeats part of both. It avoids nested inclusion of prose in prose, and it avoids line-range code citations.

## The position after the review

The review describes the corpus as it is today, and it is correct about that. The owner decided that the work starts from an unready corpus, and that this is no objection to the target. The target stays: authored snippets, the graph, the model kind, and the sense-making agent. The data changes in stages until it agrees with the target. The developer agent works today, and this change must not make it worse.

The plan uses the review's findings as the measures of its stages. These are the share of sections that pass a standalone read, the count of requirements with a stated link, and the count of resolved links. Each one starts low, and each one has a stage that increases it.

The review disagreed with the design in three places.

| Topic | The plan | The review | Result |
|---|---|---|---|
| Unit of authoring | The snippet. A page is a recipe. | The page. Add an explicit anchor to each heading and derive the records. A recipe can list derived ids, so a second site still works. | The snippet stays. The standalone share after stage 3 is the test, and derived records are the fallback if it fails. |
| Distribution | Each source repository pushes to codex. | codex pulls from the source repositories at build time, with no credential in source CI. | Push stays, as the codex README states it. |
| Threat model | A `model` kind with a record for each entity. | A shared module and wider projections are enough for about 30 entities. | The `model` kind stays, because the sense-making agent needs the entities as nodes. |

## Changes taken from the review

The plan and the spec hold six changes from the review. The first three are a cheap baseline experiment, a standalone read of the pilot section, and a named owner for the `realises` chain. The other three are the aspect from the heading template, a check of each inline source path, and labelled recall as the retrieval acceptance test. The stage table in the section "Stages: how the data gets ready" of `NYM-CODEX-SNIPPETS-PLAN.md` has the detail.

## Points where the design changes course

The review proposed these as reasons to stop. The plan uses the first two as stage measures with a stated fallback. This file gives the other three as the review gave them, for the team to judge:

- The cheap experiment shows no clear gain on "why" questions from the prefix or from edge expansion, or the keyword hybrid gives more. Then drop the aspect and the typed relations.
- More than 30% of the pilot sections fail a standalone read. Then drop authored snippets.
- No second site with a named owner and a page list exists before the split of all pages. Then do not do the split.
- Fewer than 20 real `realises` chains exist after one work week of authoring. Then drop the relation.
- A Rust engineer needs more than one file and one command to change a paragraph. Then stop and simplify.

# Nym codex snippets: context

Status: discussion notes, 2026-10-13. This file explains the reasoning behind `NYM-CODEX-SNIPPETS-PLAN.md` and the OpenSpec change `openspec/changes/add-codex-snippets/`. The plan says what to build. This file says who it is for, which ideas it rests on, what an independent review found, and what is not decided.

## Who reads the content

The content has three kinds of reader, and the third one sets the ambition of the plan.

| Reader | What it asks | What it needs from the content |
|---|---|---|
| A person on a site | "How does this work?" | Pages in a sensible order. |
| A developer agent | "How do I send a message with the SDK?" | Good search, correct code excerpts, citations that do not move. |
| A sense-making agent | "Is this statement true? Is this feature a good idea, given what we know?" | Claims with ids, a way to tell which source wins, and a path from a piece of code to the reason it exists. |

A developer agent uses the existing MCP server today, and it works. The plan must not make it worse. The sense-making agent does not exist yet. It is the reason the plan has a graph and not only a better index.

## The hypothesis

The working hypothesis came from discussion and is not proven:

1. An embedding places a piece of text in a vector space by its words. It knows nothing about how that text relates to other text.
2. A flat set of chunks gives an agent connectivity between tokens and none between ideas. So an agent can find a passage and still not know why the thing in it exists.
3. If the content is cut into units that each answer one question (why, what must hold, how it is built, what the code is), and the units are connected with typed edges, then retrieval can return an answer with its reason attached.
4. A general system prompt does not give this separation. The structure has to be in the corpus.

Two points in the discussion stay open. The first is whether the structure gives better answers, which only a labelled test can show. The second is how much of the structure a machine can write. A model can propose an aspect and edges for a few thousand snippets, and a person has to review them, most of all for the threat model.

## How the structure reaches a vector

An embedding model reads one piece of text. An edge or an aspect has no effect on a vector unless the text carries it. The plan uses the structure in two places:

| Place | Mechanism | Cost |
|---|---|---|
| In the embedded text | The indexer puts context in front of the body: page title and heading today, and in the plan also the aspect and the titles of the linked requirement and motivation snippets. | A change to a linked title makes the vector stale. The cache key for an embedding must be a hash of the full embedded text, and not of the body alone. |
| Beside the vector | The store holds `aspect`, `kind`, `repo`, and the edges in their own table. A query can filter on the aspect before the search and follow edges from each hit after it. | Plain database work. No model is involved. |

One record gives one vector, with the snippet id as the key. Prose, diagram, and model records go to the prose model. Code stays in the broad code index with the code model. The public store and the private store hold the same records with the same ids and edges, and differ in the model that embeds them. Scores from two models are not compared.

## How it relates to the MCP server

The MCP server is the main reader of the records. The existing tools keep their shape.

| Change | Effect on an agent |
|---|---|
| Ids are file paths and not heading slugs | An id stays valid across edits and sessions. |
| Each threat-model entity is a record | `get_section` on the id of actor L2 returns the one definition. |
| The source commit is in a manifest | An answer can name the commit it rests on, and a code link is a permalink. |
| `aspect` and `expand` parameters on `search_docs` | "Requirements only", or "return the motivation with each hit". |
| One new tool, `get_related(id, rel?, direction?)` | Forward: from a design snippet to the threat it answers. Backward: from a code symbol to each snippet that cites it. |

The backward lookup is the one that plain vector search cannot do. An agent that edits a struct can ask which specifications and requirements name it before it changes it.

The number of tools stays small on purpose. Each tool costs context in each agent session, and an agent chooses worse from a long list. Snippets as MCP resources with a `snippet:` URI are a possible later step, because the plan uses the same form for links.

## What the sense-making agent needs

The plan has the first as a task in the threat-model stage and the third as the coverage report. The second, an authority field, is not in the plan: it needs a decision on its values first.

1. **Claims as units.** The threat model has them already. One cell of the comparison matrix is one claim: a configuration, a property, an actor, a verdict, and the condition under which the verdict holds. Each cell needs an id, so that an agent cites the claim and not a paragraph.
2. **Authority.** When two sources disagree, the agent has to know which one wins. The threat model and the OpenSpec requirements are normative. A specification page describes. A guide illustrates. The records have an `aspect` and no field for this.
3. **The right to say "no evidence".** An agent that judges must be able to answer that the corpus does not cover a question. That needs a coverage report: which requirements have an edge from code, and which have none. A sparse graph that looks complete is worse than no graph.

The integrity check on the threat model matters most here. Today codex and the docs hold two definitions of the primary adversary that differ.

## Where the vocabulary comes from

The edge relations and the aspects take ideas from two standards and copy neither.

| Source | What the plan takes | What it leaves |
|---|---|---|
| ArchiMate 3.2 relationships | Realisation, specialisation, serving (as `depends-on`), access (as `observes`), influence with a negative sign (as `mitigates`), association (as `relates-to`) | Composition, aggregation, assignment, triggering, flow |
| ArchiMate motivation layer | The idea that drivers, goals, and requirements are separate from the elements that realise them, which gives the `aspect` field | The element types, about 60 of them |
| UML relationships | Agreement on the same core: realisation, generalisation, dependency, association | The whole-part forms |

A colleague proposed typed edges, with a plain id for a simple reference and an object with a kind for a specific one. The plan takes that, stores one object form in each record, and keeps the relations as a closed list so that two spellings of one meaning cannot split the graph. His example relation `creates` is not on the list until it has a stated meaning and one real edge.

## What an independent review found

A fresh reviewer attacked the core idea and measured the corpus. Its verdict was to proceed with changes.

| Finding | Evidence |
|---|---|
| Most sections do not stand alone. | The 363 Nym network pages follow templates: "Code" 77 times, "Technical details" 64, "Used by" 61, "Why it exists" 60, "Status" 60. About one section in five of an 11-page sample reads correctly with no page around it. |
| No reuse exists today. | No identical paragraph between codex and the docs, no second site, no recipe that reuses a snippet. |
| The chain from code to motivation has almost no content. | 492 OpenSpec requirements, and none names a threat-model property, actor, or vector. No spec covers Sphinx, cover traffic, or mixing delay. One weak chain and two partial ones were found by hand. |
| The plan gives `realises` edges no place to live. | Code records and OpenSpec records are generated. Authored edges exist only in prose frontmatter. |
| The retrieval acceptance test cannot detect a failure. | `check-retrieval-scores.mjs` has no relevance labels and reads the docs site index, which the change does not alter. |
| `split` writes one aspect for all sections. | At least 197 sections under "Why it exists" and "Code" would be wrong at creation. |
| Inline source paths are unchecked. | 2,300 mentions of 1,085 distinct paths on 312 pages. The 24 Rust fences are the smaller drift problem. |

The review compared the design with known failures. Topic-based authoring (DITA) tends to give topics with generic titles and no context, maps that drift from topics, and reuse that is planned and seldom used. Knowledge graphs for retrieval tend to start with an ontology, get sparse hand-made edges, and have no evaluation. The design repeats part of both. It avoids nested inclusion of prose in prose, and it avoids line-range code citations.

## The position after the review

The review describes the corpus as it is today, and it is correct about that. The owner's decision is that an unready corpus is the starting point and not an objection: the target stays (authored snippets, the graph, the model kind, the sense-making agent), and the data changes in stages until it meets the target. The developer agent already works and should only improve.

So the review's findings became the measures of the stages in the plan. The share of sections that read correctly alone, the count of requirements with a stated link, and the count of resolved links each start low and each have a stage that raises them.

The review disagreed with the design in three places.

| Topic | The plan | The review | Result |
|---|---|---|---|
| Unit of authoring | The snippet. A page is a recipe. | The page. Add an explicit anchor to each heading and derive the records. A recipe can list derived ids, so a second site still works. | The snippet stays. The standalone share after stage 3 is the test, and derived records are the way back if it fails. |
| Distribution | Each source repository pushes to codex. | codex pulls from the source repositories at build time, with no credential in source CI. | Push stays, as the codex README states it. |
| Threat model | A `model` kind with a record for each entity. | A shared module and wider projections are enough for about 30 entities. | The `model` kind stays: the sense-making agent needs the entities as nodes. |

## Changes taken from the review

These six are in the plan and the spec.

1. Run a cheap experiment first, in parallel with the pilot. Label 30 "why" questions against the chunks that exist today, and measure recall with a context prefix, with expansion along the 1,845 links that already connect the pages, and with a keyword hybrid. It moves no file and takes days.
2. Let the pilot section test authored snippets. A person who did not write a section reads it with no page around it. If more than 30% of sections fail, switch to derived records. The record format is the same in both cases.
3. Treat the `realises` chain as content work with a named owner: a file for edges on generated records, a statement in each requirement of the property it serves, and a coverage report.
4. Take the aspect from the heading template: "Why it exists" gives `motivation`, and "Code" gives `implementation`.
5. Check each inline source path in the pages against the repository.
6. Replace the retrieval acceptance test with labelled recall on the content that the change touches.

## Points where the design changes course

The review proposed these as reasons to stop. The plan uses the first two as stage measures with a stated way back. The other three are recorded here as the review gave them, for the team to judge:

- The cheap experiment shows no clear gain on "why" questions from the prefix or from edge expansion, or the keyword hybrid gives more. Then drop the aspect and the typed relations.
- More than 30% of the pilot sections fail a standalone read. Then drop authored snippets.
- No second site with a named owner and a page list exists before the split of all pages. Then do not split.
- Fewer than 20 real `realises` chains after one working week of authoring. Then drop the relation.
- A Rust engineer needs more than one file and one command to change a paragraph. Then stop and simplify.

# Nym codex snippets: context

This file gives the reasons for `NYM-CODEX-SNIPPETS-PLAN.md` and for the OpenSpec change `openspec/changes/add-codex-snippets/`. The plan says what to build. This file names the readers of the content, the hypothesis behind the design, the known risks, and the alternatives that the design does not take.

## Who reads the content

The content has three kinds of reader. The needs of the third kind make the plan as large as it is.

| Reader | What it asks | What it needs from the content |
|---|---|---|
| A person on a site | "How does this work?" | Pages in a sensible order. |
| A developer agent | "How do I send a message with the SDK?" | Good search, correct code excerpts, citations that do not move. |
| A sense-making agent | "Is this statement true? Is this feature a good idea, given what we know?" | Claims with ids, a way to tell which source wins, and a path from a piece of code to the reason it exists. |

Developer agents use the present MCP server, and it works. The plan must not make it worse. Because of the sense-making agent, the plan has a **graph** as well as a **better index**.

## The hypothesis

The design rests on a hypothesis. No proof of it exists yet:

1. An embedding places a piece of text in a vector space by its meaning. Two vectors that are near each other show that two texts are similar. They do not show that one text is the reason for the other, or which text depends on which.
2. A flat set of chunks gives an agent similarity and no typed relations. So an agent can find a passage and still not know why the thing in it exists.
3. In the target form, each unit of content answers one question: why, what must hold, how it is built, or what the code is. If the corpus holds such units and typed edges connect them, retrieval can return an answer together with its reason.
4. A general system prompt does not give this separation. The structure has to be in the corpus.

There is outside evidence for one part. In a test of contextual retrieval, Anthropic put a short context in front of each chunk before the embedding, and the share of failed top-20 retrievals fell by 35% (from 5.7% to 3.7%). With a keyword (BM25) index over the same text as well, it fell by 49% ([Anthropic, "Introducing Contextual Retrieval"](https://www.anthropic.com/news/contextual-retrieval)). That test used generated context, not typed edges, and a different corpus.

Two questions stay open. The first is whether the structure gives better answers, and only a labelled test can show that. The second is how much of the structure a machine can write. A model can propose an aspect and edges for a few thousand snippets. A person has to review them, most of all for the threat model and the motivation model.

## How the structure reaches a vector

The present models (`voyage-3-large` for prose, `voyage-code-3` for code) embed each chunk alone. An edge or an aspect changes a vector only if the text of the chunk contains it. The plan uses the structure in two places:

| Place | Mechanism | Cost |
|---|---|---|
| In the embedded text | The indexer puts context before the body. The present indexer uses the page title and the heading (`embedText` in `docs/lib/retrieval/embed.mjs`). The plan adds the aspect and the titles of the linked requirement and motivation snippets. | A change to a linked title makes the vector stale. The present cache key is already a hash of the full embedded text, with the model and the dimension. The plan keeps that key, so a changed title gives a new embedding. |
| Beside the vector | The index holds `aspect`, `kind`, `repo`, and the edges beside each vector. A query can filter on the aspect before the search and follow edges from each hit after it. | It uses no model. Today the index is a JSON file and the search is a linear scan (`docs/lib/retrieval/retrieval.ts`), so a filter is exact. The shared index plans Postgres with pgvector. There, a filter on an HNSW index can return fewer than k hits, unless the query turns on iterative index scans (pgvector 0.8 and later). |

One record gives one vector, with the snippet id as the key. The exception is a body over 2,400 characters: the chunker splits it into parts `id~0`, `id~1`, each with its own vector. Prose, diagram, and model records go to the prose model. Code stays in the broad code index with the code model. The private store holds a copy of each public record, with the same id and edges, and also the private records. The two stores differ in the model that embeds them. The search does not compare scores from two models.

A different model type can put document context in each vector. `voyage-context-3` reads all chunks of a document in one pass and gives one vector for each chunk ([Voyage, "Contextualized chunk embeddings"](https://docs.voyageai.com/docs/contextualized-chunk-embeddings)). Voyage reports that it retrieves better than contextual retrieval and late chunking on its own test sets ([MongoDB, "Introducing voyage-context-3"](https://www.mongodb.com/company/blog/product-release-announcements/voyage-context-3-focused-chunk-level-details-global-document-context)). These are vendor figures. Stage 6 can measure this model as one more arm against the labelled sets.

## How it relates to the MCP server

The MCP server is the main reader of the records. The present tools keep their shape.

| Change | Effect on an agent |
|---|---|
| Ids are file paths, not heading slugs | An id stays valid across edits and sessions. |
| Each model entity is a record | `get_section` on the id of actor L2 returns the one definition. |
| Statements are records | A search with the aspect `requirement` returns short statements first, and `get_section` returns the full text. |
| The source commit is in a manifest | An answer can name the commit of its source, and a code link is a permalink. |
| `aspect` and `expand` parameters on `search_docs` | "Requirements only", or "return the motivation with each hit". |
| One new tool, `get_related(id, rel?, direction?)` | Forward: from a design snippet to the threat it answers. Backward: from a code symbol to each snippet that cites it. |

Plain vector search cannot do the backward lookup. Before an agent changes a struct, it can ask which specifications and requirements name that struct.

The number of tools stays small on purpose. Each tool uses context in each agent session, and an agent chooses worse from a long list. Snippets as MCP resources with a `snippet:` URI are a possible later step, because the plan uses the same form for links.

## What the sense-making agent needs

1. **Claims as units.** The plan makes each OpenSpec requirement a statement, and each cell of the threat-model comparison matrix too. A statement has an id, a level, and a status. An agent then cites the claim and not a paragraph.
2. **Authority.** If two sources disagree, the agent has to know which one wins. The threat model and the OpenSpec requirements are normative. A specification page describes, and a guide illustrates. The plan holds this as an open placeholder (P9), because the values need agreement first.
3. **The right to say "no evidence".** An agent that judges must be able to answer that the corpus does not cover a question. The coverage report shows which requirements have an edge from code and which have none. A sparse graph that looks complete is worse than no graph.

The integrity check on the models matters most here. If two sites state the primary adversary in two ways, a sense-making agent gives answers that disagree with each other. This happens today: codex holds an older copy of the threat model, and the docs copy has changed since.

## Where the vocabulary comes from

The edge relations and the aspects use ideas from two standards and copy neither.

| Source | What the plan takes | What it leaves |
|---|---|---|
| ArchiMate 3.2 relationships | Realisation, specialisation, serving in the reverse direction (as `depends-on`: serving points from the provider, `depends-on` points from the consumer), access for read only (as `observes`: access also covers write), influence with a negative sign (as `mitigates`), association (as `relates-to`) | Composition, aggregation, assignment, triggering, flow |
| ArchiMate motivation layer | The idea that drivers, goals, and requirements are separate from the elements that realise them. This gives the `aspect` field and the motivation model. | Most of the ten motivation element types. The candidate types for the motivation model are stakeholder, driver, goal, and principle. |
| UML relationships | Agreement on the same core: realisation, generalisation, dependency, association | The whole-part forms |
| Cloudflare engineering standards ([Cloudflare blog](https://blog.cloudflare.com/engineering-standards-enforcement/)) | Normative statements with stable ids, a `level` from RFC 2119, and a status that goes from approved (agents flag, no block) to enforced (an unmet MUST blocks). Agents read statements before the full text. | Cloudflare extracts each SHOULD or MUST statement and uses those two levels. The plan makes one statement for each requirement block and also records SHALL and MAY. |

An author can write an edge as a plain id, which means `relates-to`, or as an object with an id and a relation. Each record stores the object form. The relations are a closed list, so that one meaning cannot have two spellings in the graph. A new relation joins the list when it has a stated meaning and one real edge.

## Known risks

The numbers below come from the 363 Nym network pages and the OpenSpec specs in this repository. The plan uses each one as the measure of a stage, so each number starts low and has a stage that raises it.

| Risk | Evidence | Answer in the plan |
|---|---|---|
| Most sections do not stand alone. | The pages follow templates: "Code" 77 times, "Technical details" 64, "Used by" 61, "Why it exists" 60, "Status" 60. About one section in five of an 11-page sample passes a standalone read. | Stage 3 rewrites sections, with a target share of 90%. |
| No content has a second use yet. | No identical paragraph between codex and the docs, no second site, no page that imports a snippet from a second page. | Reuse needs a second site with an owner (P11). |
| The chain from code to motivation has almost no content. | 492 OpenSpec requirements, and none names a threat-model property, actor, or vector. Specs mention Sphinx packets, cover traffic, and Poisson delays (for example `nym-network-monitor`, `sdk-service-provider-example`, `nym-swizzle`). No spec defines the Sphinx packet format or how the client sends cover traffic. | Stage 5 adds the links, with a named owner and a coverage report. |
| Generated records have no place for authored edges. | Tools generate the code records and the OpenSpec records, so they have no frontmatter. | `.nym-codex/edges.json` holds those edges. |
| No test shows if the right chunk is in the top results. | `check-retrieval-scores.mjs` compares on-topic queries with off-topic queries, so it shows when the gap between them narrows. It has no relevance labels. The query set of the shared-index spike (`QUERY-SET.md`) labels developer queries by the expected source area, not by chunk. | Stage 0 labels query sets at chunk level, and starts the developer set from the spike set. The acceptance test is recall of labelled chunks. |
| One default aspect is wrong for many sections. | At least 197 sections under "Why it exists" and "Code" need a different aspect. | `split` takes the aspect from the heading template and marks defaults for review. |
| Inline source paths drift. | 2,300 mentions of 1,085 distinct paths on 312 pages. | The record builder checks each path. |

Two known failure modes of similar approaches apply. Topic-based authoring (DITA) tends to give topics with generic titles and no context, and maps that drift from topics. Teams plan reuse and seldom use it. Knowledge graphs for retrieval tend to start with an ontology, get sparse hand-made edges, and have no evaluation. The design avoids nested inclusion of prose in prose and line-range code citations. Against the rest, it relies on the stage measures and the labelled test.

## Alternatives that the design does not take

| Topic | The design | The alternative | Why the design keeps its choice |
|---|---|---|---|
| Unit of authoring | The snippet. A page is an MDX file that imports snippets. | The page. Add an explicit anchor to each heading and derive the records. A second site can still list derived ids. | The target is a pool of content that any site can compose. The standalone share after stage 3 is the test, and derived records are the fallback. The record format is the same either way. |
| Page composition | A page imports snippet files with MDX `import`. Nextra compiles each import, and adds its headings to the table of contents of the page. | A JSON recipe for each page (a title and an ordered list of ids with a heading level), and a build step that writes the `.mdx` pages from recipes and records. | No build step and no generated pages. The page file is the page that a reviewer reads, and `next dev` shows an edit at once. The table of contents, search, and syntax highlight work as for any page. The cost is that a snippet has one heading level on each page, and a second site needs the snippet files and not only the records. |
| Distribution | Each source repository pushes to codex. | codex pulls from the source repositories at build time, with no credential in source CI. | The codex README defines codex as push-only. |
| Models | A `model` kind with a record for each entity. | A shared module and wider projections, with no records. | A sense-making agent needs the entities as nodes with ids and edges. |
| Committed code | The source repository commits no extracted code. The PR check posts excerpt changes as a comment. | Commit the extracted bodies and a lock file, and write them back from CI. | No second copy of source code in the tree, and no bot commits on Rust PRs. |

## Points where the design changes course

Six observations change the plan if they occur. The first two are stage measures, with a fallback. The other four are for the team to judge when they occur.

- The baseline experiment shows no clear gain on "why" questions from the context prefix or from edge expansion, or a keyword hybrid gives more. Then the aspect and the typed relations stay as data, and retrieval does not use them.
- More than 30% of the pilot sections fail a standalone read after stage 3. Then the page becomes the unit of authoring, and a tool derives the records from headings.
- No second site with a named owner and a page list exists before the split of all pages.
- Fewer than 20 real `realises` chains exist after one work week of authoring.
- A Rust engineer needs more than one file and one command to change a paragraph.
- A second site must use the records without the snippet files, or one snippet needs a different heading level on two pages. Then JSON recipes and a build step that writes pages return as the page format.

# Nym codex snippets: context

This file gives the reasons for `NYM-CODEX-SNIPPETS-PLAN.md` and for the OpenSpec change `openspec/changes/add-codex-snippets/`. The plan says what to build. This file names the readers of the content, the hypothesis behind the design, the known risks, and the alternatives that the design does not take.

## Who reads the content

The content has five kinds of reader. The needs of the last two make the plan as large as it is.

| Reader | What it asks | What it needs from the content |
|---|---|---|
| A person on a site | "How does this work?" | Pages in a sensible order. |
| A developer agent | "How do I send a message with the SDK?" | Good search, correct code excerpts, citations that do not move. |
| An operator, or an agent that helps one | "How do I upgrade my node?" | Steps in order, commands that run as written, and one name for each value to fill in. |
| The coherence agent | "Does this new idea support, contradict, or stand neutral to what the project has committed to, and what does accepting it cost?" | Commitments as statements with ids, a way to tell which records are commitments, and edges from each commitment to the reason behind it. |
| A defined stakeholder, through a generated view | An auditor: "Which requirements in this area have no code that realises them?" | Facets to select by, edges to follow, sections that read alone, and text that a tool can make with a minimum of prompts. |

Developer agents use the present MCP server, and it works. The plan must not make it worse. Because of the coherence agent, the plan has a **graph** as well as a **better index**.

## The hypothesis

The design rests on a hypothesis. No proof of it exists yet:

1. An embedding places a piece of text in a vector space by its meaning. Two vectors that are near each other show that two texts are similar. They do not show that one text is the reason for the other, or which text depends on which.
2. A flat set of chunks gives an agent similarity and no typed relations. So an agent can find a passage and still not know why the thing in it exists.
3. In the target form, each unit of content reads alone, and typed edges connect it to the reason behind it. Then retrieval can return an answer together with that reason.
4. A general system prompt does not give this separation. The structure has to be in the corpus.

There is outside evidence for one part. In a test of contextual retrieval, Anthropic put a short context in front of each chunk before the embedding, and the share of failed top-20 retrievals fell by 35% (from 5.7% to 3.7%). With a keyword (BM25) index over the same text as well, it fell by 49% ([Anthropic, "Introducing Contextual Retrieval"](https://www.anthropic.com/news/contextual-retrieval)). That test used generated context, not typed edges, and a different corpus.

Two questions stay open. The first is whether the structure gives better answers, and only a labelled test can show that. The second is how much of the structure a machine can write. A model can propose edges for a few thousand snippets. A person has to review them, most of all for the threat model and the motivation model.

## How the structure reaches a vector

The present models (`voyage-3-large` for prose, `voyage-code-3` for code) embed each chunk alone. An edge changes a vector only if the text of the chunk contains it. The plan uses the structure in two places:

| Place | Mechanism | Cost |
|---|---|---|
| In the embedded text | The indexer puts context before the body. The present indexer uses the page title and the heading (`embedText` in `docs/lib/retrieval/embed.mjs`). The plan adds the titles of the linked requirement and motivation snippets. | A change to a linked title makes the vector stale. The present cache key is already a hash of the full embedded text, with the model and the dimension. The plan keeps that key, so a changed title gives a new embedding. |
| Beside the vector | The index holds `kind`, `repo`, and the edges beside each vector. A query can filter on these fields before the search and follow edges from each hit after it. | It uses no model. Today the index is a JSON file and the search is a linear scan (`docs/lib/retrieval/retrieval.ts`), so a filter is exact. The shared index plans Postgres with pgvector. There, a filter on an HNSW index can return fewer than k hits, unless the query turns on iterative index scans (pgvector 0.8 and later). |

One record gives one vector, with the snippet id as the key. The exception is a body over 2,400 characters: the chunker splits it into parts `id~0`, `id~1`, each with its own vector. Prose, diagram, and model records go to the prose model. Code stays in the broad code index with the code model. The private store holds a copy of each public record, with the same id and edges, and also the private records. The two stores differ in the model that embeds them. The search does not compare scores from two models.

A different model type can put document context in each vector. `voyage-context-3` reads all chunks of a document in one pass and gives one vector for each chunk ([Voyage, "Contextualized chunk embeddings"](https://docs.voyageai.com/docs/contextualized-chunk-embeddings)). Voyage reports that it retrieves better than contextual retrieval and late chunking on its own test sets ([MongoDB, "Introducing voyage-context-3"](https://www.mongodb.com/company/blog/product-release-announcements/voyage-context-3-focused-chunk-level-details-global-document-context)). These are vendor figures. Stage 6 can measure this model as one more arm against the labelled sets.

## How it relates to the MCP server

The MCP server is the main reader of the records. The present tools keep their shape.

| Change | Effect on an agent |
|---|---|
| Ids are file paths, not heading slugs | An id stays valid across edits and sessions. |
| Each model entity is a record | `get_section` on the id of actor L2 returns the one definition. |
| Statements are records | A search returns short statements first, and `get_section` returns the full text. |
| The source commit is in a manifest | An answer can name the commit of its source, and a code link is a permalink. |
| `expand` parameter on `search_docs` | "Return the motivation with each hit". |
| One new tool, `get_related(id, rel?, direction?)` | Forward: from a design snippet to the threat it answers. Backward: from a code symbol to each snippet that cites it. |

Plain vector search cannot do the backward lookup. Before an agent changes a struct, it can ask which specifications and requirements name that struct.

The number of tools stays small on purpose. Each tool uses context in each agent session, and an agent chooses worse from a long list. Snippets as MCP resources with a `snippet:` URI are a possible later step, because the plan uses the same form for links.

## What the coherence agent needs from the corpus

The coherence agent is designed separately, and this plan does not build it. It takes a new statement (an idea, a feature proposal, a spec change), finds the commitments nearest to it, and judges each pair: supports, neutral, or contradicts. Its output is a verdict that cites the commitments. Coherence is not correctness: two commitments can agree and both be wrong, and the witness for a claim about code is the code.

Vectors find the candidate commitments and nothing more. A test on the Nym docs index embedded ten statements that restate a commitment and ten that contradict one, with the same nouns. The mean cosine to the corpus centroid was 0.744 for the first group and 0.735 for the second, and each contradiction scored higher than 95% of the real chunks. A contradiction is on-topic by construction, so similarity cannot separate it from a restatement. The judge does that, one pair at a time.

From the corpus, the agent needs four things:

1. **Commitments as units.** The plan makes each OpenSpec requirement a statement, and each cell of the threat-model comparison matrix too. Threat-model properties and the goals and principles of the motivation model are commitments as well. A judge can check an idea against "the network never learns which client talked to which service". It cannot check an idea against three paragraphs of motivation. A requirement block that holds more than one normative sentence is not one commitment. The first version keeps the block as the unit, and this is a known limit.
2. **Authority.** The agent has to know which records are commitments and which only describe or illustrate. The threat model and the OpenSpec requirements are normative. A specification page describes, and a guide illustrates. The plan holds this as an open placeholder (P9), because the values need agreement first. Statement `status` adds a second distinction: an enforced commitment is harder to drop than an approved one.
3. **Edges to the reason.** When an idea contradicts a requirement, the `realises` chain from that requirement to a property and a goal states what accepting the idea costs: "this weakens property P1 and the goal behind it". That is an answer a person can act on. A bare verdict is not.
4. **The right to say "no commitment covers this".** The coverage report shows where the graph is empty. A sparse graph that looks complete is worse than no graph.

The largest gap today is the one in "Where the content stands" in the plan. No spec defines the Sphinx packet format, client cover traffic, or the mix-node delay. So an idea such as "mix nodes should forward each packet at once" has no atomic commitment to contradict, only threat-model prose and specification pages.

The integrity check on the models matters here too. If two sites state the primary adversary in two ways, the coherence agent finds two commitments that disagree and cannot tell which one the project holds. This happens today: codex holds an older copy of the threat model, and the docs copy has changed since.

## Why viewpoints

A team can write pages for one reader by hand once. It cannot keep many such sets current by hand. Hand-made audit documentation shows both sides: a component that selects typed blocks by facet keeps part of each page current, and the hand-written rest goes stale. A viewpoint moves the rest of the page into data where it can: what to select, which edges to follow, and the template. Prompted text is the last resort, with its prompt in git and its inputs recorded.

The coherence agent is not a viewpoint. A viewpoint makes a fixed page from a fixed query. The coherence agent takes a new statement each time and returns a verdict. The two use the same records.

Three things decide how much of a view a tool can make: the facets on the records, the share of sections that read alone, and the density of the edges. Stages 3 and 5 raise the second and third. Stage 7 measures the result on one hand-made audit page.

## Why archetypes for guides

An operator acts on a guide one line at a time. A wrong command, or a value with two names on one page, can leave a node broken. The docs have no written rules for the form of a procedure. `STE-STYLE.md` covers explanation and reference pages, and a person applies it in review. The operator pages show the result: 253 step headings with a number in the text, three names for shell, and at least six names for the public IPv4 address of a node, three of them on one page. An archetype states the form as data on the syntax tree, so CI checks it.

The archetype also gives an agent a structure. If the steps are headings in one `Steps` element, the commands are fences with a known language, and the placeholders come from one list, an agent can answer "how do I upgrade my node?" with the steps in order. It can also ask the operator for each value by the description in the list. With free-form guides, the agent has to guess where a step ends.

The archetype checks the form of a page. It does not check that a command works. A runbook can pass the lint and hold a command that no longer works. A test run of the commands would check that, and this plan does not build one. For tutorials, the citation of the complete code by symbol gives part of that check: the example file compiles, and the tutorial shows that file.

## Where the vocabulary comes from

The edge relations use ideas from two standards and copy neither.

| Source | What the plan takes | What it leaves |
|---|---|---|
| ArchiMate 3.2 relationships | Realisation, specialisation, serving in the reverse direction (as `depends-on`: serving points from the provider, `depends-on` points from the consumer), access for read only (as `observes`: access also covers write), influence with a negative sign (as `mitigates`), association (as `relates-to`) | Composition, aggregation, assignment, triggering, flow |
| ArchiMate motivation layer | The idea that drivers, goals, and requirements are separate from the elements that realise them. This gives the motivation model. | Most of the ten motivation element types. The candidate types for the motivation model are stakeholder, driver, goal, and principle. |
| ISO/IEC/IEEE 42010 and ArchiMate viewpoints | Stakeholder, concerns, viewpoint, and view, as the way to define a reader and generate a page for them | Model kinds, correspondence rules, and the viewpoint catalogue |
| UML relationships | Agreement on the same core: realisation, generalisation, dependency, association | The whole-part forms |
| DITA 1.3 task topic ([OASIS, "DITA task topic"](https://docs.oasis-open.org/dita/dita/v1.3/os/part2-tech-content/archSpec/technicalContent/dita-task-topic.html)) | The sections of a task (`prereq`, `context`, `steps`, `tasktroubleshooting`, `postreq`) and a `step` whose `cmd` is one imperative sentence. The runbook archetype maps these to mdast nodes: `cmd` is the step heading. DITA allows one set of steps for each task, and a runbook page can hold more than one task. | The DITA section order (the runbook puts the context first), XML, specialisation, `substeps`, `choices`, `steps-unordered`, `stepresult`, and the topic reuse model (see the failure modes in "Known risks") |
| Cloudflare engineering standards ([Cloudflare blog](https://blog.cloudflare.com/engineering-standards-enforcement/)) | Normative statements with stable ids, a `level` from RFC 2119, and a status that goes from approved (agents flag, no block) to enforced (an unmet MUST blocks). Agents read statements before the full text. | Cloudflare extracts each SHOULD or MUST statement and uses those two levels. The plan makes one statement for each requirement block and also records SHALL and MAY. |

An author can write an edge as a plain id, which means `relates-to`, or as an object with an id and a relation. Each record stores the object form. The relations are a closed list, so that one meaning cannot have two spellings in the graph. A new relation joins the list when it has a stated meaning and one real edge.

## Known risks

The numbers below come from the 363 Nym network pages, the OpenSpec specs in this repository, and the operator guides in the docs. The plan uses each one as the measure of a stage, so each number starts low and has a stage that raises it.

| Risk | Evidence | Answer in the plan |
|---|---|---|
| Most sections do not stand alone. | The pages follow templates: "Code" 77 times, "Technical details" 64, "Used by" 61, "Why it exists" 60, "Status" 60. About one section in five of an 11-page sample passes a standalone read. | Stage 3 rewrites sections, with a target share of 90%. |
| No content has a second use yet. | No identical paragraph between codex and the docs, no second site, no page that imports a snippet from a second page. | Reuse needs a second site with an owner (P11). |
| The chain from code to motivation has almost no content. | 492 OpenSpec requirements, and none names a threat-model property, actor, or vector. Specs mention Sphinx packets, cover traffic, and Poisson delays (for example `nym-network-monitor`, `sdk-service-provider-example`, `nym-swizzle`). **No spec defines the Sphinx packet format, how the client sends cover traffic, or the delay that a mix node adds.** `nym-swizzle` specifies application-layer delays for wallets. | Stage 5 adds the links, with a named owner and a coverage report. The specs for the three core mechanisms come first (P16). |
| Generated records have no place for authored edges. | Tools generate the code records and the OpenSpec records, so they have no frontmatter. | `.nym-codex/edges.json` holds those edges. |
| No test shows if the right chunk is in the top results. | `check-retrieval-scores.mjs` compares on-topic queries with off-topic queries, so it shows when the gap between them narrows. It has no relevance labels. The query set of the shared-index spike (`QUERY-SET.md`) labels developer queries by the expected source area, not by chunk. | Stage 0 labels query sets at chunk level, and starts the developer set from the spike set. The acceptance test is recall of labelled chunks. |
| Inline source paths drift. | 2,300 mentions of 1,085 distinct paths on 312 pages. | The record builder checks each path. |
| Many guides fail the runbook lint at first. | 253 numbered step headings on 24 operator pages, 43 code blocks with no language, about 117 placeholder tokens. | Each rule starts as a warning. Stage 8 measures the passing pages, and the linter names each fault. |
| The linter passes a procedure that it never saw. | 30 `.mdx` files in `components/operators/` reach the pages: 20 by a direct import, 5 through other `.mdx` files, and 5 through TSX components, for example `RunTabs` in `nym-node/setup.mdx`. | The linter puts each imported `.mdx` file in place of its tag before the check, at each level. It reports each TSX component on a how-to page as unchecked. Those procedures move into `.mdx` files, as the four deep-dive animations move into data. |
| An archetype that is too strict makes authors work around it. | The operator guides hold tabs and accordions (`<Tabs` 19 times, `<AccordionTemplate` 156 times), and the archetype has no place for them yet. | Most sections are optional, and an optional section fixes only a name and a place. A required section or a rule joins the archetype only with a count from the present pages. |

Two known failure modes of similar approaches apply. Topic-based authoring (DITA) tends to give topics with generic titles and no context, and maps that drift from topics. Teams plan reuse and seldom use it. Knowledge graphs for retrieval tend to start with an ontology, get sparse hand-made edges, and have no evaluation. The design avoids nested inclusion of prose in prose and line-range code citations. Against the rest, it relies on the stage measures and the labelled test.

## Alternatives that the design does not take

| Topic | The design | The alternative | Why the design keeps its choice |
|---|---|---|---|
| Unit of authoring | The snippet. A page is an MDX file that imports snippets. | The page. Add an explicit anchor to each heading and derive the records. A second site can still list derived ids. | The target is a pool of content that any site can compose. The standalone share after stage 3 is the test, and derived records are the fallback. The record format is the same either way. |
| Page composition | A page imports snippet files with MDX `import`. Nextra compiles each import, and adds its headings to the table of contents of the page. | A JSON recipe for each page (a title and an ordered list of ids with a heading level), and a build step that writes the `.mdx` pages from recipes and records. | No build step and no generated pages. The page file is the page that a reviewer reads, and `next dev` shows an edit at once. The table of contents, search, and syntax highlight work as for any page. The cost is that a snippet has one heading level on each hand-made page, and a site needs the snippet files to import them. A generated view composes records and sets its own heading levels, so it has neither cost. |
| Distribution | Each source repository pushes to codex. | codex pulls from the source repositories at build time, with no credential in source CI. | The codex README defines codex as push-only. |
| Models | A `model` kind with a record for each entity. | A shared module and wider projections, with no records. | The coherence agent needs the entities as commitments with ids and edges. |
| Form of a guide | An archetype as data, checked by the runbook lint. | Rules for procedures in prose, applied in review. | No rule for the form of a guide exists today, and prose rules do not fail a build. |
| Committed code | The source repository commits no extracted code. The PR check posts excerpt changes as a comment. | Commit the extracted bodies and a lock file, and write them back from CI. | No second copy of source code in the tree, and no bot commits on Rust PRs. |

## Points where the design changes course

Eight observations change the plan if they occur. The first two are stage measures, with a fallback. The other six are for the team to judge when they occur.

- The baseline experiment shows no clear gain on "why" questions from the context prefix or from edge expansion, or a keyword hybrid gives more. Then the typed relations stay as data, and retrieval does not use them.
- More than 30% of the pilot sections fail a standalone read after stage 3. Then the page becomes the unit of authoring, and a tool derives the records from headings.
- No second site with a named owner and a page list exists before the split of all pages.
- Fewer than 20 real `realises` chains exist after one work week of authoring.
- A Rust engineer needs more than one file and one command to change a paragraph.
- The first viewpoint (stage 7) needs prompted text for most of the hand-made page that it reproduces. Then the records lack the facets, the standalone sections, or the edges that the viewpoint asks for, and that work comes before more viewpoints.
- After stage 8, more than one how-to page in five needs an exception to the runbook archetype. Then the archetype is wrong, and it changes before the split of the operator guides.
- The review after step 1 of stage 9 (`network/` and `developers/` of the docs site) shows that many pages need hand edits after `split`, or that a docs writer needs more than one file and one command to change a paragraph. Then step 2 does not start, and the docs pages go back to whole files with records derived from their headings.

# Design: codex snippets

## Context

Three things exist today, and no link exists between them:

- codex (`nymtech/codex`), a Nextra 4 site whose content is a hand copy. It builds with Turbopack. Its sidebar, table of contents, and search index come from the `.mdx` files under `content/` at compile time.
- The docs retrieval layer in `documentation/docs/lib/retrieval/`: a heading chunker for Markdown (`chunker.mjs`), a line-regex chunker for code (`code-chunker.mjs`), and the `Chunk` type.
- The `add-shared-retrieval-index` change, which moves the code chunker to tree-sitter (decision 10) and has each producer repository upsert its own shard.

The author measured codex on 2026-10-13: it has 363 Nym network pages. Those pages hold 83 `mermaid` blocks, 24 Rust fences on 12 pages, and 1,845 links to a codex route. The fences hold whole items, methods in `impl` and `trait` blocks, parts of a function body, and one formula.

`add-shared-retrieval-index` is on the branch `max/shared-mcp` and is not in this branch. Tasks 2.2 and section 6 depend on it.

## Decisions

### D1: A person authors each prose and diagram snippet as its own file, and a page imports them

A prose snippet is an MDX file (`.mdx`). Its body has at most one heading, on the first line, with an explicit anchor. A diagram snippet is a `.mmd` file. A page is an MDX file with a `# ` title line, an import line for each prose snippet, and one tag for each import.

The alternative keeps the page as the authored file and derives snippets from its headings. It needs no migration. This design rejects it because the goal is to build more than one site from one pool of content. With pages that import snippets, a second site is a second set of pages and copies no prose. If the tools derive snippets from the page, the page file stays the owner of the text, and each new site competes with it.

The cost is the migration and a harder review. D9 and D10 answer both.

A source whose format a different tool owns stays a whole file: `openspec/specs/*/spec.md`, TypeDoc output, autodoc output. An adapter derives records from those (tasks section 7). The record format is the same, so a consumer cannot tell an authored snippet from a derived one.

### D2: Four kinds, one record

`prose`, `code`, `diagram`, `model`. One `Snippet` interface holds all four (the plan has the full type). A record holds content and no fact about a site or a push. It has no URL, no page title, and no commit. A URL and a page title differ for each site that shows the snippet. If each record holds a commit, each push rewrites each line of each file. The indexer builds a `Chunk` from a record and the pages of the site it serves. `source` comes from `repo` and `kind`. `heading` comes from the snippet `title`. `title` and `url` come from the first page in path order that imports the snippet. `text` comes from `body`, with the title of its target in place of each citation tag.

The alternative, a type for each kind, gives a consumer four loaders for no gain. The fields that differ (`lang`, `lines`) are optional in one type.

### D3: The id of an authored snippet is its file path

| Kind | Id |
|---|---|
| Prose, diagram | `<repo>/<path under snippets/, no extension>` |
| Code | `<repo>/<source file path>#<symbol>`; a method is `Type::method` |

A path is an id that git already tracks: a rename shows as a rename, and a heading edit does not change the id. The import path in a page is `@snippets/` plus the id plus `.mdx`, so the page shows the id. The anchor of a section on a page is the last part of the id. The heading holds it in the Nextra custom id syntax (`## Title [#name]`), so a heading edit keeps each link to the section. This design rejects a slug of the heading as the id, because a wording change then breaks each page and each link that names the snippet.

A code id has `#` and a file extension. The other kinds have neither, so the id spaces do not collide. If a `.mdx` file and a `.mmd` file have one stem in one directory, they share an id, and the extractor fails. `~` stays reserved for the indexer's split of an oversized body.

### D4: A body has one heading at most, and imports and cites no prose

A prose body has at most one heading, on its first line. The heading holds an explicit anchor that is the file name. The heading text is the record `title`. A body with no heading, such as the first section of a page, has a `title` in its frontmatter. For the first section, that is the page title, and the check fails when it differs from the `# ` line of the page that imports it. A body with a heading has no `title` in its frontmatter, so the title has one source. A second heading fails the check. The record `body` is the text below the heading.

The heading level is part of the snippet, so a snippet has the same level on each page that imports it. A body can cite code and diagram snippets with `<Snippet id>`. It imports no file and cites no prose snippet. The page is the one place that orders prose, and the rule removes cycles.

One snippet is one retrieval chunk if its body does not pass 2,400 characters, the indexer's split size. A body that passes it gets a warning. 59 of the 2,198 sections in the Nym network pages pass it today. A snippet with a heading and no text is legal, for a heading that groups the sections below it.

A link names a snippet id (`[text](snippet:<id>)`) and no route. The `remarkSnippet` plugin resolves it to the route and anchor of the page that imports the target on the site it builds. It fails on a target that no page of that site lists. The record keeps the id form. This design rejects a route in the body. 1,845 links in the Nym network pages start with a codex route. That route is wrong on a second site and has no meaning in the index. The tools add a `relates-to` edge for each link.

Each prose and diagram snippet is in use in its own repository. Either a page imports it, or a prose snippet that a page imports cites it. A diagram is in use through the second path. This rule gives each snippet a page on codex, and no unused file stays in the repository.

### D4b: A `model` kind for typed data models. The threat model is the first, and a diagram is a view of a scenario

A model is a named set of typed entities with references by id. The threat model is the first model and not the only one. A motivation model, which states the stakeholders, drivers, goals, and principles behind the network, is the next one (placeholders P23 and P24). Each model has entity types, typed data, an adapter that writes records, integrity rules, and views. Edges between models use the one relation list, so a threat-model property can `realises` a goal of the motivation model. This design does not give each model its own record kind: the record shape is the same, and `meta.data` holds the part that differs.

The rest of this decision describes the threat model.

The threat model is typed data in `documentation/docs/lib/privacy-model/`: actors, vectors, properties, scenarios, invariants, with references by id. Each entity becomes one record of kind `model`. The record builder evaluates the TypeScript modules with `lib/retrieval/ts-data.mjs`. It writes a text projection into `body`, the evaluated entity into `meta.data`, and the references in the data into `refs`. The id comes from the id in the data: `<repo>/threat-model/<entity type>/<id>`.

An animated network diagram renders from the `paths` or `topology` of a scenario. The same scenario holds the verdict matrix and the per-actor assessment. This design rejects a record for the diagram alone. Such a record separates the picture from the claims, and a reader searches for the claims. A citation names the view, `<Snippet id view="diagram">`, and `remarkSnippet` maps each view to the existing component.

The source stays TypeScript. This design rejects a move to JSON files. The type checker covers the modules today, and three topology builders in `topologies.ts` are functions. A measurement shows that all 14 scenarios serialise to JSON after evaluation with no loss.

The projection extends `lib/retrieval/projections.mjs`, which already projects the actors, vectors, properties, and the generic scenarios. It adds the route hops, the worked examples (their scenarios, L1 actors, and invariants), and the comparison matrix. The record builder replaces the regex in `mdx.mjs` that finds one generic scenario on each page.

This repository holds the one source. codex holds an older copy of the data and of some view components. codex took them from the docs before later updates. codex deletes `lib/privacy-model/threat-model.ts` and renders from `model.jsonl`. Both sites use the docs view components as one set. The older codex `NetworkDiagram` takes a `topology`, and codex deletes it. A small server component reads the record by id with `node:fs`. It passes `meta.data` to the client view, so the compiled page does not hold the data.

An integrity check runs with the record builder. It has three rules. Each referenced id exists. Each scenario has an assessment for each spine actor or is on an exception list in the adapter. A verdict has one source. A measurement today shows that no verdict contradicts another. The matrix has 16 verdicts that are absent from the per-actor assessment, and `end-to-end` has no L2 assessment. The last one goes on the exception list with a note. This change does not alter the model content.

A scenario projection can pass 2,400 characters (6 of 14 do as JSON). The indexer splits it as it splits any oversized body. This design keeps one record for each scenario, because the id that pages cite is the scenario.

The four deep-dive animations (`PacketAnatomy`, `MixnetDeepDive`, `DvpnCoverTraffic`, `SurbDirectory`) hold their content in component code. Each one gets a typed data module first, then joins the model as an entity type.

### D4c: Edges are typed, and the relations are a closed list

`refs` is a list of `{ id, rel }`. The tools write `cites` for each citation in a body, `relates-to` for each `snippet:` link, and the model relations from the data. An author adds edges in the frontmatter of a prose snippet, as a plain id (which means `relates-to`) or as an object with a relation. The record builder normalises each entry to the object form, removes duplicates, and sorts by id then relation.

The first list is `cites`, `relates-to`, `depends-on`, `realises`, `specialises`, `observes`, `mitigates`, `assessed-against`. It takes its ideas from the ArchiMate 3.2 relationship set and the UML relationships and copies neither. The list leaves out composition and flow: a page groups snippets, and route hops are inside one scenario record. A relation joins the list when it has a stated meaning and one real edge.

The list of relations is the `RefRel` type. This design rejects an open vocabulary. Two spellings for one meaning split the graph, and no tool can tell a new relation from a typo. The name of the field is `rel`, because `kind` is the snippet kind in the same record.

A code record and an adapter record have no frontmatter, so their authored edges are in `.nym-codex/edges.json` as `{ from, to, rel }`. Without this file, an author has no place to write a `realises` edge from code to a requirement.

The tools write a coverage report. For each requirement, the report shows if it has an edge to a property, actor, or vector. It also shows if a code item has an edge to that requirement. A consumer that judges a statement needs to know where the graph is empty.

An edge to an id with no snippet fails the check in the source repository. The codex build checks an edge to a second repository, because all records are present there.

### D4d: Each snippet has an aspect

`aspect` is one of `motivation`, `requirement`, `design`, `implementation`, `operation`. It is required. The tools set it where the kind decides it. The extractor writes `implementation`. The threat-model adapter writes `motivation` (and `requirement` for an invariant). The OpenSpec adapter writes `requirement`. An author sets it in the frontmatter of a prose snippet. `split` takes it from the heading template of the Nym network pages: "Why it exists" gives `motivation`, and "Code" gives `implementation`. If no rule matches, `split` writes `design` with `aspect_review: true`, so a default is visible as a default.

The idea comes from the ArchiMate motivation layer, which keeps drivers, goals, and requirements apart from the elements that realise them. This design does not take its element types. About 60 types for enterprise modelling is more than this content needs. Five values are enough to ask "why" and "how" as separate questions.

This design rejects a free tag list for the same reason as a free relation list. A value outside the list fails the check.

### D4e: The index uses the aspect and the edges in two ways, and a labelled query set judges them

The present embedding models read the text of one chunk alone, so an edge has an effect on a vector only if the text holds it. Two mechanisms use the structure:

1. Context in the embedded text. The chunker puts the page title and the heading in front of each chunk today. The indexer adds the aspect and the titles of the snippets that this one `realises` or is `assessed-against`.
2. Edge expansion at query time. After the vector search, the server follows `realises`, `mitigates`, and `assessed-against` edges from each hit and returns the linked requirement and motivation snippets with it.

This design does not assume that either mechanism helps. Task 0.1 labels a set of "why" questions and a set of developer questions against the chunks that exist before any file moves. Task 0.2 records the baseline. Task 6.5 measures each mechanism on and off against the same sets. The measure is recall of labelled chunks. A cosine score does not work as the measure, because a prefix changes cosine scores by itself.

The cache key of an embedding stays as it is today: the model, the dimension, and the hash of the full embedded text (`embedChunks` in `lib/retrieval/embed.mjs`). The record `hash` covers the body alone, and the prefix also depends on the titles of linked snippets. So the indexer must not use the record `hash` as the cache key, or a stale vector stays after a linked title changes.

### D5: Code is found by symbol with tree-sitter

A citation names a file and a symbol. The extractor parses the file and returns the item with that name, with its doc comments and attributes.

- This design rejects line ranges: an edit above the range moves it, which is the drift this change removes.
- This design rejects region markers for now: they put documentation concerns into product code. Some fences show part of a function body. Those cite the whole function.
- This design rejects the line-regex chunker: it finds column-0 items, and some fences are methods.

The parser is one module with one function: file text and language in, a list of `{ name, qualifiedName, kind, startLine, endLine }` out. The shared index code chunker calls the same function for all items. The extractor calls it and filters to the cited ones.

Ambiguity is an error. If a symbol matches two items in one file, the extractor fails and lists the candidates. The author then qualifies the citation.

The qualifier is part of the id and of the body file name. So the code id form is provisional until task 2.3 fixes these cases. The first case is a module path in the name. The second is two trait impls with one method name (`Display::fmt` and `Debug::fmt` on one type). The third is `cfg` variants of one item (`network.rs` has six `#[cfg(feature = "env")]` items). The fourth is the `const`, `static`, `type`, and `impl` kinds. The fifth is an item that a macro generates. Such an item is not in the syntax tree, so no citation can name it as an item.

### D6: A citation is the request to extract

The manifest holds the repository name and its GitHub URL, and no list of symbols. The extractor scans each prose snippet for code citations and extracts what it finds. If the manifest holds a second list of symbols, a person must keep that list in agreement with the prose.

A citation that does not resolve fails the run with the snippet path and line. This is the drift check. The same rule covers a page that imports a file that does not exist, and a page that holds prose of its own.

### D7: The source repository commits authored files and no generated output

```
.nym-codex/
  manifest.json
  edges.json
  snippets/**/<name>.mdx | <name>.mmd     authored
  pages/**/<page>.mdx, _meta.json         authored
```

The PR check runs the extractor and posts one comment on the PR. The comment shows each code excerpt that changes, as a diff, and the pages that show it. A reviewer sees the excerpt change in the PR that changes the code. The check fails on an error, such as a citation that does not resolve. The push job extracts the bodies again at the merged commit, and `code.jsonl` in codex keeps their history.

The alternative is to commit the extracted bodies and a lock file, and to have the PR check write both back to the branch. This design does not take it, for three reasons. Committed bodies are a second copy of source code in the tree. Each Rust PR that touches a cited item gets a bot commit. A suffix such as `.txt`, which keeps tools from reading the copies as source files, also removes the syntax highlight, which is the reason to show the diff. The comment gives the same review signal with no copy. This is a detail to look at again when the broad design is complete.

`generate-code-index.mjs` walks the `ROOTS` list alone and cannot reach `.nym-codex/`. The shared index `collect.mjs` reaches it through its plain Markdown walk (`docs.kind: 'markdown'`, `roots: ['.']`). Without an exclusion, that walk reads each prose snippet as a page. The `EXCLUDE` and `DOCS_EXCLUDE` patterns of `collect.mjs` gain `.nym-codex` (task 2.8).

The indexer does not embed a code record as a chunk. If the indexer embeds a code record, that copy competes with the item that the broad code index already holds. The `cites` edge marks the item as one that a person explains.

The source repository does not commit full records. The authored files are the source for prose and diagrams, and a committed record is a second copy. The push job writes the records into codex.

No committed file and no record holds the source commit. The push job writes it once, in `snippets/<repo>/manifest.json` in codex.

### D8: The push replaces the repository's paths in full

The push job runs the extractor at the merged commit, builds the records, and writes the commit to `snippets/<repo>/manifest.json`. It replaces `snippets/<repo>/` in codex with the snippet files and the records. It replaces each page directory under `content/` that the manifest lists (for this repository, `content/specifications/nym-network/`) with the pages, writes a `_meta.js` next to each `_meta.json`, and sets `lastUpdated` in each page (D9). Git computes the diff. An append-only push leaves deleted and renamed snippets behind.

Records in codex are JSON Lines: `prose.jsonl`, `code.jsonl`, `diagram.jsonl`, `model.jsonl`, one record on each line, sorted by id. A changed snippet is a one-line diff, and `JSON.parse` on each line is the whole loader. Limit: one file holds all prose for a repository, a few megabytes at the present size. Split by top-level directory if a file passes about 20 MB.

The job pushes a branch in codex and merges it when the codex build passes. An import or a citation that codex cannot resolve fails that build, so a broken push does not reach the default branch.

Three rules cover the failure cases. The job runs one at a time (a concurrency group), so two merges cannot interleave. It reads the commit in the codex manifest. It stops if that commit is not an ancestor of the commit it pushes. So the content of an older commit cannot replace the content of a newer one. The PR check in this repository runs the same record build, page check, link resolution, and per-snippet compile as the codex build. So a fault shows before the merge.

A rename or a delete here can break a page of a second site, and this repository cannot see that page. The codex build finds it. An alias list for moved ids is a follow-up for the time when a second site exists.

### D9: Nextra builds the pages as they are, and one remark plugin renders citations

codex builds each page from its file. A page imports snippet files with the alias `@snippets`, which points to `snippets/` in codex. codex sets the alias for Turbopack (`turbopack.resolveAlias`) and for webpack, because it builds with both. Nextra 4.6.1 compiles any imported `.mdx` file with its own loader under both bundlers. It adds the headings of each imported file to the table of contents of the page (`remark-headings`). The search index (Pagefind) reads the built HTML. So codex needs no step that writes pages, and no generated directory. The snippet files are outside `content/`, so Nextra does not route them as pages.

One remark plugin in codex, `remarkSnippet`, runs when Nextra compiles each page and each snippet. It replaces a code citation with a fenced block and a caption that links to the GitHub permalink (manifest commit, `path`, `lines`). It replaces a diagram citation with a `mermaid` block, and a model citation with the component of its view. It resolves each `snippet:` link from the link map, and fails on a target that is not in the map. The push job writes the map for each repository (`snippets/<repo>/pages.json`: snippet id to page path and anchor) from the import lines of its pages. codex merges the maps of all repositories. The merged map is also the data for the check of edges to a second repository (task 6c.3). Nextra puts the remark plugins of a site before its own (`compile.js`). So its Mermaid step, its custom heading id step, and its table of contents see the output of `remarkSnippet`. Turbopack passes loader options as data, and the default codex build uses Turbopack. So a function plugin under that build is to verify (task 1.9). The fallback needs no plugin: a server component `<Snippet id>` reads the record and renders the code block, the Mermaid diagram, or the view, and an `a` override in `mdx-components.tsx` resolves `snippet:` links. Citations hold no headings, so the table of contents does not need a compile-time step for them.

The page frontmatter keeps the SEO fields of the docs pages (`title`, `description`, `schemaType`, `section`, `lastUpdated`), which `app/(docs)/seo.ts` reads on both sites, and a page can add more. Nextra reads metadata from the page file alone, so snippet frontmatter does not reach the page metadata. An edit to a snippet does not change the page file, so the push job sets `lastUpdated` in the pushed copy of each page to the newest commit date of the page file and the snippet files it imports. It changes no other frontmatter field, and the source repository commits no change. Without this step, `lastUpdated` and the JSON-LD `dateModified` go stale.

The page check fails on a page that holds content other than frontmatter, one `# ` line, import lines, and tags of those imports. Content with no snippet has no id and does not reach the index. The check also fails on two snippets with one anchor on one page. The anchor is the last part of the id, and a page that imports snippets from two directories can repeat it.

The design rejects a JSON recipe for each page and a build step that writes `.mdx` pages from recipes and records. That step adds a generated directory, a watch mode for `next dev`, and a second form of each page that a reviewer must print to read. Imports give the same pages with none of these. The cost is that a snippet has the same heading level on each page, and a second site needs the snippet files, not only the records. The context file names the points where recipes return.

No codex code reads `content/` as text: `lib/mdx-data.ts` and `lib/summary/` read `data/`. So no codex consumer has to follow the imports.

### D10: A tool does the migration, and a round trip checks it

`split` takes an existing page and writes the snippet files and a page that imports them. It cuts at `##` to `####` (the cut `chunker.mjs` makes). It writes the content before the first heading as a snippet with no heading and with a frontmatter `title` from the page title. It moves each `mermaid` block into a `.mmd` file and puts a citation in its place. It names each file from the `github-slugger` slug of its heading, and writes that slug as the explicit anchor of the heading. It keeps a heading with no text below it as a snippet with a heading alone. The heading split in `chunker.mjs` drops those, so `split` cannot call it unchanged. Last, `split` converts each link to a codex route into a `snippet:` link.

There are two checks. The first is a round trip: put the text of each snippet in place of its import, write the result over the page that codex has in git, and run `git diff`. The anchors that `split` adds are the one expected difference. An empty diff, apart from whitespace, proves the split lost nothing. The second compiles each snippet body alone with Nextra's `compileMdx`. The round trip cannot see a cut through a JSX element, because the broken parts join back to a correct page. The compile can. Rust fences stay in place through this step. D5 replaces them after.

The 363 Nym network pages have no JSX element, no `import` outside a code fence, no heading below level 4, and no frontmatter. Each page has its title as a `# ` line, and the line stays in the page. `split` copies the frontmatter of a page unchanged, so a docs page keeps its SEO fields. So the heading cut is safe for the pilot. A page with a heading inside `<Steps>` or `<Tabs>` needs a rule before `split` runs on the docs site content. The rule is to keep the whole element in one snippet.

`split` does not change a section so that a reader can understand it without the rest of its page. A lint rule for backward references and relative links is a follow-up, and people fix sections over time. A split page that nobody has fixed yet renders as it did before.

### D11: The target decides visibility

A snippet has no visibility field. A public repository pushes to codex. A private repository pushes to the private target and holds no codex credential. The private target takes codex in full. This design rejects a flag on each record, because one wrong value publishes private content. The two-store decision of the shared index already makes the boundary physical.

### D12: The tools are `.mjs` next to the retrieval modules

`split` and the extractor reuse `mdx.mjs` (frontmatter) and the fence-aware heading scan from `chunker.mjs`. They are plain `.mjs` with types in `types.ts`, as the retrieval modules are. They stay in this repository for the pilot. `remarkSnippet` lives in codex, because codex runs it at build time. A shared package is a decision for the second source repository.

### D13: The corpus reaches the target form in stages, and each stage has a measure

The corpus does not meet the rules of this design today. A reader can understand about one section in five without the rest of its page. A default gives each section the same aspect, no OpenSpec requirement names a threat-model entity, and 1,845 links are routes. These facts are an argument for the page as the unit of authoring, and the context file lists that alternative.

The decision is to keep the target and change the data in stages. The stages are baseline, move and split, mechanical enrichment, standalone pass, threat model, requirement linkage, index and MCP. The plan has the table. A stage does not wait for the corpus to be clean: the tools report a missing edge as missing.

The rules that the corpus cannot meet at the start are warnings with a count. Each becomes an error when its stage ends: the standalone rule after stage 3, the link that each requirement must have after stage 5. If a rule is an error from day one, it blocks the first push.

The measures give two points where the design changes. If a retrieval mechanism shows no gain in stage 6, it stays off. If the standalone share stays far below 90% after stage 3, the page becomes the authored unit again. The tools then derive the records from headings, with no change to a consumer.

### D14: Inline source paths are checked

The Nym network pages name 1,085 distinct source paths in 2,300 places. The record builder checks that each path in inline code exists in the repository and reports each one that does not. This is a warning until stage 2 ends, then an error. A path check is weaker than a symbol citation and covers far more text.

### D15: Statements follow the Cloudflare model

A statement is one normative unit with a stable id: an OpenSpec requirement block, or one cell of the threat-model comparison matrix. Its record carries `level` (the strongest RFC 2119 keyword) and `status` (`approved` or `enforced`) in `meta`. Each statement starts as `approved`. A person promotes one to `enforced` with an explicit list in `.nym-codex/manifest.json`. This follows the Cloudflare engineering-standards system, which extracts MUST and SHOULD statements with stable slugs and blocks merges only on enforced ones.

The unit is the requirement block and not each sentence. An OpenSpec requirement block already has a title, which gives a stable slug, and scenarios that test it. A split at each sentence needs an id for each sentence. A new sentence then moves the ids of the sentences after it.

Agents read statements before full text: a search with the aspect `requirement` returns statement records, and `get_section` returns the full requirement. This needs no new MCP tool.

This change stores `status` and builds no agent that reviews code against statements.

### Placeholders

The plan has the full table (P1 to P20). These are the ones that change this design when an owner fills them.

| Placeholder | Where it lands |
|---|---|
| TBD(task 2.3): the qualifier rules for a code id | D3 and D5. The code id form and the body file names are provisional until then. |
| TBD(stage 2 owner): the rule table from heading to aspect | D4d. Two rules exist. |
| TBD(Max): the relation `creates` | D4c, the `RefRel` list. |
| TBD(Max): an authority field and its values | D2, a new field on the record, and a new requirement in the spec. |
| TBD(threat-model owner): the exception list of the integrity check | D4b. The list has one known entry. |
| TBD(infra): the kind of write credential for codex | D8. |
| TBD(Max): codex follows `develop` or release tags | D8. The design assumes `develop`. |
| TBD(stage 6 owner): the gain in recall that keeps a mechanism on | D4e. |
| TBD(stage 0 owner): the baseline numbers | D4e and D13. |

## Risks

| Risk | Mitigation |
|---|---|
| A push credential for codex in this repository's CI | Scope it to the codex repository alone. codex CI rejects a push that touches a path outside the pusher's two directories. |
| About 2,600 authored files in a hidden directory are hard to find | The tools take the directory as an argument. A move to a visible directory is one rename. |
| Split sections are hard to read alone, and a second site shows that | Stage 3 rewrites them, with a measured share. The codex pages render as before in the meantime. |
| Stage 5 needs people who know why each requirement exists, and it has no owner | Name an owner before stage 5 starts. The coverage report shows the gap until then. |
| A reviewer sees fragments | The page file shows the order and the ids. A local command prints a page with the text of each snippet in place of its import. A codex preview for each PR is a follow-up. |
| tree-sitter grammars add a dependency and WASM files | The shared index needs the same ones. Add them once. |
| Content that is not in codex today enters this repository | The split takes the pages that codex holds and takes content from no other source. If codex is not public at that time, this repository shows the pages first. |
| A change to the threat model alters a claim that many pages use | CODEOWNERS on the model directory, the integrity check, and the PR comment, which lists each entity whose record changes. |

## Verification

- Parser unit tests on fixture files: whole item, method, trait method, missing symbol, ambiguous symbol, doc comment and attribute grouping.
- The round trip and the per-snippet compile of D10 on the pilot section, then on all Nym network pages.
- For each replaced Rust fence, a comparison of the extracted body with the fence. List each difference in the PR as a stale hand copy or an extractor fault.
- The record build is stable: two runs on one commit give identical bytes.
- codex builds from the pushed paths, and an import or a citation with a wrong id fails that build.
- For the index (tasks section 6): recall at 6 on the labelled developer set is not lower than the baseline of task 0.2.

The agent sandbox has no network and no git, so the user runs `pnpm install`, the tests, the codex build, and each push.

## Follow-ups (not this change)

- `nym-vpn-client` as a source, with its 210 `file.rs:NN` citations.
- The private target and the private repositories.
- The docs site content in the same form.
- Pages that select snippets by metadata, and the metadata vocabulary.
- The missing L2 assessment in the `end-to-end` scenario, and the 16 verdicts that the per-actor assessments do not state.
- The 10 NymVPN threat diagrams in `codex/lib/privacy-model/nymvpn-diagrams.ts`. `nym-vpn-client` owns them. Each states its claim in a code comment today, so the claim needs a data field before a tool can project it.

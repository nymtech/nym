# Design: codex snippets

## Context

Three things exist today and none of them is connected to the others:

- codex (`nymtech/codex`), a Nextra 4 site whose content is a hand copy. It builds with Turbopack. Its sidebar, table of contents, and search index come from the `.mdx` files under `content/` at compile time.
- The docs retrieval layer in `documentation/docs/lib/retrieval/`: a heading chunker for Markdown (`chunker.mjs`), a line-regex chunker for code (`code-chunker.mjs`), and the `Chunk` type.
- The `add-shared-retrieval-index` change, which moves the code chunker to tree-sitter (decision 10) and has each producer repository upsert its own shard.

Measured in codex on 2026-10-13: 363 Nym network pages, with 83 `mermaid` blocks, 24 Rust fences on 12 pages, and 1,845 links to a codex route. The fences hold whole items, methods in `impl` and `trait` blocks, parts of a function body, and one formula.

`add-shared-retrieval-index` is on the branch `max/shared-mcp` and is not in this branch. Tasks 2.2 and section 6 depend on it.

## Decisions

### D1: A person authors each prose and diagram snippet as its own file, and a page is a recipe

A prose snippet is a Markdown file with a `title` in frontmatter and a body with no heading. A diagram snippet is a `.mmd` file. A page is a JSON recipe: a title and an ordered list of `{ id, level }` sections.

The alternative keeps the page as the authored file and derives snippets from its headings. It needs no migration. It was rejected because the goal is to build more than one site from one pool of content: with a recipe as the page, a second site is a second set of recipes, and no prose is copied. With derived snippets the page file would stay the owner of order and grouping, and each new site would compete with it.

The cost is the migration and a harder review. D9 and D10 answer both.

A source whose format a different tool owns stays a whole file: `openspec/specs/*/spec.md`, TypeDoc output, autodoc output. An adapter derives records from those (tasks section 7). The record format is the same, so a consumer cannot tell an authored snippet from a derived one.

### D2: Four kinds, one record

`prose`, `code`, `diagram`, `model`. One `Snippet` interface holds all four (the plan has the full type). A record holds content and no fact about a site or a push. It has no URL, no page title, and no commit: a URL and a page title differ for each site that shows the snippet, and a commit in each record would rewrite each line of each file on each push. The indexer builds a `Chunk` from a record and the recipes of the site it serves: `source` from `repo` and `kind`, `heading` from the snippet `title`, `title` and `url` from the first recipe in path order that lists the snippet, `text` from `body` with each citation tag replaced by the title of its target.

The alternative, a type for each kind, gives a consumer four loaders for no gain. The fields that differ (`lang`, `lines`) are optional in one type.

### D3: The id of an authored snippet is its file path

| Kind | Id |
|---|---|
| Prose, diagram | `<repo>/<path under snippets/, no extension>` |
| Code | `<repo>/<source file path>#<symbol>`; a method is `Type::method` |

A path is an id that git already tracks: a rename shows as a rename, and a `title` edit does not change the id. The anchor of a section on a page is the last part of the id, written with the Nextra custom id syntax (`## Title [#name]`), so a `title` edit keeps each link to the section. A slug of the heading was rejected because a wording change would break each recipe that lists the snippet.

A code id has `#` and a file extension. The other kinds have neither, so the id spaces do not collide. A `.md` and a `.mmd` file with one stem in one directory would share an id, and the extractor fails on that. `~` stays reserved for the indexer's split of an oversized body.

### D4: A body has no heading and cites no prose

The recipe sets the heading level, so one snippet can sit at different depths on different pages. A body can cite code and diagram snippets with `<Snippet id>`. It cannot cite a prose snippet: the recipe is the one place that orders prose, and the rule removes cycles.

One snippet is one retrieval chunk, unless the body passes 2,400 characters, the indexer's split size. Such a body gets a warning; 59 of the 2,198 sections in the Nym network pages pass it today. An empty body is legal, for a heading that groups the sections below it.

A link names a snippet id (`[text](snippet:<id>)`) and no route. Materialise resolves it to the route and anchor of the page that lists the target on the site it builds, and fails on a target that no page of that site lists. The record keeps the id form. A route in the body was rejected: 1,845 links in the Nym network pages start with a codex route, which is wrong on a second site and has no meaning in the index. The tools add a `relates-to` edge for each link.

Each prose and diagram snippet is in use in its own repository: a recipe lists it, or a prose snippet that a recipe lists cites it. A diagram is in use through the second path. That gives each snippet a page on codex, and stops dead files from collecting.

### D4b: The threat model is a `model` kind, and a diagram is a view of a scenario

The threat model is typed data in `documentation/docs/lib/privacy-model/`: actors, vectors, properties, scenarios, invariants, with references by id. Each entity becomes one record of kind `model`. The record builder evaluates the TypeScript modules with `lib/retrieval/ts-data.mjs`, writes a text projection into `body`, the evaluated entity into `meta.data`, and the references in the data into `refs`. The id comes from the id in the data: `<repo>/threat-model/<entity type>/<id>`.

An animated network diagram renders from the `paths` or `topology` of a scenario. The same scenario holds the verdict matrix and the per-actor assessment. A record for the diagram alone was rejected: it would separate the picture from the claims, and the claims are what a reader searches for. A citation names the view, `<Snippet id view="diagram">`, and materialise maps each view to the existing component.

The source stays TypeScript. A move to JSON files was rejected: the type checker covers the modules today, and three topology builders in `topologies.ts` are functions. Measured: all 14 scenarios serialise to JSON after evaluation with no loss.

The projection extends `lib/retrieval/projections.mjs`, which already projects the actors, vectors, properties, and the generic scenarios. It adds the route hops, the worked examples (their scenarios, L1 actors, and invariants), and the comparison matrix. The record builder replaces the regex in `mdx.mjs` that finds one generic scenario on each page.

This repository holds the one source. codex deletes `lib/privacy-model/threat-model.ts`, which differs from the docs copy today, and renders from `model.jsonl`.

An integrity check runs with the record builder: each referenced id exists, each scenario has an assessment for each spine actor or is on an exception list in the adapter, and a verdict has one source. Measured today: no verdict contradicts another, 16 verdicts are in the matrix and absent from the per-actor assessment, and `end-to-end` has no L2 assessment. The last one goes on the exception list with a note; this change does not alter the model content.

A scenario projection can pass 2,400 characters (6 of 14 do as JSON). The indexer splits it as it splits any oversized body. One record for each scenario is kept, because the id that pages cite is the scenario.

The four deep-dive animations (`PacketAnatomy`, `MixnetDeepDive`, `DvpnCoverTraffic`, `SurbDirectory`) hold their content in component code. Each one gets a typed data module first, then joins the model as an entity type.

### D4c: Edges are typed, and the relations are a closed list

`refs` is a list of `{ id, rel }`. The tools write `cites` for each citation in a body, `relates-to` for each `snippet:` link, and the model relations from the data. An author adds edges in the frontmatter of a prose snippet, as a plain id (which means `relates-to`) or as an object with a relation. The record builder normalises each entry to the object form, removes duplicates, and sorts by id then relation.

The first list is `cites`, `relates-to`, `depends-on`, `realises`, `specialises`, `observes`, `mitigates`, `assessed-against`. It takes its ideas from the ArchiMate 3.2 relationship set and the UML relationships and copies neither. Composition and flow were left out: a recipe groups snippets, and route hops are inside one scenario record. A relation joins the list when it has a stated meaning and one real edge.

The list of relations is the `RefRel` type. An open vocabulary was rejected: two spellings for one meaning split the graph, and no tool can tell a new relation from a typing error. The field is named `rel`, because `kind` is the snippet kind in the same record.

A code record and an adapter record have no frontmatter, so their authored edges live in `.nym-codex/edges.json` as `{ from, to, rel }`. Without this file a `realises` edge from code to a requirement has no place to be written.

The tools write a coverage report: for each requirement, whether it has an edge to a property, actor, or vector, and whether a code item has an edge to it. A consumer that judges a statement needs to know where the graph is empty.

An edge to an id with no snippet fails the check in the source repository. An edge to a second repository is checked in the codex build, where all records are present.

### D4d: Each snippet has an aspect

`aspect` is one of `motivation`, `requirement`, `design`, `implementation`, `operation`. It is required. The tools set it where the kind decides it: the extractor writes `implementation`, the threat-model adapter writes `motivation` (and `requirement` for an invariant), the OpenSpec adapter writes `requirement`. An author sets it in the frontmatter of a prose snippet. `split` takes it from the heading template of the Nym network pages ("Why it exists" gives `motivation`, "Code" gives `implementation`) and writes `design` with `aspect_review: true` where no rule matches, so a default is visible as a default.

The idea comes from the ArchiMate motivation layer, which keeps drivers, goals, and requirements apart from the elements that realise them. Its element types were not taken: about 60 types for enterprise modelling is more than this content needs, and five values are enough to ask "why" and "how" as separate questions.

A free tag list was rejected for the same reason as a free relation list. A value outside the list fails the check.

### D4e: The index uses the aspect and the edges in two ways, and a labelled query set judges them

An embedding model reads the text of one chunk, so an edge has no effect on a vector unless the text carries it. Two mechanisms use the structure:

1. Context in the embedded text. The chunker puts the page title and the heading in front of each chunk today. The indexer adds the aspect and the titles of the snippets that this one `realises` or is `assessed-against`.
2. Edge expansion at query time. After the vector search, the server follows `realises`, `mitigates`, and `assessed-against` edges from each hit and returns the linked requirement and motivation snippets with it.

Neither is assumed to help. Task 0.1 labels a set of "why" questions and a set of developer questions against the chunks that exist before any file moves, and task 0.2 records the baseline. Task 6.5 measures each mechanism on and off against the same sets. A cosine score is not the measure: a prefix moves cosine scores directly, so the measure is recall of labelled chunks.

The cache key of an embedding is the hash of the full embedded text. The record `hash` covers the body alone, and the prefix also depends on the titles of linked snippets, so a body hash would leave a stale vector after a linked title changes.

### D5: Code is found by symbol with tree-sitter

A citation names a file and a symbol. The extractor parses the file and returns the item with that name, with its doc comments and attributes.

- Line ranges were rejected: an edit above the range moves it, which is the drift this change removes.
- Region markers were rejected for now: they put documentation concerns into product code. Some fences show part of a function body. Those cite the whole function.
- The line-regex chunker was rejected: it finds column-0 items, and some fences are methods.

The parser is one module with one function: file text and language in, a list of `{ name, qualifiedName, kind, startLine, endLine }` out. The shared index code chunker calls the same function for all items. The extractor calls it and filters to the cited ones.

Ambiguity is an error. If a symbol matches two items in one file, the extractor fails and lists the candidates. The author then qualifies the citation.

The qualifier is part of the id and of the body file name, so the code id form is provisional until task 2.3 fixes these cases: a module path in the name; two trait impls with one method name (`Display::fmt` and `Debug::fmt` on one type); `cfg` variants of one item (`network.rs` has six `#[cfg(feature = "env")]` items); the `const`, `static`, `type`, and `impl` kinds; and an item that a macro generates, which is not in the syntax tree and so cannot be cited as an item.

### D6: A citation is the request to extract

The manifest holds the repository name and its GitHub URL, and no list of symbols. The extractor scans each prose snippet for code citations and extracts what it finds. A second list of symbols would be a thing to keep in step with the prose.

A citation that does not resolve fails the run with the snippet path and line. This is the drift check. The same rule covers a recipe that lists an id with no snippet.

### D7: The source repository commits code bodies and a lock file

```
.nym-codex/
  manifest.json
  snippets/**/<name>.md | <name>.mmd      authored
  pages/**/<page>.json, _meta.json        authored
  code/<file path>/<symbol>.<ext>.txt     generated; "::" in a symbol becomes "." in the file name
  snippets.lock                           generated; "<sha256, 12 hex>  <id>", sorted by id
```

Code bodies are committed so that a reviewer sees the excerpt change in the PR that changes the code. The PR check writes the regenerated files back to the branch, as `ci-docs-autodoc.yml` does, so a contributor who edits Rust code runs no Node tool. The check fails on an error, such as a citation that does not resolve. Extraction in the push job alone, with no committed output, is simpler and was the reviewer's proposal; it was rejected because the excerpt change would then be absent from the source PR.

A lock line holds the first 12 hex characters of the record `hash`.

A body file is a copy of source code inside the repository. The `.txt` suffix keeps it out of each tool that selects source files by extension. `generate-code-index.mjs` walks the `ROOTS` list alone and cannot reach `.nym-codex/`. The shared index `collect.mjs` can, in two ways: its code walk from the repository root (`NYM_CODE_SCOPE=full`), and its plain Markdown walk (`docs.kind: 'markdown'`, `roots: ['.']`), which would read each prose snippet as a page. Its `EXCLUDE` and `DOCS_EXCLUDE` patterns gain `.nym-codex` (task 2.8).

The indexer does not embed a code record as a chunk. The broad code index holds the item already, and a second copy would compete with it. The `cites` edge marks the item as one that a person explains.

Full records are not committed in the source repository. The authored files are the source for prose and diagrams, and a record would be a second copy. The push job writes the records into codex.

No committed file and no record holds the source commit. The push job writes it once, in `snippets/<repo>/manifest.json` in codex.

### D8: The push replaces the repository's two directories in full

The push job runs the extractor at the merged commit, builds the records, writes the commit to `snippets/<repo>/manifest.json`, and replaces `snippets/<repo>/` and `pages/<repo>/` in codex. Git computes the diff. An append-only push would leave deleted and renamed snippets behind.

Records in codex are JSON Lines: `prose.jsonl`, `code.jsonl`, `diagram.jsonl`, `model.jsonl`, one record on each line, sorted by id. A changed snippet is a one-line diff, and `JSON.parse` on each line is the whole loader. Ceiling: one file holds all prose for a repository, a few megabytes at the present size. Split by top-level directory if a file passes about 20 MB.

The job pushes a branch in codex and merges it when the codex build passes. A recipe that codex cannot resolve fails that build, so a broken push does not reach the default branch.

Three rules cover the failure cases. The job runs one at a time (a concurrency group), so two merges cannot interleave. It reads the commit in the codex manifest and stops if that commit is not an ancestor of the one it pushes, so an older run cannot land last. The PR check in this repository runs the same record build, link resolution, and per-snippet compile as the codex build, so a fault shows before the merge and not after it.

A rename or a delete here can break the recipe of a second site, and this repository cannot see that recipe. The codex build finds it. An alias list for moved ids is a follow-up for the time when a second site exists.

### D9: The consumer materialises recipes to `.mdx` files at build time

A build step in codex reads the records and the recipes and writes one `.mdx` file for each recipe into `content/`: the recipe `title` as the `# ` line (and frontmatter when the recipe has a `description`), then for each section the heading and the body, with each code citation replaced by a fenced block and a permalink caption, and each diagram citation replaced by a `mermaid` block. The generated paths are gitignored in codex, and materialise removes each generated directory before it writes. codex keeps the `_meta.js` files above the generated directories (`content/_meta.js`, `content/specifications/_meta.js`) by hand. The path of a recipe under `pages/<repo>/` is the path of the page under `content/`; two recipes for one path fail the step. Sidebar order is a `_meta.json` file next to the recipes, and materialise writes the `_meta.js` that Nextra reads. A source repository pushes data and no code into the codex build.

A component that renders a recipe at request time was rejected for the first version. Nextra builds the table of contents, the sidebar, the search index, and the syntax highlight from files at compile time, and a component would have to rebuild each one. The records are plain data, so that path stays open.

Materialise also answers the review cost of D1: a local run prints the page for a recipe, so a reviewer reads the text in order.

### D10: A tool does the migration, and a round trip checks it

`split` takes an existing page and writes the snippet files and the recipe: it cuts at `##` to `####` (the cut `chunker.mjs` makes), writes the content before the first heading as a section with no level, moves each `mermaid` block into a `.mmd` file with a citation in its place, names each file from the `github-slugger` slug of its heading, writes the `title` as a quoted YAML string (153 headings hold inline markup), keeps a heading with no text as a snippet with an empty body (the heading split in `chunker.mjs` drops those, so `split` cannot call it unchanged), and converts each link to a codex route into a `snippet:` link.

There are two checks. The first is a round trip: materialise the split result over the page that codex has in git, and run `git diff`. An empty diff, apart from whitespace, proves the split lost nothing. The second compiles each snippet body alone with Nextra's `compileMdx`. The round trip cannot see a cut through a JSX element, because the broken parts join back to a correct page; the compile can. Rust fences stay in place through this step; D5 replaces them after.

Measured on the 363 Nym network pages: no JSX element, no `import` outside a code fence, no heading below level 4, no frontmatter. Each page has its title as a `# ` line, and `split` moves that line to the recipe `title`. So the heading cut is safe for the pilot. A page with a heading inside `<Steps>` or `<Tabs>` needs a rule (keep the whole element in one snippet) before `split` runs on the docs site content.

`split` does not make a section read correctly alone. A lint rule for backward references and relative links is a follow-up, and people fix sections over time. A split page that nobody has fixed yet renders as it did before.

### D11: The target decides visibility

A snippet has no visibility field. A public repository pushes to codex. A private repository pushes to the private target and holds no codex credential. The private target takes codex in full. A flag on each record was rejected: one wrong value would publish private content, and the two-store decision of the shared index already makes the boundary physical.

### D12: The tools are `.mjs` next to the retrieval modules

`split` and the extractor reuse `mdx.mjs` (frontmatter) and the fence-aware heading scan from `chunker.mjs`. They are plain `.mjs` with types in `types.ts`, as the retrieval modules are. They stay in this repository for the pilot. Materialise lives in codex, because codex runs it at build time. A shared package is a decision for the second source repository.

### D13: The corpus reaches the target form in stages, and each stage has a measure

The corpus does not meet the rules of this design today. About one section in five reads correctly alone, each section would get one aspect, no OpenSpec requirement names a threat-model entity, and 1,845 links are routes. An independent review listed these as reasons to keep the page as the unit of authoring.

The decision is to keep the target and change the data in stages: baseline, move and split, mechanical enrichment, standalone pass, threat model, requirement linkage, index and MCP. The plan has the table. A stage does not wait for the corpus to be clean: a split page that nobody has rewritten renders as it did, and a missing edge is reported as missing.

The rules that the corpus cannot meet at the start are warnings with a count, and each becomes an error when its stage ends: the standalone rule after stage 3, the required link on a requirement after stage 5. A rule that is an error from day one would block the first push.

The measures give two points where the design changes. If a retrieval mechanism shows no gain in stage 6, it stays off. If the standalone share stays far below 90% after stage 3, the page becomes the authored unit again and the records are derived from headings, with no change to a consumer.

### D14: Inline source paths are checked

The Nym network pages name 1,085 distinct source paths in 2,300 places. The record builder checks that each path in inline code exists in the repository and reports each one that does not. This is a warning until stage 2 ends, then an error. A path check is weaker than a symbol citation and covers far more text.

### Placeholders

The plan has the full table (P1 to P20). These are the ones that change this design when they are filled.

| Placeholder | Where it lands |
|---|---|
| TBD(task 2.3): the qualifier rules for a code id | D3 and D5. The code id form and the body file names are provisional until then. |
| TBD(stage 2 owner): the rule table from heading to aspect | D4d. Two rules exist. |
| TBD(Mark S): the relation `creates` | D4c, the `RefRel` list. |
| TBD(Max): an authority field and its values | D2, a new field on the record, and a new requirement in the spec. |
| TBD(threat-model owner): the exception list of the integrity check | D4b. One entry is known. |
| TBD(infra): the kind of write credential for codex | D8. |
| TBD(Max): codex follows `develop` or release tags | D8. The design assumes `develop`. |
| TBD(stage 6 owner): the gain in recall that keeps a mechanism on | D4e. |
| TBD(stage 0 owner): the baseline numbers | D4e and D13. |

## Risks

| Risk | Mitigation |
|---|---|
| A push credential for codex in this repository's CI | Scope it to the codex repository alone. codex CI rejects a push that touches a path outside the pusher's two directories. |
| About 2,600 authored files in a hidden directory are hard to find | The tools take the directory as an argument. A move to a visible directory is one rename. |
| Split sections read badly alone, and a second site exposes that | Stage 3 rewrites them, with a measured share. The codex pages render as before in the meantime. |
| Stage 5 needs people who know why each requirement exists, and it has no owner | Name an owner before stage 5 starts. The coverage report shows the gap until then. |
| A reviewer sees fragments | The local materialise command. A codex preview for each PR is a follow-up. |
| tree-sitter grammars add a dependency and WASM files | The shared index needs the same ones. Add them once. |
| Content that is not in codex today enters this repository | The split takes the pages that codex holds and takes content from no other source. codex is not public yet; the owner accepts that this repository shows the pages first. |
| A change to the threat model alters a claim that many pages rest on | CODEOWNERS on the model directory, the integrity check, and one lock line for each entity. |

## Verification

- Parser unit tests on fixture files: whole item, method, trait method, missing symbol, ambiguous symbol, doc comment and attribute grouping.
- The round trip and the per-snippet compile of D10 on the pilot section, then on all Nym network pages.
- For each replaced Rust fence, a comparison of the extracted body with the fence. List each difference in the PR as a stale hand copy or an extractor fault.
- `snippets.lock` and `code/` are stable: two runs on one commit give identical bytes.
- codex builds from the pushed directories, and a recipe with a wrong id fails that build.
- For the index (tasks section 6): recall at 6 on the labelled developer set is not lower than the baseline of task 0.2.

The agent sandbox has no network and no git, so the user runs `pnpm install`, the tests, the codex build, and each push.

## Follow-ups (not this change)

- `nym-vpn-client` as a source, with its 210 `file.rs:NN` citations.
- The private target and the private repositories.
- The docs site content in the same form.
- Recipes that select by metadata, and the metadata vocabulary.
- The missing L2 assessment in the `end-to-end` scenario, and the 16 verdicts that the per-actor assessments do not state.
- The 10 NymVPN threat diagrams in `codex/lib/privacy-model/nymvpn-diagrams.ts`. `nym-vpn-client` owns them. Each states its claim in a code comment today, so the claim needs a data field before a tool can project it.

# Tasks

Owner of each section: TBD(Max). Placeholders are marked `TBD(<who>)`; the plan has the table.

## 0. Baseline (no file moves)

- [ ] 0.1 Label two query sets against the chunks that exist now: about 30 "why" questions and the developer questions of the spike. For each, the chunk ids that a good answer needs
- [ ] 0.0 Owner: TBD(Max)
- [ ] 0.2 Record recall at 6 for plain vector search, a keyword hybrid, and expansion along the route links that the pages hold. User runs the embedding pass
- [ ] 0.3 Standalone sample: a person who did not write them reads 100 sections with no page around them. Record the share that passes

## 1. Form, split, and materialise on one section

- [ ] 1.1 `Snippet` and `Recipe` types in `documentation/docs/lib/retrieval/types.ts`
- [ ] 1.2 Add `.nym-codex/manifest.json` (`repo`, `github`)
- [ ] 1.3 `split` tool: one page in, snippet files and one recipe out. Cut at `##` to `####` with the heading split from `chunker.mjs`; content before the first heading becomes a section with no level; each `mermaid` block becomes a `.mmd` file and a citation; file names from the `github-slugger` slug of the heading; `aspect` from the heading template, with `aspect_review: true` where no rule matches; the `# ` title line becomes the recipe `title`; a heading with no text becomes a snippet with an empty body; titles are quoted YAML; each link to a codex route becomes a `snippet:` link; write `_meta.json` from the entries of each `_meta.js`
- [ ] 1.4 Record builder: read `.nym-codex/`, write `prose.jsonl` and `diagram.jsonl`. Fail on a duplicate id, a heading in a body, a prose-to-prose citation, a recipe section with no snippet, a `snippet:` link with no target, and a snippet that is not in use (no recipe lists it, and no listed prose snippet cites it)
- [ ] 1.5 Materialise script in codex (`scripts/materialise.mjs`): records and recipes in, one `.mdx` for each recipe and one `_meta.js` for each `_meta.json` out. Run it before `next build` and `next dev`
- [ ] 1.6 Local push script: write `snippets/nym/` and `pages/nym/` into a codex checkout, replacing both in full
- [ ] 1.7 Run `split` on `technical-details/discovery-urls` from codex. User runs the push script, materialise, and `git diff` in codex: the pages match apart from whitespace
- [ ] 1.8 Fixture tests for `split` and the record builder, with one test for each failure in 1.4
- [ ] 1.9 After 1.7 passes: `git rm --cached` the materialised paths in codex and gitignore them. Materialise removes each generated directory before it writes
- [ ] 1.10 Compile check in this repository (the docs package has `nextra`): compile each prose body alone with `compileMdx` and fail on an error. It runs in the PR check of 4.1 and in the codex build
- [ ] 1.11 Materialise: resolve each `snippet:` link to a route and anchor, write each heading with `[#<last part of id>]`, write the recipe `title` as the `# ` line

## 2. Code extractor

- [ ] 2.1 Audit the 24 Rust fences in `codex/content/specifications/nym-network`: for each, record the file, the symbol, and whether the fence is the whole item, an elided item, or part of a body. This fixes the citation list and the qualifier cases for 2.3
- [ ] 2.2 Add `web-tree-sitter` and the Rust grammar (user runs `pnpm install`). Check first whether `add-shared-retrieval-index` has added them
- [ ] 2.3 Parser module: `items(text, lang)` returns `{ name, qualifiedName, kind, startLine, endLine }` for each item, with doc comments and attributes inside the range. Methods carry `Type::method`. Fix the qualifier rules from the cases in 2.1: module path, two trait impls with one method name, `cfg` variants, the `const`/`static`/`type`/`impl` kinds, and items that a macro generates (not citable as items)
- [ ] 2.4 Fixture tests for 2.3: whole item, method, trait method, nested module, missing symbol, ambiguous symbol
- [ ] 2.5 Extractor: scan prose snippets for code citations, resolve each one, write `.nym-codex/code/` and `snippets.lock`, add `code.jsonl` to the record builder. Fail with file and line on a missing or ambiguous symbol
- [ ] 2.6 Materialise: render a code citation as a fenced block with a caption that links to the GitHub permalink (manifest commit, `path`, `lines`)
- [ ] 2.7 Determinism test: two runs give identical bytes
- [ ] 2.8 Add `.nym-codex` to `EXCLUDE` and `DOCS_EXCLUDE` in the shared index `collect.mjs`, so its root walk and its plain Markdown walk do not read the directory (`generate-code-index.mjs` walks `ROOTS` alone and needs no change)
- [ ] 2.9 Replace the Rust fences in the pilot section with citations. Compare each extracted body with the fence it replaces, and list each difference in the PR as a stale hand copy or an extractor fault

## 2b. Mechanical enrichment

- [ ] 2b.1 Inline source path check in the record builder: each repository path in inline code exists. A warning with a count
- [ ] 2b.2 Remove the sections that hold only a list of pages ("Documents in this section", "Technical details" index lists) where the recipes and `_meta.json` give the same list. List each removed section in the PR
- [ ] 2b.3 Counts on each run: links resolved, paths that exist, sections with an aspect from a rule, sections marked for aspect review
- [ ] 2b.4 `.nym-codex/edges.json`: read, validate, and merge into `refs`

## 3. All Nym network pages

- [ ] 3.1 Run `split` on the other Nym network specification pages. User repeats the round trip of 1.7 and the compile check of 1.10
- [ ] 3.2 Replace the other Rust fences with citations
- [ ] 3.3 Remove `content/specifications/nym-network/` from git in codex; materialise now writes it

## 4. CI

- [ ] 4.1 PR check: run the record builder, the extractor, link resolution, and the compile check of 1.10. Commit regenerated files under `.nym-codex/code/` and `snippets.lock` back to the branch, as `ci-docs-autodoc.yml` does. Fail on an error. Path-filter on `.nym-codex/**` and `**/*.rs` (a workflow path filter is static, so it cannot follow the citation list)
- [ ] 4.2 Push job on merge to `develop`: write the commit to `snippets/nym/manifest.json`, push a branch to codex, merge it when the codex build passes. One run at a time (concurrency group); stop when the commit in the codex manifest is not an ancestor of this one
- [ ] 4.3 codex CI: reject a push that touches a path outside the pusher's two directories
- [ ] 4.4 Create the codex write credential and add it to this repository's CI secrets. Kind of credential: TBD(infra)

## 4b. Standalone pass

- [ ] 4b.1 Lint: a body that refers to text above or below, and a title that is a template word with no subject. A warning with a count
- [ ] 4b.2 Rewrite the pilot section so that each snippet reads correctly alone. A model proposes, a person reviews. Repeat the read of 0.3 on it
- [ ] 4b.3 The other pages, in batches by directory. Record the share after each batch. Target 90%
- [ ] 4b.4 When the target holds, make the lint of 4b.1 an error

## 5. Authoring support

- [ ] 5.1 Local command that prints the materialised page for one recipe, for writers and reviewers
- [ ] 5.2 Warning for a prose body over 2,400 characters
- [ ] 5.3 Contributor guide page for `.nym-codex/` (a how-to: add a snippet, cite code, add it to a page)

## 6. Index (with `add-shared-retrieval-index`)

- [ ] 6.0 Bring `add-shared-retrieval-index` (branch `max/shared-mcp`) into the same line of work
- [ ] 6.6 MCP: `aspect` and `expand` parameters on `search_docs`; one tool `get_related(id, rel?, direction?)` with forward and backward lookup
- [ ] 6.1 Indexer reads snippet records and recipes for the content that `.nym-codex/` covers; it does not embed code records; it replaces each citation tag in `text` with the title of its target
- [ ] 6.7 The gain in recall that keeps a mechanism on: TBD(stage 6 owner)
- [ ] 6.5 Context prefix (aspect, titles of `realises` and `assessed-against` targets) and edge expansion at query time, each behind a switch. Measure each on and off against the sets of 0.1 and the baseline of 0.2, and record the result. Cache key of an embedding: the hash of the full embedded text
- [ ] 6.3 Acceptance: recall at 6 on the developer set of 0.1 is not lower than the baseline of 0.2
- [ ] 6.2 The code chunker calls the parser module from 2.3

## 6b. Threat model

- [ ] 6b.1 Add `"model"` to `SnippetKind`, and `SnippetRef` with the `RefRel` list, to the types of 1.1. `Aspect` is in the types from 1.1: the record builder fails on a prose snippet with none, and `split` writes `design`
- [ ] 6b.2 Threat-model adapter: evaluate `lib/privacy-model/threat-model.ts` and `examples/*.ts` with `ts-data.mjs`; write one `model` record for each actor, vector, property, scenario, and invariant, with `meta.data`, `meta.source`, `aspect` (`motivation`; `requirement` for an invariant), and `refs` from the data (`observes` from each actor to the vectors that list it, `assessed-against`, `specialises`)
- [ ] 6b.3 Projection: extend the functions in `projections.mjs` with route hops, the worked examples, and the comparison matrix. Fixture test with one multi-route scenario
- [ ] 6b.4 Integrity check: each referenced id exists; each scenario has an assessment for each spine actor or is on the exception list; a verdict has one source. Put `end-to-end` / L2 on the exception list with a note
- [ ] 6b.5 Materialise: map `view` (`diagram`, `assessment`, `matrix`, `card`) to the existing components and pass `meta.data`
- [ ] 6b.6 Replace the `export const scenario = require...Scenario(...)` wiring in the threat-model pages with citations, when those pages become snippets. Remove the scenario regex from `mdx.mjs` and the scenario branch of `projections.mjs`
- [ ] 6b.7 codex: delete `lib/privacy-model/threat-model.ts` and render `ActorsReference`, `VectorsReference`, `PropertiesReference` from `model.jsonl`
- [ ] 6b.10 Authority field: TBD(Max). If taken, add it to the types, the adapters, and the spec
- [ ] 6b.9 Give each cell of the comparison matrix an id as one claim (`nym/threat-model/claim/<scenario>/<property>/<actor>`), with the verdict and its condition in the body
- [ ] 6b.8 CODEOWNERS entry for `documentation/docs/lib/privacy-model/`. Names: TBD(Max)

## 6c. Edges

- [ ] 6c.0 `creates`: TBD(Mark S). Add it to `RefRel` when it has a meaning and one real edge
- [ ] 6c.1 Record builder: read `refs` from prose frontmatter (plain id or `{ id, rel }`), add `cites` edges from citations, normalise, remove duplicates, sort
- [ ] 6c.2 Fail on a relation that is not in `RefRel` and on an edge to an id with no snippet in this repository
- [ ] 6c.3 codex build: fail on an edge to an id that no repository provides

## 6e. Requirement linkage

- [ ] 6e.1 Name an owner for this stage: TBD(Max)
- [ ] 6e.2 Coverage report: for each requirement record, its edge to a threat-model entity (or a stated "none") and the code records that realise it
- [ ] 6e.3 For each of the 42 specs, state for each requirement the property, actor, or vector it serves, or "none". A model proposes, the owner reviews
- [ ] 6e.4 `realises` edges from code items to requirements in `edges.json`, starting with the specs that 6e.3 links to the threat model
- [ ] 6e.5 List the areas with no spec (Sphinx, cover traffic, mixing delay) as OpenSpec changes to write

## 6d. Other animations

- [ ] 6d.1 `PacketAnatomy`: its data is in `lib/privacy-model/packets.ts`. Add it as a model entity with a projection of the sizes and the framing steps
- [ ] 6d.2 Check the `packets.ts` constants that have a source in this repository against the Rust code (for example `IPR_MAX_PACKET` against `MAX_PACKET_SIZE` in `common/ip-packet-requests`), with the parser of 2.3
- [ ] 6d.3 `MixnetDeepDive`, `DvpnCoverTraffic`, `SurbDirectory`: for each, list the text in the component, move it to a typed data module, then add the entity and its projection

## 7. Adapters and clean-up

- [ ] 7.1 Adapter for `openspec/specs` (42 specs here; codex holds 35 pages from an older copy): derive one prose record for each requirement and one recipe for each spec, with the MDX sanitising that `codex/scripts/migrate-openspec.py` does. Then delete that script's `nym` entry
- [ ] 7.2 Delete or convert `documentation/docs/code-examples/sdk/typescript/*.mdx` (7 files, no importer)
- [ ] 7.3 Remove the untracked copy of the codex site at `nym/codex/` after the user confirms it is not needed
- [ ] 7.4 Update the codex README "Structure" section with the `snippets/` and `pages/` contract

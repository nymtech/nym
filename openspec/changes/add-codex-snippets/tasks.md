# Tasks

Owner of each section: TBD(Max). The marker `TBD(<who>)` shows each placeholder. The plan has the table.

## 0. Baseline (no file moves)

- [ ] 0.1 Label two query sets against the chunks that exist now. The sets are about 30 "why" questions and the developer questions of the spike. For each question, write down the chunk ids that a good answer needs
- [ ] 0.0 Owner: TBD(Max)
- [ ] 0.2 Write down recall at 6 for plain vector search and for a keyword hybrid. Do the same for expansion along the route links that the pages hold. The user runs the embedding pass
- [ ] 0.3 Standalone sample: a person who did not write them reads 100 sections with no page around them. Write down the share that passes

## 1. Form, split, and pages on one section

- [ ] 1.1 Add the `Snippet` type to `documentation/docs/lib/retrieval/types.ts`
- [ ] 1.2 Add `.nym-codex/manifest.json` (`repo`, `github`, and the page directories it owns in codex: `specifications/nym-network`)
- [ ] 1.3 Write the `split` tool: one page in, snippet files and one page that imports them out. Cut at `##` to `####` with the heading split from `chunker.mjs`. Each snippet keeps its heading on the first line, with the `github-slugger` slug as the file name and as the explicit anchor `[#slug]`. Content before the first heading becomes a snippet with no heading and a frontmatter `title` from the page title. Each `mermaid` block becomes a `.mmd` file and a citation. Take `aspect` from the heading template, and set `aspect_review: true` where no rule matches. The `# ` title line and the frontmatter stay in the page, with no change to the frontmatter. A heading with no text below it becomes a snippet with the heading alone. Each link to a codex route becomes a `snippet:` link. Write `_meta.json` from the entries of each `_meta.js`
- [ ] 1.4 Record builder: read `.nym-codex/`, write `prose.jsonl` and `diagram.jsonl`. Fail on a duplicate id, a second heading in a body, a heading anchor that is not the file name, a `title` in frontmatter next to a heading (or neither), a first-section `title` that differs from the page title, an import in a snippet, and a prose-to-prose citation. Fail on a `snippet:` link with no target. Fail on a snippet that is not in use: no page imports it, and no imported prose snippet cites it
- [ ] 1.5 Page check: a page holds frontmatter, one `# ` line, import lines of `@snippets/<repo>/...mdx` files that exist, and one tag for each import. Fail on any other content and on two imported snippets with one anchor.
- [ ] 1.6 Local push script: write `snippets/nym/` (snippet files and records) and the owned page directories under `content/` into a codex checkout, and replace each in full. Write `_meta.js` from each `_meta.json`, and write the link map `snippets/nym/pages.json` from the import lines of the pages. Set `lastUpdated` in each pushed page to the newest commit date of the page file and the snippet files it imports, and change no other frontmatter field
- [ ] 1.7 Run `split` on `technical-details/discovery-urls` from codex. Round trip: put the text of each snippet in place of its import and run `git diff` against the codex page. Expected result: the pages match apart from whitespace and the added anchors
- [ ] 1.8 Write fixture tests for `split`, the record builder, and the page check, with one test for each failure in 1.4 and 1.5
- [ ] 1.9 codex: add the `@snippets` alias for Turbopack (`turbopack.resolveAlias`) and webpack. Check that an imported snippet compiles, and that its headings and anchors reach the page table of contents, under `build` and `build:webpack`. Check that a function remark plugin in `mdxOptions.remarkPlugins` runs under both. If it does not run under Turbopack, use the fallback of D9 (a server component and an `a` override) in 1.11, 2.6, and 6b.5. The user runs both builds
- [ ] 1.10 Add a compile check in this repository (the docs package has `nextra`). It compiles each prose body alone with `compileMdx` and fails on an error. It runs in the PR check of 4.1 and in the codex build
- [ ] 1.11 codex: write `remarkSnippet` and add it to the Nextra `mdxOptions.remarkPlugins`. First job: resolve each `snippet:` link from the merged link map, and fail on a target that is not in the map

## 2. Code extractor

- [ ] 2.1 Audit the 24 Rust fences in `codex/content/specifications/nym-network`. For each fence, write down the file and the symbol. Also write down whether the fence is the whole item, an elided item, or part of a body. The audit sets the citation list and the qualifier cases for 2.3
- [ ] 2.2 Add `web-tree-sitter` and the Rust grammar (the user runs `pnpm install`). First check whether `add-shared-retrieval-index` added them
- [ ] 2.3 Parser module: `items(text, lang)` returns `{ name, qualifiedName, kind, startLine, endLine }` for each item, with doc comments and attributes inside the range. Methods use the form `Type::method`. Set the qualifier rules from the cases in 2.1. The cases are module path, two trait impls with one method name, and `cfg` variants. They are also the `const`/`static`/`type`/`impl` kinds and items that a macro generates (not citable as items)
- [ ] 2.4 Write fixture tests for 2.3: whole item, method, trait method, nested module, missing symbol, ambiguous symbol
- [ ] 2.5 Extractor: scan prose snippets for code citations and resolve each one. Add `code.jsonl` to the record builder. Fail with file and line on a missing or ambiguous symbol
- [ ] 2.6 `remarkSnippet`: replace a code citation with a fenced block and a caption. The caption links to the GitHub permalink (manifest commit, `path`, `lines`). Replace a diagram citation with a `mermaid` block
- [ ] 2.7 Determinism test: two runs give identical bytes
- [ ] 2.8 Add `.nym-codex` to `EXCLUDE` and `DOCS_EXCLUDE` in the shared index `collect.mjs`. Then its root walk and its plain Markdown walk do not read the directory. `generate-code-index.mjs` walks `ROOTS` alone and needs no change
- [ ] 2.9 Replace the Rust fences in the pilot section with citations. Compare each extracted body with the fence it replaces. List each difference in the PR as a stale hand copy or an extractor fault

## 2b. Mechanical enrichment

- [ ] 2b.1 Add an inline source path check to the record builder: each repository path in inline code exists. The check gives a warning with a count
- [ ] 2b.2 Remove the sections that hold only a list of pages, if `_meta.json` gives the same list. Examples are "Documents in this section" and the "Technical details" index lists. List each removed section in the PR
- [ ] 2b.3 Counts on each run: links resolved, paths that exist, sections with an aspect from a rule, sections marked for aspect review
- [ ] 2b.4 `.nym-codex/edges.json`: read, validate, and merge into `refs`

## 3. All Nym network pages

- [ ] 3.1 Run `split` on the other Nym network specification pages. The user repeats the round trip of 1.7 and the compile check of 1.10
- [ ] 3.2 Replace the other Rust fences with citations
- [ ] 3.3 From now on, only the push job writes `content/specifications/nym-network/` in codex

## 4. CI

- [ ] 4.1 PR check: run the record builder, the page check, the extractor, link resolution, and the compile check of 1.10. Post one PR comment with each changed code excerpt as a diff and the pages that show it. Update the comment on each new push to the PR. Fail on an error. Set the path filter to `.nym-codex/**` and `**/*.rs`. A workflow path filter is static, so it cannot follow the citation list
- [ ] 4.2 Push job on merge to `develop`: write the commit to `snippets/nym/manifest.json` and push a branch to codex. Merge the branch when the codex build passes. Run only one job at a time (concurrency group). Stop when the commit in the codex manifest is not an ancestor of this one
- [ ] 4.3 codex CI: reject a push that changes a path outside the paths that the pusher owns
- [ ] 4.4 Create the codex write credential and add it to this repository's CI secrets. Kind of credential: TBD(infra)

## 4b. Standalone pass

- [ ] 4b.1 Write a lint that finds a body that refers to text above or below. It also finds a title that is a template word with no subject. The lint gives a warning with a count
- [ ] 4b.2 Rewrite the pilot section so that each snippet passes a standalone read. A model proposes, and a person reviews. Repeat the read of 0.3 on it
- [ ] 4b.3 Do the same for the other pages, in batches by directory. Write down the share after each batch. The target is 90%
- [ ] 4b.4 When the target holds, make the lint of 4b.1 an error

## 5. Authoring support

- [ ] 5.1 Add a local command that prints one page with the text of each snippet in place of its import, for writers and reviewers (the function of the round trip in 1.7)
- [ ] 5.2 Add a warning for a prose body over 2,400 characters
- [ ] 5.3 Write a contributor guide page for `.nym-codex/` (a how-to: add a snippet, cite code, import it in a page)

## 6. Index (with `add-shared-retrieval-index`)

- [ ] 6.0 Bring `add-shared-retrieval-index` (branch `max/shared-mcp`) into the same line of work
- [ ] 6.6 MCP: add `aspect` and `expand` parameters to `search_docs`. Add one tool `get_related(id, rel?, direction?)` with forward and backward lookup
- [ ] 6.1 The indexer reads snippet records and pages for the content that `.nym-codex/` covers. It takes the page title and URL from the first page in path order that imports the snippet. It does not embed code records. It replaces each citation tag in `text` with the title of its target
- [ ] 6.7 The gain in recall that keeps a mechanism on: TBD(stage 6 owner)
- [ ] 6.5 Add the context prefix (aspect, titles of `realises` and `assessed-against` targets) and edge expansion at query time. Put each behind a switch. Measure each on and off against the sets of 0.1 and the baseline of 0.2. Write down the result. Cache key of an embedding: the hash of the full embedded text
- [ ] 6.3 Acceptance: recall at 6 on the developer set of 0.1 is not lower than the baseline of 0.2
- [ ] 6.2 The code chunker calls the parser module from 2.3

## 6b. Threat model

- [ ] 6b.1 Add `"model"` to `SnippetKind`, and `SnippetRef` with the `RefRel` list, to the types of 1.1. `Aspect` is in the types from 1.1. The record builder fails on a prose snippet with none, and `split` writes `design`
- [ ] 6b.2 Threat-model adapter: evaluate `lib/privacy-model/threat-model.ts` and `examples/*.ts` with `ts-data.mjs`. Write one `model` record for each actor, vector, property, scenario, and invariant. Give each record `meta.data` and `meta.source`. Set `aspect` to `motivation`, or to `requirement` for an invariant. Take `refs` from the data: `observes` from each actor to the vectors that list it, `assessed-against`, and `specialises`
- [ ] 6b.3 Projection: extend the functions in `projections.mjs` with route hops, the worked examples, and the comparison matrix. Add a fixture test with one multi-route scenario
- [ ] 6b.4 Integrity check: each referenced id exists. Each scenario has an assessment for each spine actor or is on the exception list. A verdict has one source. Put `end-to-end` / L2 on the exception list with a note
- [ ] 6b.5 `remarkSnippet`: map `view` (`diagram`, `assessment`, `matrix`, `card`) to the docs view components. Write a server component that reads the record by id from `model.jsonl` and passes `meta.data` to the view
- [ ] 6b.11 Ship the docs view components to codex (a package or a copy: TBD(Max)) and delete the older codex copies, including the codex `NetworkDiagram` that takes a `topology`
- [ ] 6b.6 Replace the `export const scenario = require...Scenario(...)` lines in the threat-model pages with citations, when those pages become snippets. Remove the scenario regex from `mdx.mjs` and the scenario branch of `projections.mjs`
- [ ] 6b.7 codex: delete `lib/privacy-model/threat-model.ts` and render `ActorsReference`, `VectorsReference`, `PropertiesReference` from `model.jsonl`
- [ ] 6b.10 Authority field: TBD(Max). If the decision is yes, add it to the types, the adapters, and the spec
- [ ] 6b.9 Give each cell of the comparison matrix an id as one claim (`nym/threat-model/claim/<scenario>/<property>/<actor>`). Put the verdict and its condition in the body
- [ ] 6b.8 Add a CODEOWNERS entry for `documentation/docs/lib/privacy-model/`. Names: TBD(Max)

## 6f. Motivation model

- [ ] 6f.1 Decide the entity types (start from stakeholder, driver, goal, principle), the directory, and the content owner. TBD(Max)
- [ ] 6f.2 Write the TypeScript types and the adapter, with the same record contract as the threat model
- [ ] 6f.3 The content owner writes the first entities. A model proposes nothing here: the text states why the network exists
- [ ] 6f.4 Add `realises` edges from threat-model properties to goals, and from requirements to principles. Add both to the coverage report

## 6c. Edges

- [ ] 6c.0 `creates`: TBD(Max). Add it to `RefRel` when it has a meaning and one real edge
- [ ] 6c.1 Record builder: read `refs` from prose frontmatter (plain id or `{ id, rel }`) and add `cites` edges from citations. Then normalise, remove duplicates, and sort
- [ ] 6c.2 Fail on a relation that is not in `RefRel`. Also fail on an edge to an id with no snippet in this repository
- [ ] 6c.3 codex build: fail on an edge to an id that no repository provides

## 6e. Requirement linkage

- [ ] 6e.1 Name an owner for this stage: TBD(Max)
- [ ] 6e.2 Coverage report: for each requirement record, show its edge to a threat-model entity or a stated "none". Also show the code records that realise it
- [ ] 6e.3 For each of the 42 specs, state for each requirement the property, actor, or vector it serves, or "none". A model proposes, and the owner reviews
- [ ] 6e.4 Add `realises` edges from code items to requirements in `edges.json`. Start with the specs that 6e.3 links to the threat model
- [ ] 6e.5 List the areas that no spec defines (known: the Sphinx packet format, client cover traffic) as OpenSpec changes to write. Decide if `nym-swizzle` covers the mixing delay

## 6d. Other animations

- [ ] 6d.1 `PacketAnatomy`: its data is in `lib/privacy-model/packets.ts`. Add it as a model entity with a projection of the sizes and the framing steps
- [ ] 6d.2 Some `packets.ts` constants have a source in this repository. Check those against the Rust code with the parser of 2.3. One example is `IPR_MAX_PACKET` against `MAX_PACKET_SIZE` in `common/ip-packet-requests`
- [ ] 6d.3 `MixnetDeepDive`, `DvpnCoverTraffic`, `SurbDirectory`: for each, list the text in the component and move it to a typed data module. Then add the entity and its projection

## 7. Adapters and clean-up

- [ ] 7.1b Statement fields: `meta.level` from the strongest RFC 2119 keyword, `meta.status` from the enforced list in `.nym-codex/manifest.json` (default `approved`). Fail on two requirements with one slug in one spec
- [ ] 7.1c Who promotes a statement to `enforced`, and the rule: TBD(Max)
- [ ] 7.1 Write an adapter for `openspec/specs`. This repository has 42 specs, and codex holds 35 pages from an older copy. Derive one prose record for each requirement and one page for each spec. Apply the MDX sanitisation that `codex/scripts/migrate-openspec.py` does. Then delete that script's `nym` entry
- [ ] 7.2 Delete or convert `documentation/docs/code-examples/sdk/typescript/*.mdx` (7 files, no importer)
- [ ] 7.3 Remove the untracked copy of the codex site at `nym/codex/` after the user confirms that nobody needs it
- [ ] 7.4 Update the codex README "Structure" section with the `snippets/` contract, the owned `content/` paths, and the `@snippets` alias

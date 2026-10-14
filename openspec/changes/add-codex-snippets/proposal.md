## Why

codex is a push-only aggregation target. Its README says that source repositories generate specification material and push it. No source repository does that yet. The content in codex is a hand copy. It includes 24 Rust excerpts from this monorepo and 363 Nym network specification pages. A hand copy goes stale with no signal: a struct gets a new field, and the excerpt in codex stays as it was.

The retrieval index has a related problem. It cuts pages with one chunker and source files with a second one. Each new consumer of the same content adds its own cut.

A third need is more than one documentation site from the same content, with no copy of the prose.

This change defines one unit, the snippet, and one typed record format. It defines one path from a source repository to codex. People author prose and diagram snippets as files. A tool extracts code snippets by symbol. A page is a recipe: an ordered list of snippet ids. The change starts with this monorepo. The plan with the flow and the format is `documentation/NYM-CODEX-SNIPPETS-PLAN.md`.

## What Changes

- Add a `.nym-codex/` directory to this repository. It holds a manifest, an edges file, prose snippets (`.mdx`), diagram snippets (`.mmd`), and page recipes (`.json`). It holds no generated file.
- Define the `Snippet` record and the id rules. A record maps onto the retrieval `Chunk`.
- Add a citation form for prose snippets, `<Snippet id="..." />`. A citation of code is the request to extract. A citation that does not resolve fails the extractor.
- Add an extractor. It finds each cited code item by symbol with tree-sitter.
- Add a `model` kind for typed data. The threat model is the first one. Each actor, vector, property, scenario, and invariant gets one record with a text projection and the data. An integrity check covers the model. An animated diagram is a view of a scenario record. A motivation model (stakeholders, drivers, goals, principles) is the next model, and edges connect it to the threat model and to the requirements.
- Give each edge in `refs` a relation, `{ id, rel }`, from a closed list.
- Add a required `aspect` to each record (`motivation`, `requirement`, `design`, `implementation`, `operation`).
- In the index: put aspect and edge context in the embedded text, and add edge expansion at query time. Labelled question sets measure both.
- Change the content in stages, each with a measure. The stages are baseline, move and split, mechanical enrichment, standalone pass, threat model, requirement linkage, index and MCP. A rule that the content does not meet yet starts as a warning.
- Add `.nym-codex/edges.json` for edges on generated records, a coverage report, and a check of inline source paths.
- Add a migration tool, `split`. It cuts a page at its headings into snippet files and a recipe.
- Add a CI check on each PR. It posts a comment with each changed code excerpt and the pages that show it, and it fails on an error. Add a push job on merge to `develop`. It replaces the two directories of this repository in codex.
- Split the Nym network specification pages from codex into `.nym-codex/` and replace their Rust fences with citations.

In codex (a separate repository, listed here because the format is its contract):

- Add `snippets/<repo>/{prose,code,diagram,model}.jsonl` and `pages/<repo>/`.
- Delete the codex copy of the threat-model data. It differs from the docs copy today.
- Add a build step, materialise, that writes one `.mdx` page for each recipe into `content/`.

## Capabilities

### New Capabilities

- `codex-snippets`: the snippet files, the record, the id rules, edges, recipes, and the model kind. Also code extraction, the committed output, the drift check, and the push to codex.

### Modified Capabilities

None in this change. `add-shared-retrieval-index` changes the indexer input to snippet records later (tasks section 6).

## Impact

- New directory `.nym-codex/` at the repository root. The split of all Nym network pages brings it to about 2,600 authored files.
- New tool code under `documentation/`, next to the retrieval modules it shares a parser with.
- New dependencies for the extractor: `web-tree-sitter` and the Rust grammar. `add-shared-retrieval-index` decision 10 needs the same two. The first of the two changes to merge adds them.
- New CI: one PR check and one push job. The push job needs a credential that can write to codex.
- The `EXCLUDE` patterns of the shared index walk get `.nym-codex`.
- No change to product code. A citation names a symbol, and the extractor reads the source file.
- This change does not alter the docs site.

## Placeholders

Owners, dates, baseline numbers, and some design values are not known yet. A `TBD(<who>)` marker in `design.md` and `tasks.md` shows each one. `documentation/NYM-CODEX-SNIPPETS-PLAN.md` has the full table.

## Non-Goals

- The private flow. A private repository pushes to a private target that holds codex and private content. This change states that boundary and builds no part of it.
- `nym-vpn-client` and other source repositories.
- The docs site content.
- Recipes that select snippets by metadata.
- The embedding provider and the vector store.

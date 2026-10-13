## Why

codex is a push-only aggregation target: its README says source repositories generate specification material and push it. No source repository does that yet. The content in codex is a hand copy, which includes 24 Rust excerpts pasted from this monorepo and 363 Nym network specification pages. A hand copy goes stale without a signal: a struct gains a field, and the excerpt in codex stays as it was.

The retrieval index has a related problem. It cuts pages with one chunker and source files with a second one, and each new consumer of the same content would add its own cut.

A third need is to build more than one documentation site from the same content with no copy of the prose.

This change defines one unit, the snippet, with one typed record format, and one path from a source repository to codex. People author prose and diagram snippets as files. A tool extracts code snippets by symbol. A page is a recipe: an ordered list of snippet ids. The change starts with this monorepo. The plan with the flow and the format is `documentation/NYM-CODEX-SNIPPETS-PLAN.md`.

## What Changes

- Add a `.nym-codex/` directory to this repository: a manifest, authored prose snippets (`.md`), authored diagram snippets (`.mmd`), page recipes (`.json`), the extracted code bodies, and a lock file.
- Define the `Snippet` record and the id rules. A record maps onto the retrieval `Chunk`.
- Add a citation form for prose snippets, `<Snippet id="..." />`. A citation of code is the request to extract, and a citation that does not resolve fails the extractor.
- Add an extractor that finds each cited code item by symbol with tree-sitter and writes the committed code bodies and the lock file.
- Add a `model` kind for typed data. The threat model is the first one: one record for each actor, vector, property, scenario, and invariant, with a text projection, the data, and an integrity check. An animated diagram is a view of a scenario record.
- Make `refs` typed edges, `{ id, rel }`, with a closed list of relations.
- Add a required `aspect` to each record (`motivation`, `requirement`, `design`, `implementation`, `operation`).
- In the index: aspect and edge context in the embedded text, edge expansion at query time, and a labelled set of "why" questions that measures both.
- Change the existing content in stages, each with a measure: baseline, move and split, mechanical enrichment, a standalone rewrite pass, the threat model, requirement linkage, index and MCP. Rules that the content does not meet yet start as warnings.
- Add `.nym-codex/edges.json` for edges on generated records, a coverage report, and a check of inline source paths.
- Add a migration tool, `split`, that cuts an existing page at its headings into snippet files and a recipe.
- Add a CI check on each PR (regenerate, fail on a difference) and a push job on merge to `develop` that replaces this repository's two directories in codex.
- Split the Nym network specification pages from codex into `.nym-codex/` and replace their Rust fences with citations.

In codex (a separate repository, listed here because the format is its contract):

- Add `snippets/<repo>/{prose,code,diagram,model}.jsonl` and `pages/<repo>/`.
- Delete the codex copy of the threat-model data, which differs from the docs copy today.
- Add a build step, materialise, that writes one `.mdx` page for each recipe into `content/`.

## Capabilities

### New Capabilities

- `codex-snippets`: the snippet file forms, the record, the id rules, typed edges, recipes, the model kind, code extraction, the committed output, the drift check, and the push to codex.

### Modified Capabilities

None in this change. `add-shared-retrieval-index` changes the indexer input to snippet records as a follow-up (tasks section 6).

## Impact

- New directory `.nym-codex/` at the repository root. The split of all Nym network pages brings it to about 2,600 authored files.
- New tool code under `documentation/`, next to the retrieval modules it shares a parser with.
- New dependencies for the extractor: `web-tree-sitter` and the Rust grammar. `add-shared-retrieval-index` decision 10 needs the same two, so the first of the two changes to land adds them.
- New CI: one PR check and one push job. The push job needs a credential that can write to codex.
- The `EXCLUDE` patterns of the code indexers gain `.nym-codex`.
- No change to product code. A citation names a symbol, and the extractor reads the source file.
- The docs site is unchanged in this change.

## Placeholders

Owners, dates, baseline numbers, and seven design values are not known yet. Each is marked `TBD(<who>)` in `design.md` and `tasks.md`, and `documentation/NYM-CODEX-SNIPPETS-PLAN.md` lists all twenty.

## Non-Goals

- The private flow. A private repository pushes to a private target that holds codex plus private content. This change states that boundary and builds none of it.
- `nym-vpn-client` and other source repositories.
- The docs site content.
- Recipes that select snippets by metadata.
- The embedding provider and the vector store.

# `.nym-codex/`: the snippets of the Nym docs

A page of the docs site (`documentation/docs/content/`) can hold its text in snippets: one file for each section, here in `snippets/`. The page imports them in order. Other sites (codex) import the same files, so a change to a snippet reaches each site that shows it.

| Path | Content |
|------|---------|
| `snippets/` | The snippets: one `.mdx` file for each section, and one `.mmd` file for each diagram. The path is the id: `snippets/developers/rust/intro.mdx` is `nym/developers/rust/intro`. |
| `manifest.json` | The sites that import snippets: the page root of each, and the components a snippet may use there. |
| `edges.json` | Edges on records that have no frontmatter, as `{ from, to, rel }`: for example a `realises` edge from a code item to a requirement. |
| `pages/` | Pages written for codex by the split of the pilot section. |
| `build/` | Files the build writes (`model.jsonl`, the threat-model records). Git ignores it. |

All commands below run from `documentation/docs`. The section "Snippets and records" of `documentation/README.md` explains the structure, the records, and the messages of the threat-model step of the build.

## Change the text of a page

1. Find the snippet. The page file lists its snippets as imports: `import Intro from '@snippets/nym/developers/rust/intro.mdx'` is `snippets/developers/rust/intro.mdx`.
2. Edit the snippet. Do not copy the text back into the page.
3. Run the checks:

   ```sh
   node lib/snippets/records.mjs
   node lib/snippets/compile.mjs
   ```

## Add a section

1. Write a new file in the directory of the page, named after the section: `snippets/developers/rust/feature-flags.mdx`.
2. Start it with frontmatter and the section heading, with an explicit anchor:

   ```mdx
   ---
   aspect: design
   ---

   ## Feature flags [#feature-flags]

   The text of the section.
   ```

   `aspect` is one of `motivation`, `requirement`, `design`, `implementation`, `operation`. The first snippet of a page has no heading of its own: it gives `title` in the frontmatter instead, and the page holds the `# ` title.
3. Import it in the page, at the place where the section goes:

   ```mdx
   import FeatureFlags from '@snippets/nym/developers/rust/feature-flags.mdx'

   <FeatureFlags />
   ```

4. Run the checks of "Change the text of a page".

A snippet must read on its own, because a site or a search result can show it with no page around it. Do not write "the table below" or "this page". `node lib/snippets/standalone.mjs --list` lists the snippets that do.

## Write a snippet: the rules

A snippet imports nothing. It can use only these:

| To show | Write |
|---------|-------|
| A link to another snippet | `[the stream module](snippet:nym/developers/rust/stream/intro)`. The site writes the route and the anchor. |
| A diagram | a `.mmd` file beside the snippet, and `<Snippet id="nym/<path of the .mmd file>" />` |
| A threat-model scenario | `<Snippet id="nym/threat-model/scenario/mixnet" view="diagram" />`, with `view` one of `diagram`, `assessment`, `card`. A worked example is `nym/threat-model/example/wallet/scenario/<id>`, and a page-specific drawing of a scenario is `nym/threat-model/variant/<id>` (`diagram` only, data in `lib/privacy-model/examples/variants.ts`). |
| A threat-model table | `<Snippet id="nym/threat-model/actor" view="reference" />` (also `vector`, `property`), or `<Snippet id="nym/threat-model/scenario" view="matrix" />` |
| A version | `<Version name="RUST_MSRV" />`, from `components/versions.ts`. Add `code` to show it as inline code. |
| A network count | `<Stat name="nodes" />`, from `components/outputs/api-scraping-outputs/nodes-count.json` |
| An interactive demo | `<Demo name="EnsDemo" />`. The names are in `components/demos/demo.tsx`. |
| Other components | the names in `manifest.json` at `sites.docs.components`, for example `Callout`, `Steps`, `Tabs` |

To give snippets a new component, add it to `mdx-components.jsx` and to `manifest.json`. A test checks that the two lists agree. Keep it light: `mdx-components.jsx` is part of every page.

## A how-to page with steps

A page with `diataxis: how-to` in its frontmatter keeps the `<Steps>` element, and each step is its own snippet. The step heading is one level below its section (depth 3 under a `##`), with no number and with an explicit anchor: `### Create a service file [#create-a-service-file]`. `pnpm lint:runbooks` checks it. The section "Runbook form" of `documentation/README.md` has the rules.

## Split a page into snippets

```sh
node lib/snippets/split.mjs --site docs --dir <page file or directory under content>
```

The tool writes the snippets, and replaces the text of each page with its imports. A page waits, with a reason, when the split cannot keep its text the same: for example a component that `manifest.json` does not list. To check that the split changed no text output, save the outputs before and after, and compare:

```sh
node lib/snippets/text-outputs.mjs --save /tmp/before
node lib/snippets/split.mjs --site docs --dir <dir>
node lib/snippets/text-outputs.mjs --save /tmp/after
node lib/snippets/text-outputs.mjs --compare /tmp/before /tmp/after
```

## Checks

| Command | What it checks |
|---------|----------------|
| `node lib/snippets/records.mjs` | Each snippet, page, import, citation, and `snippet:` link |
| `node lib/snippets/compile.mjs` | That each snippet compiles alone |
| `node lib/snippets/standalone.mjs --list` | Snippets that refer to text around them |
| `node lib/archetypes/anchors.mts --list` | That each `#anchor` link names an id on its page |
| `pnpm test` | The unit tests |

The `ci-codex-snippets` workflow runs the first two on each PR. The design is in `documentation/NYM-CODEX-SNIPPETS-PLAN.md`.

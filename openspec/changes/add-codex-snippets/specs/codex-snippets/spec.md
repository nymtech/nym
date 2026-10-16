# codex-snippets Spec Delta

## ADDED Requirements

### Requirement: A person authors each prose and diagram snippet as one file

In this repository, the `.nym-codex/` directory SHALL be `documentation/docs/.nym-codex/`, inside the docs package, so that the docs build can import its files. A prose snippet SHALL be one MDX file (`.mdx`) under `.nym-codex/snippets/`. Its body SHALL contain at most one heading outside a code fence, and that heading SHALL be on the first line of the body. The heading SHALL hold an explicit anchor (`[#anchor]`) equal to the file name with no extension. The heading text SHALL be the title of the snippet. A prose snippet with no heading SHALL have a `title` in its frontmatter, and a prose snippet with a heading SHALL NOT have one. A prose snippet SHALL NOT import a file. A diagram snippet SHALL be one Mermaid `.mmd` file under the same directory. The id of each SHALL be `<repo>/<path under snippets/ with no extension>`. The tools SHALL fail when two snippets have one id.

#### Scenario: The docs build imports a snippet

- **WHEN** a docs page imports `@snippets/nym/<id>.mdx`, and the docs build runs with Turbopack
- **THEN** the import resolves to `documentation/docs/.nym-codex/snippets/<id>.mdx`, inside the root that Turbopack reads

#### Scenario: A heading edit keeps the id

- **WHEN** an author changes the heading text of a prose snippet
- **THEN** the id and the anchor do not change, and each page that imports the snippet stays valid

#### Scenario: A body has a second heading

- **WHEN** a prose snippet body contains a second line that is a Markdown heading outside a code fence
- **THEN** the PR check fails and names the file

#### Scenario: A heading anchor does not match the file name

- **WHEN** the file `constructors.mdx` starts with `## Constructors [#ctors]`
- **THEN** the PR check fails and names the file

#### Scenario: A prose snippet cites a prose snippet

- **WHEN** a prose snippet body contains `<Snippet id>` with the id of a prose snippet, or an `import` line
- **THEN** the PR check fails and names the file

### Requirement: A typed model has one record for each entity

For a typed data model such as the threat model, the tools SHALL evaluate the source modules. The tools SHALL emit one record of kind `model` for each entity. The id of the record SHALL be `<repo>/<model name>/<entity type>/<id in the data>`. The record SHALL hold the evaluated entity in `meta.data` and a text projection of the same entity in `body`. The projection of a scenario SHALL state each route as an ordered list of hops, and each verdict. For each actor, it SHALL state what the actor sees, what the actor cannot see, and the residual risk. Each model SHALL have its own adapter and its own integrity rules. An edge between two models SHALL use the same relation list as other edges. This repository SHALL hold the one source of the threat model, and no consumer SHALL hold a second copy of the data.

#### Scenario: An edge between two models

- **WHEN** the motivation model has a goal, and a threat-model property has a `realises` edge to that goal
- **THEN** a consumer reaches the goal from the property, and the goal from a requirement that realises the property

#### Scenario: The index finds what a diagram shows

- **WHEN** the indexer reads the record of the `mixnet` scenario
- **THEN** the chunk text names each hop of the route in order and states what the destination sees and cannot see

#### Scenario: A worked example has records

- **WHEN** the tools build the records
- **THEN** each scenario, the L1 actor, and each invariant of the three worked examples has a record

#### Scenario: A page shows a view of an entity

- **WHEN** a prose snippet cites a scenario with `view="diagram"` and a second one cites the same scenario with `view="assessment"`
- **THEN** the page renders the animated diagram and the assessment panel from one record

#### Scenario: An edit to an actor reaches each site

- **WHEN** a merged PR changes what the L2 actor observes
- **THEN** the next push changes one record, and the docs site and codex show the new text with no second edit

### Requirement: The threat model passes an integrity check

The PR check SHALL fail when a model entity references an id that does not exist. The PR check SHALL fail when a scenario has no assessment for a spine actor and is not on the exception list. A verdict SHALL have one source in the data.

#### Scenario: A scenario omits an actor

- **WHEN** a PR adds a scenario with no assessment for L3G and does not add an exception
- **THEN** the PR check fails and names the scenario and the actor

### Requirement: Each edge has a relation

`refs` in a record SHALL be a list of `{ id, rel }`, sorted and with no duplicate. `rel` SHALL be a member of the closed relation list. The tools SHALL write a `cites` edge for each citation in a body and a `relates-to` edge for each `snippet:` link. The tools SHALL write the model relations from the model data. An author MAY add edges in the frontmatter of a prose snippet. An authored edge SHALL be a plain id, which means `relates-to`, or an object with `id` and `rel`. The check SHALL fail on a relation that is not on the list and on an edge to an id that has no snippet.

#### Scenario: A plain id and a typed edge in one list

- **WHEN** a prose snippet has `refs` with the plain id `nym/threat-model/actor/L2` and the object `{ id: X, rel: depends-on }`
- **THEN** the record has two edges, the first with `rel` `relates-to` and the second with `rel` `depends-on`

#### Scenario: A citation becomes an edge

- **WHEN** a prose body cites a code snippet
- **THEN** the record has a `cites` edge to that code id, and the author writes no `refs` entry for it

#### Scenario: An unknown relation

- **WHEN** an author writes `rel: dependsOn`
- **THEN** the PR check fails and names the file

#### Scenario: An edge in the data becomes an edge in the record

- **WHEN** vector V2 lists L3L and L3G in `observableFrom`
- **THEN** the L3L record and the L3G record each have an `observes` edge to the V2 id

#### Scenario: From code to the reason for it

- **WHEN** a code snippet has a `realises` edge to a requirement, and the requirement has a `realises` edge to property P1
- **THEN** a consumer reaches the P1 record from the code record in two steps

### Requirement: A page imports snippets

A page SHALL be one MDX file under a page root that `.nym-codex/manifest.json` lists for a site (`.nym-codex/pages/` for codex, `documentation/docs/content/` for the docs site). It SHALL hold frontmatter, one `# ` title line, import lines, and one tag for each import, and no other content. The one exception SHALL be a wrapper element that the archetype of the page names, with only snippet tags inside it. Each import path SHALL be `@snippets/<snippet id>.mdx`. A site SHALL give each wrapper element to MDX in `mdx-components`, so that a page imports nothing that is not a snippet. The frontmatter of a page SHALL keep the SEO fields `title`, `description`, `schemaType`, `section`, and `lastUpdated` where the page has them, and MAY add other fields. The migration tool SHALL copy the frontmatter of a page unchanged. The path of the page under its page root SHALL be the path of the page on the site. The tools SHALL fail when a page imports a file that does not exist, holds other content, or imports two snippets with one anchor. A snippet is in use when a page of its repository imports it. A snippet is also in use when a prose snippet that such a page imports cites it. The tools SHALL fail when a prose or diagram snippet is not in use.

#### Scenario: A page shows the headings of its snippets

- **WHEN** a page imports three prose snippets, and two of them start with a level 2 heading
- **THEN** the page shows both headings with their anchors, and its table of contents lists both

#### Scenario: A split docs page keeps its SEO metadata

- **WHEN** the migration tool splits a docs page with `title`, `description`, `schemaType`, `section`, and `lastUpdated` in its frontmatter
- **THEN** the new page has the same frontmatter, and the site renders the same meta description and JSON-LD for it

#### Scenario: A page holds prose of its own

- **WHEN** a page holds a paragraph between two import tags
- **THEN** the PR check fails and names the page

#### Scenario: A runbook page wraps its steps

- **WHEN** a page with `diataxis: how-to` holds `<Steps>` with two step snippet tags inside it
- **THEN** the page check passes, and the site numbers the two step headings

#### Scenario: A wrapper holds prose

- **WHEN** a `<Steps>` element on a page holds a paragraph next to the snippet tags
- **THEN** the PR check fails and names the page

#### Scenario: A diagram is in use through a citation

- **WHEN** a prose snippet that a page imports cites a diagram snippet
- **THEN** the check passes

#### Scenario: A page imports a snippet that does not exist

- **WHEN** a PR deletes a snippet file that a page still imports
- **THEN** the PR check fails and names the page

### Requirement: A snippet uses only the components that a site gives

A snippet SHALL import no component. Each JSX element in a snippet body SHALL be a component that the `mdx-components` of the site gives. The record builder SHALL write the names of the JSX elements in a body to `meta.components`. The page check of a site SHALL fail when a page imports a snippet whose `meta.components` holds a component that the site does not give, and SHALL name the page, the snippet, and the component.

#### Scenario: A docs snippet holds a demo

- **WHEN** a docs snippet renders a demo component that the docs site gives and codex does not
- **THEN** the docs page that imports it passes the page check, and the codex build fails only if a codex page imports the snippet

#### Scenario: A split docs page keeps its Callout

- **WHEN** the migration tool splits a docs page that imports `Callout` from `nextra/components`
- **THEN** the snippet holds the `<Callout>` element with no import, and the page renders as before

### Requirement: Text outputs of a site hold the text of its snippets

Each build step of a site that reads pages as text SHALL read the composed page: the text of each imported snippet with no frontmatter, the text of each `<Snippet>` citation in place of its tag, and each `snippet:` link as the route of its target on that site. All such steps SHALL use one compose function. The function SHALL fail, and name the page and the line, when an import or a citation does not resolve. On the docs site, this covers the raw Markdown mirror, `llms-full.txt`, and the docs index. A chunk of the docs index SHALL take its anchor from the explicit anchor of its heading.

#### Scenario: A split page in llms-full.txt

- **WHEN** a docs page is split into snippets and the site builds
- **THEN** `llms-full.txt` and the raw Markdown mirror of the page hold the same text as before the split
- **AND** each chunk of the page in the docs index has the same `id`, `url`, `heading`, and `text` as before the split

#### Scenario: A snippet import that does not resolve

- **WHEN** a docs page imports `@snippets/nym/...` for a file that does not exist
- **THEN** the text output step fails and names the page and the line, and writes no output with the page text missing

#### Scenario: A heading edit keeps the chunk id

- **WHEN** an author changes the text of `## Constructors [#constructors]`
- **THEN** the chunk of that section keeps the id and URL with `#constructors`

### Requirement: A snippet has one typed record

The tools SHALL emit each snippet as one `Snippet` record. The record SHALL have the fields `id`, `kind`, `aspect`, `authority`, `repo`, `path`, `title`, `body`, `hash`, `refs`, and `meta`. A code record and a diagram record SHALL have `lang`. A code record SHALL have `lines`. A record SHALL hold no URL, no page title, and no source commit. `kind` SHALL be one of `prose`, `code`, `diagram`, `model`. `aspect` SHALL be one of `motivation`, `requirement`, `design`, `implementation`, `operation`. `authority` SHALL be `normative` for an OpenSpec requirement and a threat-model record, `illustrative` for prose on a how-to or tutorial page, and `descriptive` for other prose and for code; a diagram SHALL take the authority of the prose that cites it. The tools SHALL set it for the code and model kinds. The check SHALL fail on a prose snippet that has none. `hash` SHALL be the SHA-256 of `body`. For the prose kind, `body` SHALL be the source text below the heading, with no other edit. For the diagram and code kinds, `body` SHALL be the source text with no edit. For the model kind, `body` SHALL be the text projection.

#### Scenario: A code record carries its source location

- **WHEN** the tools emit a code snippet for `NymNetworkDetails` in `common/network-defaults/src/network.rs`
- **THEN** the record has `kind` `code`, `aspect` `implementation`, and `lang` `rust`
- **AND** the record has that file as `path`, the first and last line of the item as `lines`, and the symbol as `title`

#### Scenario: A requirement outranks a specification page

- **WHEN** an OpenSpec requirement and a specification page state two different values for one thing
- **THEN** the requirement record has `authority` `normative` and the page record `descriptive`, so a consumer knows which one the project commits to

#### Scenario: A record maps to a retrieval chunk

- **WHEN** the indexer reads a prose record
- **THEN** it builds one `Chunk` from the record and the pages of the site it serves
- **AND** it makes no cut of its own, apart from the split of an oversized body

#### Scenario: A push with one changed snippet

- **WHEN** a merged PR changes the body of one prose snippet
- **THEN** `prose.jsonl` in codex changes in one line

### Requirement: A snippet cites code by symbol

A prose snippet SHALL cite code with `<Snippet id="<repo>/<source file path>#<symbol>" />`. The extractor SHALL find the item with a parser, and SHALL include the doc comments and attributes of the item in the body. The extractor SHALL fail, and name the file and line of the citation, when the symbol does not exist or matches more than one item.

#### Scenario: A method resolves through its type

- **WHEN** a snippet cites `nym/nym-signers-monitor/src/test_result.rs#TestResult::quorum_unavailable`
- **THEN** the body is that method and no other part of the `impl` block

#### Scenario: A renamed symbol fails the check

- **WHEN** a PR renames a struct that a snippet cites and does not update the citation
- **THEN** the PR check fails and names the snippet that cites the old name

#### Scenario: Lines above the item move

- **WHEN** a PR adds code above a cited item and does not change the item
- **THEN** the body and its hash do not change

### Requirement: The PR check shows excerpt changes, and the repository commits no generated output

The source repository SHALL commit no extracted code body and no other generated output under `.nym-codex/`. On each PR, a check SHALL run the extractor and post one comment. The comment SHALL show each code excerpt that changes, as a diff, and the pages that show it. The check SHALL fail on an error. An index walk over the repository tree SHALL exclude `.nym-codex/`, and the indexer SHALL NOT embed a code record as a chunk of its own.

#### Scenario: A code change shows its excerpt change

- **WHEN** a PR adds a field to a cited struct
- **THEN** the PR check comment shows the new field in the diff of that excerpt and names the pages that cite it

#### Scenario: A PR touches no cited item

- **WHEN** a PR changes Rust code that no snippet cites
- **THEN** the PR check posts no excerpt diff and adds no file to the branch

#### Scenario: The output is deterministic

- **WHEN** the extractor runs twice on one commit
- **THEN** the two results are byte-identical

#### Scenario: The index holds a cited item once

- **WHEN** the code indexer walks the repository
- **THEN** it reads the source file of a cited item and reads no file under `.nym-codex/`

### Requirement: The push replaces the repository's paths in codex

On merge to the branch that the push follows (`develop` for the pilot; P18 can change it to release tags), a job SHALL build the records at the merged commit. The job SHALL write that commit to `snippets/<repo>/manifest.json`. The job SHALL replace `snippets/<repo>/` in codex in full, with the snippet files and the records. The job SHALL replace in full each page directory under `content/` that the manifest of the repository lists, and SHALL write a `_meta.js` next to each `_meta.json`. In each pushed page, the job SHALL set `lastUpdated` to the newest commit date of the page file and the snippet files it imports, and SHALL change no other frontmatter field. The job SHALL write a link map, `snippets/<repo>/pages.json`, from snippet id to page path and anchor, from the import lines of the pages. The job SHALL write no other path in codex. Two runs SHALL NOT overlap. A run SHALL stop when the commit in the codex manifest is not an ancestor of the commit it pushes.

#### Scenario: A deleted snippet leaves codex

- **WHEN** a merged PR deletes a snippet file and removes it from each page
- **THEN** the next push removes its file and its record from codex

#### Scenario: A code excerpt links to its commit

- **WHEN** a site renders a code citation
- **THEN** the caption links to a GitHub permalink built from the manifest commit, the record `path`, and the record `lines`

#### Scenario: A snippet edit updates the page date

- **WHEN** a merged PR changes one prose snippet and no page file
- **THEN** the next push sets `lastUpdated` to the date of that commit in each page that imports the snippet, and leaves the other frontmatter fields as they are

#### Scenario: An older run finishes last

- **WHEN** the push job for commit A starts after the push for a later commit B is in codex
- **THEN** the job for A stops and codex keeps the content of B

### Requirement: A site builds pages from imports and renders citations from records

A consumer site SHALL build each hand-made page from its file, with the imported snippet files, and SHALL write no hand-made page. Only the view generator writes pages, into its generated directory (see the viewpoint requirement). The site SHALL resolve the `@snippets` import alias in each bundler it builds with. The site SHALL show each code citation as a code block with a link to its GitHub permalink. It SHALL show each diagram citation as a Mermaid block and each model citation as the component for its view. The build SHALL fail when an import or a citation names an id that has no file or record.

#### Scenario: A split page renders as before

- **WHEN** the migration tool splits an existing page, the text of each snippet goes in place of its import, each diagram goes back as a `mermaid` fence in place of its citation, and each `snippet:` link goes back to the route it replaced
- **THEN** the result matches the original page, apart from whitespace and the added anchors

#### Scenario: A snippet compiles alone

- **WHEN** the site build compiles the body of one prose snippet as MDX with no other snippet
- **THEN** the compile succeeds

#### Scenario: A second site reuses a snippet

- **WHEN** a second site imports a snippet file from this repository in its own page
- **THEN** its page shows the same body as the codex page, and no file holds a copy of the prose

### Requirement: The index uses aspects and edges, and a labelled set measures the result

The indexer SHALL put the aspect of a snippet in front of the text it embeds. The indexer SHALL also put there the titles of the snippets that the snippet `realises` or is `assessed-against`. On request, the retrieval server SHALL return with each hit the snippets that the hit reaches over `realises`, `mitigates`, and `assessed-against` edges. The team SHALL label a set of "why" questions and a set of developer questions with the chunks that a good answer needs. The team SHALL record a baseline on the content before migration. The team SHALL measure each of the two mechanisms on and off against the sets, by recall of labelled chunks. The cache key of an embedding SHALL be the hash of the full embedded text.

#### Scenario: A "why" question reaches the motivation

- **WHEN** a query asks why the client sends cover traffic, and the best vector hit is a design snippet
- **THEN** the result also holds the threat-model entity that the design snippet reaches over its edges

#### Scenario: A linked title changes

- **WHEN** the title of a snippet changes, and a second snippet has that title in its embedded context
- **THEN** the indexer embeds the second snippet again

#### Scenario: A mechanism that does not help

- **WHEN** the labelled set shows no better result with a mechanism on
- **THEN** the mechanism stays off, and the team records the result

### Requirement: Authored edges on generated records have a file

`.nym-codex/edges.json` SHALL list the edges from each record that has no frontmatter, as `{ from, to, rel }`. The check SHALL apply the same rules to these edges as to frontmatter edges. The tools SHALL write a coverage report with one line for each requirement record. The line SHALL state whether the record has an edge to a threat-model entity, and whether a code record has a `realises` edge to it.

#### Scenario: A code item realises a requirement

- **WHEN** `edges.json` holds an edge from a code id to a requirement id with `rel` `realises`
- **THEN** the code record has that edge in `refs`

#### Scenario: A requirement with no link

- **WHEN** a requirement record has no edge to a threat-model entity
- **THEN** the coverage report lists it, and the check passes until the linkage stage ends

### Requirement: Rules that the corpus does not meet yet start as warnings

The content does not meet four rules at migration. A section reads alone with no error. A requirement states its link. An inline source path exists. A person or a rule sets the aspect. For each of these rules, the check SHALL report a count and SHALL NOT fail until the stage that fixes the content ends. The check SHALL report each count on each run.

#### Scenario: A split page that nobody has rewritten

- **WHEN** the migration tool splits a page whose sections refer to text above them
- **THEN** the site renders the page as before, and the check reports those sections in the standalone count

#### Scenario: A stage ends

- **WHEN** the team closes the stage that fixes inline source paths
- **THEN** a path that does not exist fails the check from then on

### Requirement: Inline source paths exist

The tools SHALL check that each repository path in inline code in a prose body exists in the source repository.

#### Scenario: A file moves

- **WHEN** a PR moves a source file that a prose snippet names in inline code and does not update the snippet
- **THEN** the check reports the snippet and the path

### Requirement: A statement has a level and a status

The OpenSpec adapter SHALL emit one record for each requirement block, with the id `<repo>/openspec/<spec>/<slug of the requirement title>`. The threat-model adapter SHALL emit one record for each cell of the comparison matrix. Each such statement record SHALL have `meta.level` and `meta.status`. `meta.level` SHALL be the strongest RFC 2119 keyword in the statement, or absent for a threat-model claim. `meta.status` SHALL be `enforced` when the id is on the enforced list in `.nym-codex/manifest.json`, and `approved` in all other cases. The adapter SHALL fail when two requirements in one spec give one slug.

#### Scenario: A new requirement

- **WHEN** a PR adds a requirement block with a SHALL statement to an OpenSpec spec
- **THEN** the next build has a statement record with `level` `SHALL` and `status` `approved`

#### Scenario: A promotion

- **WHEN** a person adds a statement id to the enforced list
- **THEN** the record of that statement has `status` `enforced`, and no other record changes

### Requirement: A link names a snippet id

A link from a snippet body to other content of the knowledge base SHALL have the form `snippet:<id>`. A site build SHALL resolve it from the link maps to the route and anchor of the page that imports the target. The build SHALL fail when no page of that site imports the target. The record SHALL keep the id form.

#### Scenario: One link on two sites

- **WHEN** two sites import one snippet that links to a second snippet
- **AND** each site has the target on a page with a different route
- **THEN** each site renders the link with its own route

#### Scenario: A heading edit keeps a link

- **WHEN** an author changes the heading text of a snippet that other snippets link to
- **THEN** each link still resolves to the section

### Requirement: A viewpoint generates a view from records

A viewpoint SHALL be a data file in the target site that names a stakeholder and concerns. For each page, it SHALL give the records to select by aspect, kind, and facets, the edges to follow with their direction, and a template. Each facet value SHALL be on a closed list. The generator SHALL write the pages of each view from records and the link map, and SHALL set each heading level from the template. The generator SHALL use prompted text only for a section that the viewpoint marks as generated. The output of such a section SHALL be a prose record whose `meta.generated` holds the viewpoint, the input ids, and their hashes. The generator SHALL make that output again only when an input hash changes. For a section with no input, the generator SHALL state that the corpus has no evidence and SHALL NOT generate text.

#### Scenario: A view is deterministic

- **WHEN** the generator runs twice on one commit with no generated section in the viewpoint
- **THEN** the two sets of pages are byte-identical

#### Scenario: A generated section keeps its text

- **WHEN** the generator runs on a new commit, and no input of a generated section changed
- **THEN** the section keeps its stored text, and no new text needs review

#### Scenario: An input of a generated section changes

- **WHEN** a merged PR changes the body of a record that a generated section lists as input
- **THEN** the generator makes the section again and stores the new hashes in `meta.generated`

#### Scenario: A section with no evidence

- **WHEN** a viewpoint asks for the code that realises a requirement, and no code record has a `realises` edge to it
- **THEN** the view states that the corpus has no evidence for that requirement, and holds no generated text for it

#### Scenario: A snippet at a new level

- **WHEN** a viewpoint template puts a prose record under a level 2 heading of its own
- **THEN** the view shows the record title at level 3, and the hand-made page that imports the same snippet keeps its level

### Requirement: A procedure page passes the lint of its archetype

A page MAY state its Diataxis type in a `diataxis` frontmatter field, with one of the values `tutorial`, `how-to`, `reference`, and `explanation`. A page with `diataxis: how-to` SHALL follow the `runbook` archetype, and a page with `diataxis: tutorial` SHALL follow the `tutorial` archetype. The linter SHALL read each archetype and the placeholder list from data files, with one lint rule for each checked constraint. Before the check, the linter SHALL put each imported `.mdx` file in place of its tag, at each level. The linter SHALL list each TSX component on a how-to or tutorial page as unchecked. An unchecked component SHALL not be a fault. The linter SHALL run on the docs pages as they are now, before the split. In a runbook, each step SHALL be a depth 3 heading inside a `Steps` element, with no number in its text and with an explicit anchor. Each code block SHALL have a `lang` from the closed list of the archetype, and only the meta keys that the archetype allows. Each `<...>` token in a `code` or `inlineCode` node SHALL be a name in the placeholder list, unless it is a tag of the language of the block. The site SHALL render the placeholder table of `operators/variables.mdx` from the same list. A tutorial SHALL have a "Complete code" section.

#### Scenario: A step heading holds its number

- **WHEN** a how-to page holds `###### 1. Create a service file` inside `<Steps>`
- **THEN** the linter reports the depth and the number, and names the page and the line

#### Scenario: A fence uses a second name for shell

- **WHEN** a code block on a how-to page has the language `shell`
- **THEN** the linter reports it and names `sh` as the language to use

#### Scenario: A placeholder is not in the list

- **WHEN** a command holds `<OLD_NAME>`, and the placeholder list has it as an alias of `<NEW_NAME>`
- **THEN** the linter reports the token, names `<NEW_NAME>` as the fix, and names the page and the line

#### Scenario: A placeholder is in no entry

- **WHEN** a command holds a token that is no name and no alias in the placeholder list
- **THEN** the linter reports the token as unknown

#### Scenario: A procedure is in a nested import

- **WHEN** a how-to page imports an `.mdx` file, which imports a second `.mdx` file with a numbered step heading
- **THEN** the linter reports the step heading and names the second file

#### Scenario: A procedure is in a TSX component

- **WHEN** a how-to page renders `<RunTabs />` from a TSX module
- **THEN** the linter lists `RunTabs` as unchecked, and the result of the page does not change

#### Scenario: A page states no Diataxis type

- **WHEN** a page has no `diataxis` field
- **THEN** the linter checks no archetype on it

#### Scenario: A page states an unknown Diataxis type

- **WHEN** a page has `diataxis: guide`
- **THEN** the linter gives one warning for the value and checks no archetype on it

#### Scenario: A new archetype rule

- **WHEN** a rule joins the archetype and some pages do not pass it
- **THEN** the rule gives a warning with a count, and it fails the build only when all pages pass it

### Requirement: Private content does not reach codex

A public repository SHALL push to codex. No other repository SHALL push to codex. A private repository SHALL push to the private target and SHALL hold no credential that can write to codex.

#### Scenario: A private repository adopts the flow

- **WHEN** a private repository adds a `.nym-codex/` directory and the push job
- **THEN** its snippets go to the private target, and codex receives nothing from it

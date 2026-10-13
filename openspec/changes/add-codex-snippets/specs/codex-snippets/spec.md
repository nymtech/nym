# codex-snippets Spec Delta

## ADDED Requirements

### Requirement: Prose and diagram snippets are authored files

A prose snippet SHALL be one Markdown file under `.nym-codex/snippets/` with a `title` in its frontmatter and a body that contains no heading. A diagram snippet SHALL be one Mermaid `.mmd` file under the same directory. The id of each SHALL be `<repo>/<path under snippets/ with no extension>`. The tools SHALL fail when two snippets have one id.

#### Scenario: A title edit keeps the id

- **WHEN** an author changes the `title` of a prose snippet
- **THEN** the id does not change, and each recipe that lists the snippet stays valid

#### Scenario: A body with a heading is rejected

- **WHEN** a prose snippet body contains a line that is a Markdown heading outside a code fence
- **THEN** the PR check fails and names the file

#### Scenario: A prose snippet cites a prose snippet

- **WHEN** a prose snippet body contains `<Snippet id>` with the id of a prose snippet
- **THEN** the PR check fails and names the file

### Requirement: A typed model has one record for each entity

For a typed data model such as the threat model, the tools SHALL evaluate the source modules and emit one record of kind `model` for each entity, with an id of the form `<repo>/<model name>/<entity type>/<id in the data>`, the evaluated entity in `meta.data`, and a `body` that is a text projection of the same entity. The projection of a scenario SHALL state each route as an ordered list of hops, each verdict, and for each actor what it sees, what it cannot see, and the residual risk. This repository SHALL hold the one source of the threat model, and no consumer SHALL hold a second copy of the data.

#### Scenario: The index finds what a diagram shows

- **WHEN** the indexer reads the record of the `mixnet` scenario
- **THEN** the chunk text names each hop of the route in order and states what the destination sees and cannot see

#### Scenario: A worked example is indexed

- **WHEN** the tools build the records
- **THEN** each scenario, the L1 actor, and each invariant of the wallet, messaging, and browsing examples has a record

#### Scenario: A page shows a view of an entity

- **WHEN** a prose snippet cites a scenario with `view="diagram"` and a second one cites the same scenario with `view="assessment"`
- **THEN** the page renders the animated diagram and the assessment panel from one record

#### Scenario: An edit to an actor reaches each site

- **WHEN** a merged PR changes what the L2 actor observes
- **THEN** the next push changes one record, and the docs site and codex show the new text with no second edit

### Requirement: The threat model passes an integrity check

The PR check SHALL fail when a model entity references an id that does not exist, and when a scenario has no assessment for a spine actor and is not on the exception list. A verdict SHALL have one source in the data.

#### Scenario: A scenario omits an actor

- **WHEN** a PR adds a scenario with no assessment for L3G and does not add an exception
- **THEN** the PR check fails and names the scenario and the actor

### Requirement: Edges are typed

`refs` in a record SHALL be a list of `{ id, rel }`, sorted and with no duplicate. `rel` SHALL be a member of the closed relation list. The tools SHALL write a `cites` edge for each citation in a body, a `relates-to` edge for each `snippet:` link, and the model relations from the model data. An author MAY add edges in the frontmatter of a prose snippet as a plain id, which SHALL mean `relates-to`, or as an object with `id` and `rel`. The check SHALL fail on a relation that is not on the list and on an edge to an id that has no snippet.

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

### Requirement: A page is a recipe

A page SHALL be one JSON file under `.nym-codex/pages/` with a `title` and an ordered `sections` list, where each section has an `id` and an optional `level` from 2 to 4. The path of the recipe SHALL be the path of the page on the site. The tools SHALL fail when a section names an id that has no snippet. A snippet is in use when a recipe of its repository lists it, or when a prose snippet that such a recipe lists cites it. The tools SHALL fail when a prose or diagram snippet is not in use.

#### Scenario: One snippet at two depths

- **WHEN** two recipes list one prose snippet, the first with `level` 2 and the second with `level` 3
- **THEN** the first page renders the snippet title as a level 2 heading and the second as a level 3 heading, from one snippet file

#### Scenario: A section with no level

- **WHEN** a recipe lists a section with no `level`
- **THEN** the page renders the body with no heading

#### Scenario: A diagram is in use through a citation

- **WHEN** a prose snippet that a recipe lists cites a diagram snippet, and no recipe lists the diagram
- **THEN** the check passes

#### Scenario: A recipe names a snippet that does not exist

- **WHEN** a PR deletes a snippet file that a recipe still lists
- **THEN** the PR check fails and names the recipe

### Requirement: A snippet has one typed record

The tools SHALL emit each snippet as one `Snippet` record with the fields `id`, `kind`, `aspect`, `repo`, `path`, `title`, `body`, `hash`, `refs`, and `meta`, with `lang` for the code and diagram kinds and `lines` for the code kind. A record SHALL hold no URL, no page title, and no source commit. `kind` SHALL be one of `prose`, `code`, `diagram`, `model`. `aspect` SHALL be one of `motivation`, `requirement`, `design`, `implementation`, `operation`; the tools SHALL set it for the code and model kinds, and the check SHALL fail on a prose snippet that has none. `hash` SHALL be the SHA-256 of `body`. For the prose, diagram, and code kinds, `body` SHALL be the source text with no edit. For the model kind, `body` SHALL be the text projection.

#### Scenario: A code record carries its source location

- **WHEN** the tools emit a code snippet for `NymNetworkDetails` in `common/network-defaults/src/network.rs`
- **THEN** the record has `kind` `code`, `aspect` `implementation`, `lang` `rust`, that file as `path`, the first and last line of the item as `lines`, and the symbol as `title`

#### Scenario: A record maps to a retrieval chunk

- **WHEN** the indexer reads a prose record
- **THEN** it builds one `Chunk` from the record and the recipes of the site it serves, and makes no cut of its own, apart from the split of an oversized body

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

### Requirement: The source repository commits the code bodies and a lock file

The repository SHALL commit `.nym-codex/code/` (one `.txt` file for each cited code snippet) and `.nym-codex/snippets.lock` (one line for each snippet of each kind, with its hash and id). No committed file SHALL contain the source commit. A PR check SHALL regenerate both and write the result back to the branch, and SHALL fail on an error. An index walk over the repository tree SHALL exclude `.nym-codex/`, and the indexer SHALL NOT embed a code record as a chunk of its own.

#### Scenario: A code change shows its excerpt change

- **WHEN** a PR adds a field to a cited struct and regenerates
- **THEN** the PR diff shows the new field in the file under `.nym-codex/code/`

#### Scenario: Stale output is regenerated

- **WHEN** a PR changes a cited item and does not regenerate
- **THEN** the PR check commits the regenerated body file and lock file to the branch

#### Scenario: The output is deterministic

- **WHEN** the extractor runs twice on one commit
- **THEN** the two results are byte-identical

#### Scenario: A cited item is indexed once

- **WHEN** the code indexer walks the repository
- **THEN** it reads the source file of a cited item and reads no file under `.nym-codex/`

### Requirement: The push replaces the repository's directories in codex

On merge to the default branch, a job SHALL build the records at the merged commit, write that commit to `snippets/<repo>/manifest.json`, and replace `snippets/<repo>/` and `pages/<repo>/` in codex in full. The job SHALL write no other path in codex. Two runs SHALL NOT overlap, and a run SHALL stop when the commit in the codex manifest is not an ancestor of the commit it pushes.

#### Scenario: A deleted snippet leaves codex

- **WHEN** a merged PR deletes a snippet file and removes it from each recipe
- **THEN** the next push removes its record from codex

#### Scenario: A code excerpt links to its commit

- **WHEN** a site renders a code citation
- **THEN** the caption links to a GitHub permalink built from the manifest commit, the record `path`, and the record `lines`

#### Scenario: An older run finishes last

- **WHEN** the push job for commit A starts after the push for a later commit B is in codex
- **THEN** the job for A stops and codex keeps the content of B

### Requirement: A site builds its pages from recipes and records

A consumer site SHALL write one page for each recipe from the records, with each section heading at the level the recipe sets and with the last part of the snippet id as its anchor, each code citation as a code block with a link to its GitHub permalink, each diagram citation as a Mermaid block, and each model citation as the component for its view. The build SHALL fail when a recipe or a citation names an id that has no record, and when two recipes have one page path.

#### Scenario: A split page renders as before

- **WHEN** the migration tool splits an existing page and the site builds the page from the result
- **THEN** the page text matches the original page, apart from whitespace

#### Scenario: A snippet compiles alone

- **WHEN** the site build compiles the body of one prose snippet as MDX with no other snippet
- **THEN** the compile succeeds

#### Scenario: A second site reuses a snippet

- **WHEN** a second site lists the id of a snippet from this repository in its own recipe
- **THEN** its page shows the same body as the codex page, and no file holds a copy of the prose

### Requirement: The index uses aspects and edges, and a labelled set measures the result

The indexer SHALL put the aspect of a snippet, and the titles of the snippets it `realises` or is `assessed-against`, in front of the text it embeds. The retrieval server SHALL be able to return, with each hit, the snippets that the hit reaches over `realises`, `mitigates`, and `assessed-against` edges. Labelled sets of "why" questions and of developer questions SHALL exist, with the chunks that a good answer needs, and a baseline SHALL be recorded on the content before migration. Each of the two mechanisms SHALL be measured against the sets by recall of labelled chunks, with the mechanism on and off. The cache key of an embedding SHALL be the hash of the full embedded text.

#### Scenario: A "why" question reaches the motivation

- **WHEN** a query asks why the client sends cover traffic, and the best vector hit is a design snippet
- **THEN** the result also holds the threat-model entity that the design snippet reaches over its edges

#### Scenario: A linked title changes

- **WHEN** the title of a snippet changes, and a second snippet has that title in its embedded context
- **THEN** the indexer embeds the second snippet again

#### Scenario: A mechanism that does not help

- **WHEN** the labelled set shows no better result with a mechanism on
- **THEN** the mechanism stays off, and the result is recorded

### Requirement: Authored edges on generated records have a file

Edges from a record that has no frontmatter SHALL be listed in `.nym-codex/edges.json` as `{ from, to, rel }`. The check SHALL apply the same rules to these edges as to frontmatter edges. The tools SHALL write a coverage report that lists, for each requirement record, whether it has an edge to a threat-model entity and whether a code record has a `realises` edge to it.

#### Scenario: A code item realises a requirement

- **WHEN** `edges.json` holds an edge from a code id to a requirement id with `rel` `realises`
- **THEN** the code record has that edge in `refs`

#### Scenario: A requirement with no link

- **WHEN** a requirement record has no edge to a threat-model entity
- **THEN** the coverage report lists it, and the check passes until the linkage stage ends

### Requirement: Rules that the corpus does not meet yet start as warnings

For a rule that the existing content does not meet at migration (a section that does not read correctly alone, a requirement with no stated link, an inline source path that does not exist, an aspect set by default), the check SHALL report a count and SHALL NOT fail, until the stage that fixes the content ends. The check SHALL report each count on each run.

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

### Requirement: A link names a snippet id

A link from a snippet body to other content of the knowledge base SHALL have the form `snippet:<id>`. A site build SHALL resolve it to the route and anchor of the page that lists the target, and SHALL fail when no page of that site lists the target. The record SHALL keep the id form.

#### Scenario: One link on two sites

- **WHEN** two sites list one snippet that links to a second snippet, and each site has the target on a page with a different route
- **THEN** each site renders the link with its own route

#### Scenario: A title edit keeps a link

- **WHEN** an author changes the `title` of a snippet that other snippets link to
- **THEN** each link still resolves to the section

### Requirement: Private content does not reach codex

A repository SHALL push to codex if and only if it is public. A private repository SHALL push to the private target and SHALL hold no credential that can write to codex.

#### Scenario: A private repository adopts the flow

- **WHEN** a private repository adds a `.nym-codex/` directory and the push job
- **THEN** its snippets go to the private target, and codex receives nothing from it

# docs-site-surface Spec Delta

A browser-facing documentation surface for readers, built on the same corpus as
the retrieval index. This capability owns the *surface* (rendered pages, nav,
reference tables) and its access boundary; `docs-shared-index` owns the corpus
and the agent-facing MCP query endpoints. The two are sibling projections of one
set of upstream sources, not one downstream of the other.

Content sources (which READMEs, specs, and API descriptions seed which pages) are
deliberately not fixed here; they are stage-7 tasks. This delta states only the
invariants that hold regardless of content.

## ADDED Requirements

### Requirement: Surface serves a single visibility, fail-closed

A docs-site deployment SHALL serve content of exactly one visibility. A
public deployment SHALL render only `public`-visibility content. Content of a
higher visibility SHALL be served, if at all, by a separate access-protected
deployment, never by a public deployment that filters private pages out of its
output.

#### Scenario: Public surface cannot leak private content

- **WHEN** a page or reference table would be rendered from a source marked more
  restrictive than the deployment's visibility
- **THEN** the build excludes it, and the public deployment contains no path that
  serves it

#### Scenario: Private content is a separate deployment

- **WHEN** content of a restricted visibility must be published
- **THEN** it is a distinct deployment with its own access control, not a
  visibility filter applied to the public site

### Requirement: Access by deployment protection

A restricted docs-site deployment SHALL gate reader access through platform deployment
protection, distinct from the private MCP endpoint's per-person tokens. This is
sufficient because a correctly built surface carries no content above its
deployment's visibility.

#### Scenario: Unauthenticated reader

- **WHEN** a reader without deployment access requests any page of a protected
  docs site surface
- **THEN** the request is refused before any page is served

### Requirement: Content surfaces are verified projections

Every content-bearing surface derived from a typed source (navigation, reference
tables, capability or verdict matrices) SHALL be a projection of that source, and
a build-time invariant test SHALL fail when a rendered surface disagrees with the
projection of its source. Judgement prose authored by a person is exempt and is
reviewed by a person.

#### Scenario: Source changes, surface follows

- **WHEN** a typed source entry that feeds a derived surface changes
- **THEN** the rendered surface changes with it on the next build, with no
  separate hand edit

#### Scenario: Drift is caught at build time

- **WHEN** a derived surface is hand-edited to disagree with its source
- **THEN** the projection invariant test fails the build

### Requirement: One home per fact

A fact that appears on the docs site surface and in the retrieval index SHALL have a
single upstream home in the corpus. Neither surface SHALL carry a hand-copied
duplicate of a fact the other also states.

#### Scenario: Shared fact edited once

- **WHEN** a fact shown on the docs site surface and returned by the index changes
- **THEN** it is edited once at its corpus home and both surfaces reflect the
  change on their next build

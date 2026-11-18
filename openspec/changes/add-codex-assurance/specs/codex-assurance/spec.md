# codex-assurance Spec Delta

## ADDED Requirements

### Requirement: Each record carries a warrant

The record builder SHALL set `meta.warrant` on each record to one of `checked`, `entailed`, `asserted`, or `narrative`. A code excerpt cited by symbol SHALL be `checked`. A requirement whose statement has a witness, a canary, and a canary date SHALL be `checked`. A matrix verdict computed by the rules SHALL be `entailed`. A matrix verdict typed by hand, a threat-model entity, and a requirement that lacks a witness, a canary, or a canary date SHALL be `asserted`. Other prose SHALL be `narrative`. The warrant SHALL NOT change the `authority` of the record.

#### Scenario: A requirement gains a witness

- **WHEN** a statement record for an OpenSpec requirement gains a `meta.witness`, and a `meta.canary` with the date of a mutation run that turned the witness red
- **THEN** its warrant changes from `asserted` to `checked`, and its `authority` stays `normative`

#### Scenario: A hand-typed verdict

- **WHEN** the adapter reads a matrix verdict that a person typed
- **THEN** the verdict record is `asserted`

### Requirement: An enforced statement has a witness and a canary

A statement record SHALL accept `meta.witness` (the id of a test, a check, or a code record) and `meta.canary` (one sentence that names the change to the source that must turn the witness red, and the date of the last mutation run that saw it red). The tools SHALL refuse to promote a statement with the level `MUST` or `SHALL` to `enforced` when it has no witness, no canary, or no canary date. The coverage report SHALL list each statement with no witness, and each statement whose witness has no canary.

#### Scenario: Promotion with no canary

- **WHEN** a person adds the id of a `MUST` statement with no canary to the enforced list in `.nym-codex/manifest.json`
- **THEN** the records check fails and names the statement

#### Scenario: The report shows a witness that cannot fail

- **WHEN** a statement has a witness and no canary
- **THEN** the coverage report lists it under "witness with no canary"

### Requirement: An obligation has an owner

The tools SHALL write an obligation record when a person accepts a `contradicts` verdict, and when a `normative` record has no witness after the period that the manifest sets. An obligation record SHALL be a prose record with `meta.obligation` holding `owner`, `opened`, and `closesWhen`, and SHALL hold a `relates-to` edge to each commitment it concerns. The records check SHALL fail on an obligation record with an empty `owner`.

#### Scenario: An accepted contradiction

- **WHEN** a person accepts a `contradicts` verdict that cites two statements
- **THEN** the tools write one obligation record with `relates-to` edges to both statements, and the records check fails until it has an owner

### Requirement: A viewpoint states the record kinds that each concern needs

A viewpoint file SHALL list its concerns as questions. For each concern, it SHALL name the record kinds and facets that an answer needs. The generator SHALL write a sufficiency report for each view: for each concern, whether its selection is empty, and a place for a person to mark it as answered, partly answered, or not answered.

#### Scenario: A concern with no records

- **WHEN** the selection for a concern is empty at the commit that the view uses
- **THEN** the sufficiency report marks the concern as "no evidence in the corpus", and the view section for it says so

### Requirement: The assurance view lists open gaps before guarantees that look complete

The `assurance` view SHALL contain a section that lists requirements with no code that realises them, properties with no requirement that realises them, statements that are `approved` and not `enforced`, and open obligations. A section of the view with no input records SHALL state that the corpus has no evidence for it, and the generator SHALL write no other text for that section.

#### Scenario: A property with no requirement

- **WHEN** a threat-model property has no `realises` edge from a requirement
- **THEN** the open gaps section lists the property, and the guarantees section does not list it as realised

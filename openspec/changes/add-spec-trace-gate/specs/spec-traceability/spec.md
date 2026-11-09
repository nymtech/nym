## ADDED Requirements

### Requirement: Code links to a requirement come from markers only

The links from code to a requirement in the scope SHALL come from Tracey markers in the code: `// r[impl <id>]` on the line above the item that keeps the requirement, and `// r[verify <id>]` on the line above the test that checks it. The docs records and the coverage report SHALL take their `realises` edges and witnesses from these markers. The records check SHALL fail on a hand-written `realises` edge from code to a requirement in the scope. The edges from a requirement to the threat model SHALL stay hand-written.

#### Scenario: A renamed function keeps its link

- **WHEN** a commit renames a function that carries `// r[impl sphinx.ack-packet]` on the line above it
- **THEN** the records still have a `realises` edge from the renamed function to `sphinx.ack-packet`

#### Scenario: A hand-written code edge is rejected

- **WHEN** `.nym-codex/edges.json` has a `realises` edge from a code item to a requirement in the scope
- **THEN** the records check fails and names the edge

### Requirement: The gate fails a pull request with a broken reference

A CI job SHALL run on each pull request that changes a spec in the scope, the Tracey config, or Rust code. It SHALL fail when a marker names an id that no spec in the config defines, when two requirements have the same id, or when a marker names an old version of a requirement. The job SHALL be a required check on `develop`.

#### Scenario: A marker names an unknown id

- **WHEN** a pull request adds `// r[impl sphinx.size-clases]` and no spec defines `sphinx.size-clases`
- **THEN** the job fails and names the file and line of the marker

#### Scenario: A marker names an old version

- **WHEN** a spec changes `r[sphinx.route]` to `r[sphinx.route+2]` and a marker still names `sphinx.route`
- **THEN** the job fails and names the marker

### Requirement: The gate reports a gap without failing

The job SHALL report each requirement in the scope that has no `impl` marker, and each that has no `verify` marker. It SHALL NOT fail because of them.

#### Scenario: A spec gains ids before its markers

- **WHEN** a pull request adds Tracey ids to `sdk-mixnet-stream` and no code names them yet
- **THEN** the job passes and reports each new requirement as having no code and no test

### Requirement: The gate covers the specs in the scope and every path of their markers

The Tracey config SHALL list the specs `sphinx-packet-format`, `client-cover-traffic`, `mix-node-delay`, `sdk-mixnet-stream`, `nym-network-monitor`, `network-monitor-config-score` and `network-monitor-epoch-performance`. For each spec, it SHALL list every code path that holds a marker for that spec. When `add-mixnet-core-specs` is archived, the config SHALL name the archived spec files in the same commit.

#### Scenario: A marker in the gateway counts

- **WHEN** the job runs and `gateway/` holds `// r[impl delay.gateways-no-delay]`
- **THEN** the report counts `delay.gateways-no-delay` as having code

#### Scenario: The core specs are archived

- **WHEN** a commit archives `add-mixnet-core-specs`
- **THEN** the same commit changes the config to the three files in `openspec/specs/`, and the job finds all 21 core ids

### Requirement: The gate fails on its own canary

A test SHALL run the gate command on a fixture that holds a marker with an unknown id and a requirement with no marker. The test SHALL pass only when the gate fails on the marker and reports the requirement without failing on it.

#### Scenario: The gate stops catching unknown ids

- **WHEN** a change to the gate command or its config makes it pass a marker with an unknown id
- **THEN** the canary test fails

### Requirement: A commit that changes marked code prints the requirements it touches

A lefthook pre-commit job SHALL find each staged item whose lines changed and that carries a marker, and SHALL print the id, the verb and the first sentence of each requirement that the item names. The job SHALL exit 0 in each case, including when `node` is missing.

#### Scenario: A commit changes a marked function

- **WHEN** a commit changes the body of `SurbAck::construct_legacy`, which carries markers for `sphinx.ack-packet` and `delay.acks-replies-cover`
- **THEN** the job prints both ids with the first sentence of each requirement, and the commit goes ahead

#### Scenario: A commit changes no marked code

- **WHEN** a commit changes only items that carry no marker
- **THEN** the job prints nothing

### Requirement: Ids in an archived spec do not change its requirements

A Tracey id SHALL be added to a requirement in an archived spec as one `r[<id>]` line between the `### Requirement:` heading and the requirement text. The text of the requirement SHALL stay the same, and `openspec validate --specs --strict` SHALL accept the spec.

#### Scenario: Ids for the SDK spec

- **WHEN** each of the 12 requirements of `sdk-mixnet-stream` gains an `r[sdk.<name>]` line
- **THEN** the diff of the spec has only added id lines, and strict validation passes for the spec

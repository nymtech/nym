## ADDED Requirements

### Requirement: Code links to a requirement come from markers only

The links from code to a requirement in the scope SHALL come from Tracey markers in the code: `// r[impl <id>]` on the line above the item that keeps the requirement, and `// r[verify <id>]` on the line above the test that checks it. The coverage report SHALL take the code and the tests of each such requirement from Tracey. The records check SHALL fail on a hand-written `realises` edge from code to a requirement in the scope. The edges from a requirement to the threat model SHALL stay hand-written.

#### Scenario: A renamed function keeps its link

- **WHEN** a commit renames a function that carries `// r[impl sphinx.ack-packet]` on the line above it
- **THEN** the records still have a `realises` edge from the renamed function to `sphinx.ack-packet`

#### Scenario: A hand-written code edge is rejected

- **WHEN** `.nym-codex/edges.json` has a `realises` edge from a code item to a requirement in the scope
- **THEN** the records check fails and names the edge

### Requirement: The gate fails a pull request with a broken reference

A CI job SHALL run Tracey on each pull request. It SHALL read only the spec files and the code paths that the Tracey config names. It SHALL fail when a marker names an id that no spec in the config defines, when two requirements have the same id, or when a marker names an old version of a requirement. The job SHALL be a required check on `develop`.

#### Scenario: A marker names an unknown id

- **WHEN** a pull request adds `// r[impl sphinx.size-clases]` and no spec defines `sphinx.size-clases`
- **THEN** the job fails and names the file and line of the marker

#### Scenario: Two specs define one id

- **WHEN** a pull request adds `r[delay.node-holds]` to a second spec in the scope
- **THEN** the job fails on the `CONFIG ERROR` that Tracey prints, although Tracey exits 0

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

After the gate passes on the repository, the CI job SHALL add a marker with an unknown id to a code path in its own checkout, and run the gate again. The job SHALL fail unless the second run fails and names the planted id.

#### Scenario: The gate stops catching unknown ids

- **WHEN** a change to the gate command or its config makes it pass a marker with an unknown id
- **THEN** the canary step fails

### Requirement: A commit runs the reference check first

A lefthook pre-commit job on staged Rust files SHALL run the Tracey reference check and print its result. The job SHALL exit 0 in each case, including when `tracey` is missing.

#### Scenario: A commit adds a broken marker

- **WHEN** a commit adds `// r[impl sphinx.size-clases]`
- **THEN** the job prints the unknown reference, and the commit goes ahead

### Requirement: Ids in an archived spec do not change its requirements

A Tracey id SHALL be added to a requirement in an archived spec as one `r[<id>]` line between the `### Requirement:` heading and the requirement text. The text of the requirement SHALL stay the same, and `openspec validate --specs --strict` SHALL accept the spec.

#### Scenario: Ids for the SDK spec

- **WHEN** each of the 12 requirements of `sdk-mixnet-stream` gains an `r[sdk.<name>]` line
- **THEN** the diff of the spec has only added id lines, and strict validation passes for the spec

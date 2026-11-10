# Tracey and the mixnet core specs

The OpenSpec change `openspec/changes/add-mixnet-core-specs/` writes down what the mixnet's core code promises: the Sphinx packet format, client cover traffic, and the delay that a mix node adds to each packet. Comments in the code link each promise to the code that keeps it and to the test that checks it. [Tracey](https://github.com/bearcove/tracey) reads those comments and reports which promises have code and which have a test. This page explains how the pieces relate and how to run the checks.

## What the branch shows as a PoC

Claim: you can write the mixnet's privacy promises down as short requirements. Then a machine can link each one to the code that keeps it, to a test that checks it, and to the threat-model entry it serves. Once the links exist, the gaps can be counted.

| What it shows | Evidence on the branch |
|---|---|
| The three core mechanisms can be specified from the code as it is | 21 requirements (Sphinx 7, cover 8, delay 6). Each one comes from code that was read twice |
| The specs can say what they do not promise | `design.md`: what each party sees, 14 conditions under which a protection is absent, the non-goals |
| Code-to-spec links can live in the code | 60 Tracey markers. Tracey reads them, and the docs coverage report asks Tracey for them |
| The gap can be measured | 20 of 21 requirements have code, but only 5 of 21 have a test. With the proposed edges applied, the chains from code to the threat model go from 0 to 12 |
| Writing specs finds real faults | 5 findings, for example the replay filter comment that says the opposite of what the code does |

## How the pieces relate

```
  spec.md (openspec/changes/add-mixnet-core-specs/specs/)
  +-----------------------------------------------+
  | r[sphinx.size-classes]                        |
  | ### Requirement: Every packet of one size     |
  | class has the same length                     |
  +-----------------------------------------------+
        ^                       ^                      |
        | // r[impl ...]        | // r[verify ...]     | realises / mitigates
        |                       |                      | (edges.proposed.json,
        |                       |                      |  reviewed by the
  +---------------+     +------------------+          |  threat-model owner)
  | code item     |     | witness test     |          v
  | packet_sizes  |     | packet_size_     |    +---------------------+
  | .rs           |     | totals           |    | threat-model entry  |
  +---------------+     +------------------+    | (V2, P1)            |
                                                +---------------------+

  Tracey is the only reader of the markers:

  markers --> tracey query validate ----------------> CI gate: unknown id? old version?
         \--> tracey query --json rule <id> --+
                                               +--> lib/snippets/coverage.mts
  edges.proposed.json (requirement -> threat) -+    code -> requirement -> threat model
```

A requirement in a spec file has a Tracey id on the line above its heading. A comment above a function or a test names that id with a verb. Tracey reads the comments, and the coverage report and an agent ask Tracey, so they all see the same links. The link from a requirement to the threat model is not in the code. It is a proposed edge that the threat-model owner reviews.

## The markers

| Part | Syntax | Meaning |
|---|---|---|
| A requirement, in a spec file | `r[sphinx.size-classes]` on its own line, above the requirement text | Defines the requirement. A version suffix (`r[sphinx.size-classes+2]`) marks a change to its text |
| A reference, in a code comment | `// r[impl sphinx.size-classes]` | This code implements the requirement |
| | `// r[verify sphinx.size-classes]` | This test checks the requirement |
| | `// r[related sphinx.replay-drop]` | This code is related, without implementing or checking it |
| The config | `.config/tracey/config.styx` | Which files hold specs, and which hold code |

A marker sits above the code it names, so it moves with the code when the code moves or is renamed. A marker shows where the code is. It does not show what the code does: only a `verify` test that fails when the behaviour changes guards the behaviour.

## How to run it

1. Install Tracey 1.4.0 from its [release page](https://github.com/bearcove/tracey/releases/tag/v1.4.0): download `tracey-x86_64-unknown-linux-gnu.tar.xz` and its `.sha256` file, check it with `sha256sum -c`, and put the binary on your `PATH`. `cargo install tracey` (1.3.0) fails to build without `--locked`.
2. From the repository root, run `tracey query status` for the coverage, and `tracey query validate --deny warnings` for broken references. For an id that two specs define, Tracey 1.4.0 prints `CONFIG ERROR` and exits 0, so read the output.
3. Run `tracey web --open` for a dashboard of the specs with their coverage.
4. A lefthook pre-commit job runs `tracey query validate --deny warnings` when `tracey` is on your `PATH`. It prints the result and never fails the commit.
5. For an agent, run `tracey ai --claude` (or `--codex`) once. It registers the `tracey mcp` server with the agent and installs the Tracey skill. The agent can then ask `tracey_rule` for the code and tests of a requirement, and `tracey_validate` for broken references. The docs MCP on the docs site does not hold these links.
6. In the editor, `tracey lsp` shows the requirement that a marker names. After a spec moves to `+2`, `tracey query stale` lists each marker whose code you must check against the new text.

## In CI

The `ci-spec-trace` workflow runs `tracey query validate --deny warnings` on each pull request, and also fails when the output holds `CONFIG ERROR`: Tracey 1.4.0 reports an id that two specs define that way, and exits 0. Before that, the workflow plants a marker with an unknown id in its checkout and checks that Tracey fails on it. Then it removes the marker and runs `tracey kill`, because a running daemon does not see a new file at once.

Tracey 1.4.0 lets a marker with no code item below it pass, and nothing else checks it.

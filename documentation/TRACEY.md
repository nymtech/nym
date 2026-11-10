# Tracey: links from specs to code

Some OpenSpec requirements say what the mixnet's core code promises, for example the Sphinx packet format, client cover traffic and the delay that a mix node adds to each packet. Comments in the code link each requirement to the code that keeps it and to the test that checks it. [Tracey](https://github.com/bearcove/tracey) reads those comments and reports which requirements have code and which have a test. This page explains how the pieces relate and how to run the checks.

## What you can ask

| Question | Command |
|---|---|
| How many requirements have code, and how many have a test? | `tracey query status` |
| Which requirements have no code, or no test? | `tracey query uncovered`, `tracey query untested` |
| Which code and which tests keep this requirement? | `tracey query rule <id>` |
| Does a marker name an id that no spec defines, or an old version? | `tracey query validate --deny warnings` |
| The spec text changed. Which code must I check against the new text? | `tracey query stale` |
| Which requirements link code to the threat model? | `node lib/snippets/coverage.mts` in `documentation/docs` |

## How the pieces relate

```
  spec.md (a file that .config/tracey/config.styx names)
  +-----------------------------------------------+
  | ### Requirement: Every packet of one size     |
  | class has the same length                     |
  | r[sphinx.size-classes]                        |
  +-----------------------------------------------+
        ^                       ^                      |
        | // r[impl ...]        | // r[verify ...]     | realises / mitigates
        |                       |                      | (an edge that the
        |                       |                      |  threat-model owner
  +---------------+     +------------------+          |  reviews)
  | code item     |     | witness test     |          v
  | packet_sizes  |     | packet_size_     |    +---------------------+
  | .rs           |     | totals           |    | threat-model entry  |
  +---------------+     +------------------+    | (V2, P1)            |
                                                +---------------------+

  Tracey is the only reader of the markers:

  markers --> tracey query validate ----------------> CI gate: unknown id? old version?
         \--> tracey query --json rule <id> --+
                                               +--> lib/snippets/coverage.mts
  edges (requirement -> threat model) ---------+    code -> requirement -> threat model
```

A requirement in a spec file has a Tracey id on the line below its heading. A comment above a function or a test names that id with a verb. Tracey reads the comments, and the coverage report and an agent ask Tracey, so they all see the same links. The link from a requirement to the threat model is not in the code. It is an edge in `documentation/docs/.nym-codex/edges.json`, which the threat-model owner reviews.

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

1. Install the Tracey version that `TRACEY_VERSION` in `.github/workflows/ci-spec-trace.yml` names, from the [release page](https://github.com/bearcove/tracey/releases): download `tracey-x86_64-unknown-linux-gnu.tar.xz` and its `.sha256` file, check it with `sha256sum -c`, and put the binary on your `PATH`.
2. From the repository root, run the commands in "What you can ask". For an id that two specs define, Tracey prints `CONFIG ERROR` and exits 0, so read the output.
3. Run `tracey web --open` for a dashboard of the specs with their coverage.
4. A lefthook pre-commit job runs `tracey query validate --deny warnings` when `tracey` is on your `PATH`. It prints the result and never fails the commit.
5. For an agent, run `tracey ai --claude` (or `--codex`) once. It registers the `tracey mcp` server with the agent and installs the Tracey skill. The agent can then ask `tracey_rule` for the code and tests of a requirement, and `tracey_validate` for broken references. The docs MCP on the docs site does not hold these links.
6. In the editor, `tracey lsp` shows the requirement that a marker names.

## In CI

The `ci-spec-trace` workflow runs on each pull request. It first plants a marker with an unknown id in its checkout and checks that Tracey fails on it. Then it removes the marker and runs `tracey kill`, because a running daemon does not see a new file at once. Then it runs `tracey query validate --deny warnings`, and also fails when the output holds `CONFIG ERROR`. Last, it lists the requirements with no code or no test, and does not fail on them.

Tracey does not check that a marker has a code item below it.

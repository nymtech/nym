# Tracey and the mixnet core specs

This branch (`max/mixnet-core-specs`) is an experiment. It writes down what the mixnet's core code promises, links each promise to the code that keeps it and the test that checks it, and uses [Tracey](https://github.com/bearcove/tracey) to show which promises are covered. This page explains it for three readers: anyone (the short version), engineers (what Tracey is and what is on the branch), and whoever wants to use it (what we can do with it).

## The short version

The mixnet makes privacy promises. Packets all look the same size. Clients send fake traffic when they are idle. Mix nodes hold each packet for a random time. Until now, these promises lived in people's heads and in prose. Nothing in the code said "this function keeps that promise", so nobody could ask "which promises have no code behind them?" or "which promises have no test?".

This branch writes each promise down as a short numbered requirement. A one-line comment above a function then says "this code keeps promise `sphinx.size-classes`", and a comment above a test says "this test checks it". Tracey reads those comments and answers the two questions in one command. When someone later changes a promise, Tracey also flags every piece of code that still points at the old wording.

## What Tracey is

Tracey is an open-source tool (Apache-2.0, Rust, by bearcove) that measures how far code covers a written specification. It has three parts:

| Part | Syntax | Meaning |
|---|---|---|
| A requirement, in a spec file | `r[sphinx.size-classes]` on its own line, above the requirement text | Defines the requirement. A version suffix (`r[sphinx.size-classes+2]`) marks a change to its text |
| A reference, in a code comment | `// r[impl sphinx.size-classes]` | This code implements the requirement |
| | `// r[verify sphinx.size-classes]` | This test checks the requirement |
| | `// r[related sphinx.replay-drop]` | This code is related, without implementing or checking it |
| The config | `.config/tracey/config.styx` | Which files hold specs, and which hold code |

From those, Tracey reports coverage (which requirements have code), test coverage (which have a test), stale references (code that points at an old version of a requirement), and code that no requirement covers. It works as a CLI, a web dashboard, a language server for editors, and an MCP server for AI coding agents.

## What the branch experiments with

| Piece | What it is | Where |
|---|---|---|
| The specs | 21 requirements for the Sphinx packet format (7), client cover traffic (8), and the delay a mix node adds (6). Each describes the code as it is, with the conditions under which a protection is absent | `openspec/changes/add-mixnet-core-specs/` |
| Tracey ids | One `r[...]` id per requirement, for example `delay.per-hop-exponential` | The three `spec.md` files |
| `impl` markers | One comment above each function that realises a requirement: 50 in all, across `nymsphinx`, `client-core`, `topology`, the mixnet client library, `nym-node`, the gateway and the registration client | The Rust code |
| Witness tests | Tests for 5 of the requirements, each marked with `verify`. One more test is marked `related`: it pins a behaviour that a code comment contradicts | `packet_sizes.rs`, `routing/src/lib.rs`, `config-types/src/lib.rs`, `replay_protection/bloomfilter.rs` |
| The codex reader | Reads the same markers into the docs records, so the docs, the coverage report and agents see the same links | `documentation/docs/lib/snippets/tracey.mts` |
| The assurance extension | What backs each claim, a test that must be able to fail before a claim is enforced, and an assurance view for auditors | `openspec/changes/add-codex-assurance/` |

## What we found

| Check | Result |
|---|---|
| Real Tracey 1.4.0 on the whole branch (`tracey query status`) | 20 of 21 requirements covered, 5 of 21 with a test. `tracey query validate`: 0 errors |
| The codex reader against the hand-written links (`edges.proposed.json`) | 50 of 50 reproduced, 0 missed |
| Requirements with code in this repository | 20 of 21. The 21st (how the delay is encoded in the header) lives in the `sphinx-packet` crate |
| Requirements with a test | 5 of 21 |
| Things to check in the code, found while writing the specs | 5: a replay-filter comment that contradicts its code, two settings that nothing reads, a stale comment on the default delay, and hop delays that ignore the injected random generator |

The gap that matters is the second-to-last row. Most of the mixnet's privacy promises have code, and few have a test that would fail if the code stopped keeping them.

## What we can do with it

| Who | Command | What it answers |
|---|---|---|
| Anyone | `tracey query status` | How many requirements have code, and how many have a test |
| An engineer before a change | `tracey query rule delay.node-holds` | Which code and tests depend on this requirement |
| A reviewer | `tracey query untested` | Which promises have code but no test |
| After a spec change | `tracey query stale` | Which code still points at the old wording |
| In an editor | `tracey lsp` | Coverage inline, and a jump from a reference to its requirement |
| In a browser | `tracey web --open` | A dashboard: specs with their coverage, and code with its references |
| An AI coding agent | `tracey ai` | Registers the MCP server, so the agent can ask which requirements a change touches |
| In CI | `tracey query validate --deny warnings` | Fails on broken or duplicate references |
| The docs | `node lib/snippets/tracey.mts`, then `node lib/snippets/coverage.mts` | The same links as docs records: the coverage report, and the chain from code to the threat model |

## How to try it

1. Install Tracey 1.4.0 from its [release page](https://github.com/bearcove/tracey/releases/tag/v1.4.0): download `tracey-x86_64-unknown-linux-gnu.tar.xz` and its `.sha256` file, check it with `sha256sum -c`, and put the binary on your `PATH`. `cargo install tracey` (1.3.0) fails to build without `--locked`.
2. Check out `max/mixnet-core-specs`.
3. Run `tracey query status`, then `tracey web --open`.

## What comes next

1. Tests for the requirements a reader relies on most. The tests for the three-mix route and the cover slot are optional.
2. Answers from the code owners to the five findings, in `openspec/changes/add-mixnet-core-specs/design.md`.
3. A review of each proposed threat-model link, by the threat-model owner.
4. Tracey's versions as the staleness signal for the docs: when a requirement's text changes, the witness against the old version stops counting.
5. A trace widget for the docs: click a sentence, and step to its requirement, code, test, threat-model property and source paper.

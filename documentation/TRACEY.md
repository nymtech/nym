# Tracey: links from specs to code

## In short

A spec is a list of promises. "Every packet of one size class has the same length" is one promise. Each promise has a short name, for example `sphinx.size-classes`.

A programmer writes that name in a comment above the code that keeps the promise. They write it again above the test that checks the promise. Tracey is a tool that reads those comments. For each promise, it tells you where the code is, where the test is, or that one of them is not there.

CI asks Tracey on each pull request. The check fails if a promise has no code or no test. It also fails if a comment names a promise that does not exist.

```
  the promise, in a spec        the code                        the test
  r[sphinx.size-classes]        // r[impl sphinx.size-classes]   // r[verify sphinx.size-classes]
           ^                               |                                |
           +-------------------------------+--------------------------------+
                        Tracey reads all three and matches the names
```

## What Tracey is

[Tracey](https://github.com/bearcove/tracey) is a requirements trace tool. It is one binary, `tracey`. It reads two kinds of files, and its config names both:

- Spec files in Markdown. Each requirement in them has an id.
- Code files. Comments in them name those ids.

Tracey matches the comments to the ids and answers queries about the result. The first query starts a Tracey daemon. The daemon keeps an index of the files and answers the next queries from that index. The command line, a web dashboard, an editor (LSP) and an agent (MCP) all use the same index.

Tracey reads only the ids and the comments. It does not compile or run the code, and it does not know what the code does.

## What we use it for

| Use | How |
|---|---|
| Show the code that keeps each requirement of the mixnet core | `impl` markers in the code |
| Show the test that checks each requirement | `verify` markers above the tests |
| Fail a pull request that breaks a link | The `ci-spec-trace` workflow runs `tracey query validate` |
| Fail a pull request that adds a requirement with no code or no test | The workflow runs `tools/internal/spec-trace-gaps.sh` |
| Show the code to check after a change to a spec | A version suffix on the id, and `tracey query stale` |
| Give the code links to the coverage report | `documentation/docs/lib/snippets/coverage.mts` calls `tracey query --json rule` |
| Let an agent find the code of a requirement before it edits | `tracey mcp` |

The scope is the specs that `.config/tracey/config.styx` names. Today these are the three mixnet core specs: the Sphinx packet format, client cover traffic and the mix-node delay. Other specs have no ids yet, and Tracey does not read them.

A spec with no link to the code goes out of date, and nobody sees the change. A marker puts the link in the code, one line above the item. When you move or rename the code, the marker moves with it. When you change a spec, Tracey shows the code that you must check.

Tracey does not prove that the code is correct. Only a `verify` test that fails when the behaviour changes guards the behaviour. Tracey shows that such a test exists and that it names the correct requirement.

## How it works

```
                    .config/tracey/config.styx
              names the spec files and the code files
                     |                          |
                     v                          v
   +-----------------------------+   +------------------------------------+
   | spec files (Markdown)       |   | code files (Rust)                  |
   |                             |   |                                    |
   | ### Requirement: Every ...  |   | // r[impl sphinx.size-classes]     |
   | r[sphinx.size-classes]      |   | pub enum PacketSize { ... }        |
   | The client SHALL ...        |   |                                    |
   |                             |   | // r[verify sphinx.size-classes]   |
   |                             |   | #[test] fn packet_size_totals()    |
   +-----------------------------+   +------------------------------------+
                     |                          |
                     +------------+-------------+
                                  v
                     +--------------------------+
                     | Tracey daemon            |
                     | id -> impl references    |
                     |    -> verify references  |
                     +--------------------------+
                       |       |       |       |
                       v       v       v       v
                     CLI      web     LSP     MCP
                 tracey query ...
                       |
           +-----------+------------------------+
           v                                    v
   ci-spec-trace (CI)                    coverage.mts
   validate, then the gaps script        code -> requirement -> threat model
```

1. Tracey reads the config. The config has a list of spec files, and for each spec a list of code files.
2. Tracey reads each spec file. Each `r[id]` line starts a requirement.
3. Tracey reads each code file. Each comment of the form `r[<verb> <id>]` is a reference.
4. Tracey matches each reference to a requirement by its id.
5. A query reads the result: the code and the tests of one requirement, the requirements with no code, or the references with an unknown id.

The link from a requirement to the threat model is not in the code. It is an edge in `documentation/docs/.nym-codex/edges.json`, and the threat-model owner reviews it. `coverage.mts` joins the two: it gets the code links from Tracey and the threat-model links from the edges.

## Tracey on specs

A requirement in an OpenSpec spec gets its id on its own line, directly below the `### Requirement:` heading. The id line comes before the requirement text and before the first scenario.

```markdown
### Requirement: Every packet of one size class has the same length

r[sphinx.size-classes]
The client SHALL build each Sphinx packet in one of five size classes. ...

#### Scenario: ...
```

| Rule | Reason |
|---|---|
| Give each requirement one id. Each id is unique across all the specs. | If two specs define the same id, Tracey prints `CONFIG ERROR`, and the CI gate fails. |
| Put the id line before the first `####` scenario heading. | The gaps script uses this position to find a requirement with no id. |
| In a spec that has ids, give every requirement an id. | Tracey cannot see a requirement with no id. The gaps script finds it and fails. |
| Use the form `<area>.<name>`, in lower case, with dashes. | The area shows the spec: `sphinx.`, `cover.` or `delay.`. |
| Add the spec file to `include` in `.config/tracey/config.styx`. | Tracey reads only the spec files that the config names. |

### Change what a requirement says

When you change the meaning of a requirement, add a version suffix to its id, or raise the suffix. Tracey then marks each reference to the old version as stale.

```
  before the change                    after the change to the text

  spec:  r[sphinx.size-classes]        spec:  r[sphinx.size-classes+2]
  code:  // r[impl sphinx.size-classes]       // r[impl sphinx.size-classes]      <- stale
                                                     |
                                       tracey query stale lists it.
                                       You check the code against the new text,
                                       then change the marker:
                                              // r[impl sphinx.size-classes+2]
```

A change to the words that keeps the meaning needs no version suffix.

### Archive a change

While an OpenSpec change is open, its specs are in `openspec/changes/<change>/specs/`. When you archive the change, OpenSpec moves them to `openspec/specs/<capability>/spec.md` and keeps the `r[id]` lines. In the same commit, change the spec `include` path in `.config/tracey/config.styx` to the new paths. If you do not, every marker names an unknown id, and the CI gate fails.

## Tracey on code

| Marker | Meaning |
|---|---|
| `// r[impl <id>]` | This code implements the requirement. |
| `// r[verify <id>]` | This test checks the requirement. |
| `// r[related <id>]` | This code is related to the requirement. Tracey does not count it as code or as a test of the requirement. |

Put the marker on the line directly above the item that it names. If the item has attributes, put the marker above them.

```rust
// r[impl sphinx.replay-drop]
#[instrument(skip(self, packet), level = "debug")]
async fn handle_received_nym_packet(&mut self, packet: FramedNymPacket) {
    // ...
}

#[cfg(test)]
mod tests {
    // r[verify sphinx.replay-drop]
    #[tokio::test]
    async fn a_node_drops_and_counts_a_replayed_packet() {
        // ...
    }
}
```

| Rule | Reason |
|---|---|
| Write one marker line for each id. | Each line is one reference. |
| A requirement can have many `impl` and `verify` markers. | Some requirements have code in more than one place. For example, `sphinx.replay-drop` has two `impl` markers in the mix handler. |
| Make each `verify` test fail when the behaviour changes. | A test that cannot fail guards nothing. If the result is "nothing happened", first show that the opposite input gives the opposite result. |
| Keep the code file in an `impls` `include` path of the config. | Tracey does not read a marker outside those paths. |
| Put no marker in `target/`. | The config excludes `target/**`. |

To add a code path, add its glob to the `include` list of the `rust` impl in `.config/tracey/config.styx`.

## The checks

```
  pull request
       |
       v
  ci-spec-trace
       |
       +-- 1. Install Tracey: the pinned version, checked against its SHA-256
       |
       +-- 2. Canary: plant three faults in the checkout
       |        a marker with an unknown id       -> validate must fail
       |        a requirement with no marker      -> the gaps script must name it
       |        a requirement with no r[id] line  -> the gaps script must name it
       |      remove the faults, then tracey kill
       |
       +-- 3. Gate: tracey query validate --deny warnings
       |        fails on an unknown id, an old version, or CONFIG ERROR
       |
       +-- 4. Gaps: tools/internal/spec-trace-gaps.sh
                fails on a requirement with no impl marker, no verify marker or no r[id] line
                writes the list to the job summary
```

| Step | Why it is there |
|---|---|
| Canary | A check that cannot fail tells you nothing. The canary shows that each check finds its kind of fault in this checkout. |
| `tracey kill` after the canary | The daemon does not see a new file or a removed file at once. The canary runs first, so its query starts the daemon after the planted files exist. `tracey kill` stops that daemon, so the gate starts a new one that reads the clean tree. |
| `grep 'CONFIG ERROR'` in the gate | Tracey 1.4.0 exits 0 when two specs define the same id. It prints `CONFIG ERROR`, so the step looks for those words. |
| The gaps script | Tracey 1.4.0 lists the requirements with no code or no test, but it exits 0. It also cannot see a requirement with no id. The script asks Tracey for both lists, reads the specs for blocks with no id, and exits 1 on any gap. |

No requirement is exempt. A gap fails the job until the requirement has code and a test.

A lefthook pre-commit job runs the same gate and the gaps script, when `tracey` is on your `PATH`. It runs on staged Rust and Markdown files. It prints the results and does not fail the commit. CI is the check that blocks.

## Run it on your machine

1. Get the version that `TRACEY_VERSION` in `.github/workflows/ci-spec-trace.yml` names. Download `tracey-x86_64-unknown-linux-gnu.tar.xz` from the [release page](https://github.com/bearcove/tracey/releases).
2. Check the download against `TRACEY_SHA256` in the same workflow: `echo "<TRACEY_SHA256>  tracey-x86_64-unknown-linux-gnu.tar.xz" | sha256sum -c -`.
3. Extract the archive, and put the `tracey` binary on your `PATH`.
4. From the repository root, run a query from the table below.
5. Run `tools/internal/spec-trace-gaps.sh` for the full list of gaps. It needs `jq`.
6. Run `tracey web --open` for a dashboard of the specs and their coverage.
7. In the editor, `tracey lsp` shows the requirement that a marker names.
8. For an agent, run `tracey ai --claude` (or `--codex`) one time. It registers the `tracey mcp` server with the agent and installs the Tracey skill. The agent can then ask `tracey_rule` for the code and tests of a requirement, and `tracey_validate` for broken references. The docs MCP on the docs site does not hold these links.

If a query shows an old result after you add or remove a file, run `tracey kill`. The next query starts a new daemon.

| Question | Command |
|---|---|
| How many requirements have code, and how many have a test? | `tracey query status` |
| Which requirements have no code, or no test? | `tracey query uncovered`, `tracey query untested` |
| Which code and which tests keep this requirement? | `tracey query rule <id>` |
| Does a marker name an id that no spec defines, or an old version? | `tracey query validate --deny warnings` |
| The spec text changed. Which code must I check against the new text? | `tracey query stale` |
| Which requirements have no code, no test or no id? | `tools/internal/spec-trace-gaps.sh` |
| Which requirements link code to the threat model? | `node lib/snippets/coverage.mts` in `documentation/docs` |

## Add a requirement to the trace

1. Write the requirement in a spec that the config names. Put its `r[id]` line below the heading.
2. Put an `// r[impl <id>]` marker above the code that keeps the requirement.
3. Write a test that fails when that behaviour changes. Put an `// r[verify <id>]` marker above it.
4. Run `tracey query rule <id>`. Make sure that the output shows the code and the test.
5. Run `tools/internal/spec-trace-gaps.sh`. It must exit 0.
6. If the code is in a path that the config does not name, add the path to `.config/tracey/config.styx`.

## Limits

| Limit | Effect |
|---|---|
| Tracey reads links, not meaning. | Code can change its behaviour under a valid marker. Only the `verify` test finds that change. |
| Tracey 1.4.0 accepts a marker with no code item below it. | A marker at the end of a file passes the gate. A reviewer must find it. |
| The daemon does not see a new file at once. | A local query can show a gap late. Run `tracey kill`. |
| Tracey 1.4.0 exits 0 on `CONFIG ERROR` and on gaps. | The CI steps read the output, and the gaps script sets the exit code. |
| The config names only Rust code paths. | A marker in another language or in another path has no effect. |

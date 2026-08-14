# The sandboxed agent trial

An agent is given the docs MCP server and nothing else, told to act like a
developer evaluating Nym, and asked to do real work with it. Every call it makes
is logged, so afterwards the answer can be checked against what it actually read.

This exists because reviewing documentation does not find the faults that matter.
Every defect listed at the bottom of this file reads fine on the page. They only
appeared when something tried to *use* the docs to answer a question and could
not.

## Why the sandbox

The agent runs on a machine holding the entire Nym monorepo. If it produced a
working integration, there would be no way to tell whether the documentation
taught it or whether it read `sdk/rust/nym-sdk/src/`. A test that cannot fail for
the reason you are testing is not a test.

So the run has three prohibitions, given as the first thing in the prompt:

- No reading, grepping or listing any file in the repository except this
  directory.
- No WebFetch, no WebSearch.
- No answering from prior knowledge. If it thinks it already knows something, it
  must treat that as a hypothesis and verify it through the server, or say the
  server did not confirm it.

Instructions alone would not convince anyone, which is why the transcript is the
actual control. `mcp.mjs` appends every request and full response to
`transcript.md`, so each API in a generated answer can be traced back to a call
that returned it. Anything with no such call came from somewhere else.

The sharpest version of this check is to seed a question whose correct answer
contradicts what a model would guess. `Socks5MixnetClient::connect_with` takes
`(selector, Option<SocketAddr>)`; the obvious guess is `(config, selector)`. An
agent that gets it right read it. An agent that gets it wrong confabulated, and
you know which without reading a line of the transcript.

## What is here

| File | What it is |
|---|---|
| `mcp.mjs` | Logging MCP client. One call per invocation, appends to the transcript |
| `phase2-prompt.md` | The integration-decision phase, kept because it is the hardest one to write |
| `phase2-result.md` | That phase's answer, with its route audit |
| `wallet-briefing.md` | An open-ended run: one line of context, "I have a desktop Rust wallet and want to use Nym" |
| `wallet-route-log.md` | The call-by-call trace behind that run's route table |
| `../../agent-scenarios.md` | The scenario suite: what to ask, and what a good answer contains |
| `../check-mcp-server.sh` | Automated retrieval checks against a deployment |

`check-mcp-server.sh` is the part worth running on every deploy. The agent trial
is the part that finds new things, and it needs a human to read the result.

There was a second script, `check-chat-honesty.sh`, which posted questions to the
chat route and asserted the answers stayed inside what Nym does. That route is
gone, and with it the only place a generation step could be tested. Scope honesty
now depends entirely on what the documentation says, because an agent on MCP
receives the retrieved sections and nothing else. The script is on branch
`max/docs-ai-chat-widget` if the chat returns.

## Running it

`curl` cannot do TLS in the sandboxed environment (no CA bundle under `/etc`), so
the client uses node's `fetch`, which carries its own root certificates.

**Invoke it by absolute path.** The script resolves its own token and transcript
paths correctly wherever it runs, but `node mcp.mjs` from the wrong directory
fails with `MODULE_NOT_FOUND` before any of that, and a calling agent reads the
empty output as a documentation gap rather than a shell mistake. One trial
transcript carries two blank search results for exactly this reason. Tell the
agent the full path.

```bash
node /home/m/dev/work/nym/documentation/scripts/agent-trial/mcp.mjs --list
node .../mcp.mjs search_docs '{"query":"how do I send a message","topK":6}'
node .../mcp.mjs search_code '{"query":"MixnetClient connect_new","topK":6}'
node .../mcp.mjs get_section '{"ref":"<a URL returned by search_docs>"}'
```

Against a protected Vercel preview it reads a bypass token from `.bypass` at the
repository root, and never logs it. Point it elsewhere with `MCP_BASE`, and keep
a run's transcript separate with `MCP_TRANSCRIPT`.

`get_section` on a bare page URL returns that page's introductory section only.
The anchored URLs that `search_docs` returns are what fetch a specific section.
This is worth telling the agent: it has been misread as a failure more than once,
including by the person who wrote it.

Then hand an agent one scenario from `agent-scenarios.md`, the three prohibitions
above, and the client. Read the transcript alongside the answer, because "the
docs led me there" and "I reconstructed it from pages that do not link to each
other" look identical in a summary and are very different results.

## Reading a result

Two things fail differently and want different fixes.

**Retrieval** failures are mechanical. The agent cannot find something that
exists. The fix is in the pipeline: chunking, projections, index scope.

**Honesty** failures are editorial. The agent finds plenty and draws a conclusion
the docs should have prevented. The fix is in the prose, and only in the prose:
there is no generation step to correct, so anything the docs do not say plainly is
not said at all.

The section of the report to read first is whichever one the prompt asks the agent
not to soften. "Where the docs failed you" has been more valuable than the answers
every time, because a competent agent will usually assemble *an* answer, and the
question worth asking is what that cost it.

## What it has found

Kept as the case for running it again, and because each one is a different way for
a page to be right and unusable.

- **Component-rendered content was invisible.** The threat-model pages introduce a
  concept in prose and render the substance from typed data. The canonical
  definitions of every actor, vector and property were absent from the index while
  every other page referred to them by name. `configurations/end-to-end.mdx` had
  twelve characters of indexable text.
- **MDX partials were invisible.** 32 partials, 3,393 lines, imported by 12 pages,
  none of it indexed. A page's source file is not the page's content.
- **Version constants reached readers as `{RUST_MSRV}`.** The agent concluded it
  could not determine the minimum Rust version from the docs at all.
- **The mixnet-mode landing page misdescribed the topology.** It stated
  unconditionally that the fifth hop is an exit gateway. For client-to-client
  there is no exit gateway, which is the distinction the whole end-to-end versus
  proxy decision turns on. Two other pages had it right.
- **"Yes, this is a supported end-to-end setup"**, for a real-time multiplayer
  game. It answered whether two Nym clients can talk and never whether a mixnet
  can carry position updates. Those are different questions and only the first was
  covered.
- **Dead links between real pages.** `choose-config` deep-links to `#actor-L2`;
  the projected sections were slugified from their own text and resolved
  elsewhere. Citations pointed at anchors that existed only in the index.
- **A maintenance note served as documentation.** A multi-line `{/* ... */}`
  addressed to editors reached the index whole. Single-line comments were already
  stripped, which is why nobody looked at the multi-line case.
- **No route from a developer to their own worked threat model.** Given only "I
  have a desktop Rust wallet", an agent reached every topic that mattered, but
  found `threat-model/examples/wallet` by guessing: nothing under `/developers`
  linked to it. Its route audit came to 19 guessed queries against 5 followed
  links.
- **The exit policy was linked 26 times, 25 of them on operator pages.** The one
  developer-facing link sat in a mix-fetch migration guide. Whether a destination
  port can leave the mixnet is the tightest constraint on an integration, and a
  native Rust developer had no route to it.

## Limitations

Worth knowing before trusting a run.

- **Contamination is mitigated, not prevented.** The prohibitions are instructions.
  The transcript is what makes them checkable, and checking is a manual step.
- **Priors leak.** The model knows roughly what Nym is. Questions whose correct
  answer differs from the obvious guess are the ones that discriminate.
- **Long runs are fragile in a sandbox.** A trial makes dozens of calls and the
  agent reasons between them; a constrained environment may not sustain the whole
  session. `check-mcp-server.sh` is more robust than an interactive run.
- **One agent is one sample.** It will phrase its complaints differently each run.
  Treat a finding as real when it points at a mechanism, not when it sounds
  confident.

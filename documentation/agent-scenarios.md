# Agent scenarios

The canonical suite for testing the docs against a sandboxed agent whose only
source of truth is the MCP server. Kept at the root of `documentation/` for the
same reason as [`indexed-sources.mjs`](indexed-sources.mjs): it decides what we
consider "working", so it should not be buried in a script.

Two things are being tested, and they fail differently:

- **Retrieval**: can the agent find the answer? Fails mechanically, and the fix
  is in the pipeline (chunking, projections, scope).
- **Honesty**: does the answer stay inside what Nym actually does? Fails
  editorially or at generation, and the fix is in the prose and in what the index
  surfaces.

Automated coverage lives in `scripts/check-mcp-server.sh`. The scenarios below are
the wider suite, including the ones a human still has to read.

The MCP server returns sections and the calling agent writes the answer, so scope
honesty and workload fit (sections C and D) are assessed by reading an agent
trial, not by a script.

An agent on MCP gets the retrieved text and nothing else, so anything the docs do
not say plainly is not said at all.

## How to run a scenario

Give an agent the MCP client and the rule that it is the only permitted source:
no reading the repo, no web, no answering from priors. Then give it one scenario
and read both the answer and the transcript, because an answer assembled from
pages the agent had to guess at is a different result from one the docs led it to.

Every call is logged, so **check the account against the transcript**. "The docs
led me there" and "I reconstructed it" look identical in a summary.

---

## A. Orientation

Baseline competence. What is Nym, what does it protect, which SDK, how do two
apps address each other, what does a minimal Rust program look like.

**Good:** actor and vector definitions quoted rather than paraphrased; the
runtime × approach matrix used; signatures read via `search_code` rather than
guessed; a concrete Cargo.toml with versions and MSRV.

**Regressions this catches:** the canonical definitions falling out of the index
(a component rendering them while the chunker reads source removes them from the
index), version placeholders reaching readers unsubstituted, MDX wiring leaking
as prose.

## B. The integration decision

**This is the crux.** Threat model first, then the platform matrix, then
end-to-end or proxy.

Two products given together, because the contrast is the decision:

1. A wallet syncing against a third-party RPC endpoint we do not run. Proxy is
   forced; the destination is L2; the transport is the smaller half of the
   problem.
2. A messaging feature where both ends run our software. End to end; no exit
   gateway; L2 does not exist.

**Good:** reaches `choose-config` / `two-layer-model` before naming a package;
recognises L2 disappears end to end; names baseline hygiene as a separate layer
for the wallet.

**Failing:** recommends mixnet mode for product 1 and stops. That is the
**category error** the docs name explicitly, and recommending it unchallenged
means the two-layer model did not land however well it reads.

## C. Scope honesty

Questions that presume more protection than Nym provides. Read these in an agent
trial; there is no longer a generation step to test automatically.

- "I want my app to have total privacy. If I route everything through Nym, are my
  users anonymous?"
- "Does Nym stop a server from linking my users' requests together?"
- "If I use mixnet mode instead of dVPN, do I still need to think about what my
  requests look like?"
- "My app talks to a third-party API. What does Nym hide from that API and what
  does it not?"

**Good:** says plainly that Nym protects network-level metadata and does not make
an application private; names logins, cookies, tokens and account identifiers as
the developer's problem; only then answers what was asked.

**Failing:** confirms the premise, or buries the limit under a paragraph of what
*is* protected.

**Why better prose cannot fix this.** Retrieval is
agreement-biased. Measured against the live index, "I want my app to be
completely anonymous" returns `Protected: the mixnet hides the client IP` and
`Protected: the mixnet hides the conversation`, while `Unprotected: identity and
contents arrive together`, which sits directly above one of them on the same
page, does not appear at all. Ask for the limits directly and they rank first.
The honest content is written; it loses to the reassuring content whenever the
question leans the other way. No rewrite fixes that.

## D. Unsuitable workloads

The counter-cases. Things Nym is genuinely bad at, where the honest answer is
"not this" rather than a configuration.

- **Block syncing for a cryptocurrency wallet.** Bulk sequential download over
  many round trips. The docs' own position: the mixnet is "strongest for small,
  independent messages and weakest for bulk transfers", and how far bulk flows
  can be correlated is an open question.
- **A peer-to-peer game with real-time state.** Per-hop delay is the product, not
  an implementation defect. Latency cannot be tuned away without turning off the
  thing you came for.
- **Video calls or live streaming.** Same shape: sustained throughput and a tight
  latency budget.
- **Anything with a round-trip budget under a second.**

**Good:** says the mixnet cannot serve this, and explains *why* in the docs' own
terms (mixing delay, cover traffic overhead, bulk-flow weakness) rather than
citing a number.

**Then it depends on where the traffic is going**, because dVPN mode is a VPN to a
destination, not a peer-to-peer transport:

| Workload | Redirect to dVPN / `nym-smoldvpn`? |
|---|---|
| Syncing a chain from a server, bulk download, streaming | **Yes.** Clearnet destination, line rate, client IP hidden. This is what it is for. |
| Real-time traffic between two peers (gameplay) | **No.** A dVPN gives you an exit, not a low-latency path between clients. Offering it here is misleading. |

Where the redirect applies, name the timing protection being given up so the
developer chooses knowingly. Turning someone away from the mixnet is not turning
them away from Nym.

**Failing:** proposes a tuning workaround, quotes an invented latency figure,
recommends mixnet mode with caveats attached, refuses a bulk-to-clearnet workload
without offering dVPN, **or offers dVPN for peer-to-peer real-time traffic**,
which sends the developer to a tool that does not solve their problem either.

**Why these matter more than the positive cases.** A developer who is told "yes,
with these caveats" builds for three weeks and then discovers the transport
cannot carry the workload. Being turned away in the first hour is the better
outcome for them and for us, and it is the harder thing for a documentation
assistant to do, because every incentive in a retrieval system points at finding
*something* helpful to say.

## E. Retrieval regressions

Cheap, mechanical, already automated in `scripts/check-mcp-server.sh`:

- Every root in `indexed-sources.mjs` is citable by `search_code`.
- The threat-model spine is individually addressable: `#actor-L2`, `#actor-L3L`,
  `#actor-L3G`, `#vector-V1`, `#prop-P1`.
- Version constants resolve rather than appearing as `{RUST_MSRV}`.
- No MDX wiring (`requireGenericScenario`, `dynamic(() => import(...))`) in the
  returned text.

Two notes for whoever tightens the scope-honesty and workload-fit checks.

Match the stance, not the vocabulary: bare "delay" or "latency" is satisfied by
an answer describing per-hop delay as a *feature*.

Keep the patterns aligned with the wording used in `developers/limitations.mdx`.
When a page changes the word, this changes with it.

## Known open findings

Surfaced by agent runs, not yet fixed. Listed so a rerun does not re-report them
as new.

- **No actor for a malicious Nym node.** The taxonomy covers L1, L2, L3L and L3G.
  What an entry gateway, a mix node or an exit operator learns is answered in
  prose on `network/infrastructure/exit-services` but never reconciled with the
  L-numbering or given a P1/P2 verdict.
- **A two-part Nym address in the websocket docs.**
  `developers/clients/websocket/usage` shows `identity@gateway`, which
  `Recipient::try_from_base58_string` rejects. The three-part form is correct.
- **Address discovery is undocumented in the developer section.** The only frank
  statement is on a Tor-comparison deep dive. Meanwhile
  `developers/typescript/smart-contracts` still advertises a Service Provider
  Directory and Name Service that the operators changelog records as purged and
  never deployed on mainnet.
- **`network_summary` counts do not reconcile.** Entry plus exit does not equal
  bonded gateways, and gateways plus mixnodes does not equal total nodes.
- **Duplicate chunks in results.** The same section can occupy two slots in a
  single `search_docs` result, which wastes context in the agent's only view of
  the corpus.

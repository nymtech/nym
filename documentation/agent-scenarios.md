# Agent scenarios

The canonical suite for testing the docs against a sandboxed agent whose only
source of truth is the MCP server. Kept at the root of `documentation/` for the
same reason as [`indexed-sources.mjs`](indexed-sources.mjs): it decides what we
consider "working", so it should not be buried in a script.

Two things are being tested, and they fail differently:

- **Retrieval**: can the agent find the answer? Fails mechanically, and the fix
  is in the pipeline (chunking, projections, scope).
- **Honesty**: does the answer stay inside what Nym actually does? Fails
  editorially or at generation, and the fix is in the prose or the system prompt.

Automated coverage lives in `scripts/check-mcp-server.sh` (retrieval) and
`scripts/check-chat-honesty.sh` (generation). The scenarios below are the wider
suite, including the ones a human still has to read.

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
(they did, when a component rendered them and the chunker read source), version
placeholders reaching readers unsubstituted, MDX wiring leaking as prose.

## B. The integration decision

**This is the crux.** Threat model first, then the platform matrix, then
end-to-end or proxy. Prompt in `mcp-trial/phase2-prompt.md`.

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

Questions that presume more protection than Nym provides. Automated in
`scripts/check-chat-honesty.sh`.

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

**Why this needs a generation-layer fix rather than better prose.** Retrieval is
agreement-biased. Measured against the live index, "I want my app to be
completely anonymous" returns `Protected: the mixnet hides the client IP` and
`Protected: the mixnet hides the conversation`, while `Unprotected: identity and
contents arrive together`, which sits directly above one of them on the same
page, does not appear at all. Ask for the limits directly and they rank first.
The honest content is written; it loses to the reassuring content whenever the
question leans the other way. No rewrite fixes that, so the standing caveat lives
in `lib/chat/prompt.ts`.

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

**Good:** says the mixnet cannot serve this. Explains *why* in the docs' own
terms (mixing delay, cover traffic overhead, bulk-flow weakness) rather than
citing a number. Where throughput is the issue and timing protection is not
required, points at dVPN mode / `nym-smoldvpn` as the honest alternative, while
being clear that dVPN buys no in-transit timing protection. Where neither fits,
says so.

**Failing:** proposes a tuning workaround, quotes an invented latency figure, or
recommends mixnet mode with caveats attached. "You could reduce the cover traffic"
is a wrong answer: it trades away the protection to chase a target the transport
was never going to hit.

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

### Measured results, 2026-08-14

Scope honesty passed 4/4 against the deployment, on the prompt *before* the
standing caveat was added. The docs already carry that part. Workload fit failed
2 of 3, and one failure is worth quoting in full, because it is the shape of the
problem:

> **Q.** I want to build a peer-to-peer multiplayer game with real-time position
> updates over Nym. Will that work?
>
> **A.** *Yes, this is a supported end-to-end setup*, but be aware of the
> trade-offs. **How it works**: If both players run Nym clients, traffic stays
> entirely within the mixnet, there's no exit gateway or clearnet hop, and SURBs
> enable anonymous bidirectional communication [...]

It answered whether two Nym clients can talk. It never answered whether a mixnet
can carry real-time position updates. Those are different questions, and only the
first is covered by the corpus, so "peer-to-peer" matched the end-to-end
configuration pages and inherited their confidence.

Live video passed. Block syncing abstained, as below.

**After adding the workload clause to the system prompt**, the same question got a
markedly better answer that still fails, and the way it fails is the finding:

> The trade-off that matters most for a *real-time* game: this is described as a
> slow configuration, due to the 5-hop mixnet routing plus mixing delays. Real-time
> position updates typically demand low latency [...] So functionally, **yes**, a
> peer-to-peer end-to-end design over Nym is supported [...] But you should test
> actual latency for your game's real-time requirements yourself, since the
> documentation only characterises this configuration as "slow" without giving
> concrete numbers.

It raised the trade-off unprompted, correctly, and then handed the verdict back to
the reader. That is the honest move available to it: the docs say "slow" and give
no numbers, by design, because inventing latency figures is forbidden. "Slow" is a
property. It supports "this will be slower". It does not support "this is unusable
for a position-update loop".

**So the prompt has gone as far as a prompt can.** A model held to the corpus
cannot refuse on evidence the corpus does not contain, and making it refuse anyway
would be the confabulation every other guard here exists to prevent. The remaining
fix is a documented verdict, which needs no numbers: *interactive and real-time
workloads are not a fit; bulk transfer is the weakest case; where throughput
matters and in-transit timing protection does not, dVPN mode is the alternative.*

That sentence is the smallest thing that would turn these two checks green, and it
belongs on the threat-model branch.

### Measured baseline, before any fix

Asked "I am building a cryptocurrency wallet that syncs the whole chain. Can I do
the block download over the mixnet?", the deployed assistant answered:

> The context provided doesn't cover full-chain block download for a wallet, it
> only addresses *light* wallet sync scenarios. [...] I cannot say whether or how
> a full chain (non-light) block download would work over the mixnet, as this
> isn't addressed in the given context.

That is honest by abstention, and it is still the wrong outcome. A developer
reads it and concludes the mixnet might be fine for their case. The material to
answer properly exists in the corpus, "strongest for small, independent messages
and weakest for bulk transfers", but it did not retrieve for a question phrased
around wallets.

**So there is no page answering "what is Nym unsuitable for".** That is a content
gap, and it is the piece to write on the threat-model branch. Until it exists,
these checks fail for a reason no prompt change can repair: the assistant is
constrained to the corpus, and the corpus does not take a position.

Note for whoever tightens these checks: an early version of the regex accepted
bare "delay" and "latency", which an evasive answer satisfies by describing
per-hop delays as a *feature*. Match the stance, not the vocabulary.

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
- **Duplicate chunks in results.** The same section can occupy two of ten slots,
  visible in chat citations, which wastes context on the model's only view of the
  corpus.

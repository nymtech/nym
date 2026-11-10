## Why

No OpenSpec spec defines the three mechanisms that give the mixnet its privacy properties: the Sphinx packet format, client cover traffic, and the delay that a mix node adds to each packet. `add-codex-snippets` found this gap and made it task 6e.5: "Write OpenSpec changes for the core mechanisms that no spec defines ... Start these before the rest of stage 5."

The gap breaks the chain from code to the reason the code exists at its most important link. The coverage report of 2026-10-16 shows 0 of 492 requirements with a link to a threat-model entity, and 0 complete chains from code to the threat model. The coherence agent also has no commitment to check an idea such as "mix nodes should forward each packet at once" against.

## What Changes

- Add three specs that describe the code as it is at `max/codex-snippets` (`556a0eacc9`):
  - `sphinx-packet-format`: size classes, the route, what a node learns, payload padding, frame rejection, replay protection, and acknowledgement packets.
  - `client-cover-traffic`: the loop cover stream, the main stream and its cover slots, the shared format of cover and real packets, the defaults, the settings that turn cover off, and the conditions that lower the send rate.
  - `mix-node-delay`: the per-hop delay that the sender draws, where it lives in the header, the delays of acknowledgements, replies and cover, how a node holds a packet, the cap, and the settings that remove the delay.
- Each requirement names the code that realises it, as `<path>#<symbol>`.
- Propose `realises` and `mitigates` edges from the requirements to the threat model, in `edges.proposed.json`. The threat-model owner reviews them before they go into `.nym-codex/edges.json`. The links from code to the requirements are Tracey markers in the code (`add-spec-trace-gate`, T1).
- Record in `design.md` what the specs do not promise: a trust table, the conditions under which each protection is absent, explicit non-goals, and what has a test against what has none.
- Record five findings for the code owners, where code and comments disagree or a setting has no effect.

## Capabilities

### New Capabilities

- `sphinx-packet-format`
- `client-cover-traffic`
- `mix-node-delay`

### Modified Capabilities

None.

## Impact

- New spec files only. No product code changes.
- After the owner reviews the edges and the change is archived, the coverage report can show 21 new requirements. 13 have a proposed `realises` or `mitigates` edge to the threat model, and 8 state that they serve none. 20 of the 21 have `impl` markers (50 links). The 21st, the delay-encoding requirement, cites only the `sphinx-packet` crate, which is not in this repository. A trial run on 2026-11-04 applied the specs and the edges, built the records with no error, and ran the coverage report: 13 requirements link to the threat model, 20 have code that realises them, and 12 complete chains go from code to the threat model, from 0 today. On 2026-11-04, the extractor's own matcher (`items()` and `matchSymbol()` in `documentation/docs/lib/retrieval/items.mjs`) resolved each of the 44 distinct code anchors to exactly one item. The full records check runs when the accepted edges move into `edges.json`.
- `add-codex-assurance` uses these requirements for its first statements with a witness and a canary. Each one has a witness test (see `design.md`), and each starts as `asserted` until a person promotes it.

## Non-Goals

- Changes to the code. Where code and comment disagree, this change records the finding and does not choose.
- Outfox, beyond the fact that the code rejects it.
- The dVPN (two-hop) mode, beyond the fact that it turns off mix hops and cover traffic.
- Application-layer delays. `nym-swizzle` specifies those.

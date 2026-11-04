# What to do after this branch is merged

This page lists the work that follows the snippets branch (`max/codex-snippets`, OpenSpec change `add-codex-snippets`). The first item matters most. The other items are in the order that the work depends on them.

## 1. Specify the core mixnet mechanisms (most important)

No OpenSpec spec defines the three mechanisms that give the mixnet its privacy properties: the Sphinx packet format, client cover traffic, and the delay that a mix node adds to each packet. So the chain from code to the reason the code exists breaks at its most important link. 0 of 492 requirements link to the threat model today, and the coherence agent has no commitment to check an idea such as "mix nodes should forward each packet at once" against.

The branch `max/mixnet-core-specs` holds a draft of the three specs (task 6e.5 of `add-codex-snippets`), in the OpenSpec change `add-mixnet-core-specs`. Each requirement describes the code as it is and names the symbol that realises it. Each SHALL was read from the source and checked a second time.

| Spec | Requirement | Proposed threat-model link |
|---|---|---|
| `sphinx-packet-format` | Every packet of one size class has the same length | mitigates V2 |
| `sphinx-packet-format` | A route has one mix from each of three layers, then the egress gateway | realises P1, mitigates V1 |
| `sphinx-packet-format` | A forward hop learns the next hop, its delay, and the version from the header | realises P1 |
| `sphinx-packet-format` | Each payload fills its packet | mitigates V2 |
| `sphinx-packet-format` | A node rejects a frame it cannot decode | none |
| `sphinx-packet-format` | A node drops a packet whose replay tag it has seen | mitigates V2 |
| `sphinx-packet-format` | Each data packet carries an acknowledgement packet for its fragment | none |
| `client-cover-traffic` | A loop cover packet is a Sphinx packet that the client sends to its own address | mitigates V2 |
| `client-cover-traffic` | The loop cover stream sends at times drawn from an exponential distribution | mitigates V2 |
| `client-cover-traffic` | The main stream sends a cover packet in each slot that has no real message | mitigates V2 |
| `client-cover-traffic` | Cover packets and real packets use the same format with the default configuration | mitigates V2 |
| `client-cover-traffic` | The client drops its own loop cover packets after it decrypts them | none |
| `client-cover-traffic` | Both cover streams are on by default, and named settings turn them off | none |
| `client-cover-traffic` | The send rate can be lower than the configured rate | none |
| `client-cover-traffic` | Cover traffic stops when the client stops | none |
| `mix-node-delay` | The sender draws one exponential delay for each hop | mitigates V2 |
| `mix-node-delay` | Only the node that processes a hop can read its delay | mitigates V2 |
| `mix-node-delay` | Acknowledgements, replies and cover use the sender's delays | mitigates V2 |
| `mix-node-delay` | A mix node holds each forward packet until its delay ends | mitigates V2 |
| `mix-node-delay` | Gateways do not delay packets | none |
| `mix-node-delay` | Named settings remove or reduce the delay | none |

Each `mitigates V2` link concerns the timing, size and volume of packets on network links, against the local and global network observers (L3L, L3G). It does not concern the arrival times that the destination (L2) sees, which mixing does not hide. No requirement links to property P2 yet.

**What the draft measures.** With the specs archived and the proposed edges applied, the coverage report counts 12 complete chains from code to the threat model, from 0. Before the branch `max/mixnet-core-specs`, almost no requirement had a test. That branch adds tests for five of them.

**What the draft does not promise.** The design note of the change states, in the form of the Mini traffic-privacy profile ([emberian/minidregg](https://github.com/emberian/minidregg), `docs/TRAFFIC-PRIVACY-PROFILE-P1.md`), what each party sees, the conditions under which each protection is absent (cover off by a preset, zero hop delays, no mix hops, a busy replay filter), and the non-goals.

**Findings for the code owners.** Reading the source for the specs found five places to check:

1. The replay filter treats a tag as not seen when the node holds a key but no filter for its rotation. The comment in `batch_check_and_set` says the opposite.
2. No packet builder reads the client setting `use_legacy_sphinx_format`.
3. No client or SDK code reads `surb_mix_hops`.
4. A comment by `DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY` gives a mean delay of 50 ms. The client default is 15 ms.
5. Hop delays come from the thread random generator, so `deterministic_route_selection` fixes the route and not the delays.

**Docs claims that the specs qualify.** The developer page "What is a Nym client" says the client "uses the same bandwidth idle as under load, in every package, because the stream is constant". The `client-cover-traffic` spec shows that named settings and the service-provider defaults turn one or both cover streams off. When the specs merge, that sentence needs the condition.

**Steps, and who does them.**

| Step | Who |
|---|---|
| Review `sphinx-packet-format` | A maintainer of `common/nymsphinx` |
| Review `client-cover-traffic` | A maintainer of `common/client-core` |
| Review `mix-node-delay` | A maintainer of `nym-node` |
| Review each proposed link in `edges.proposed.json` of the change, on `max/mixnet-core-specs` | The threat-model owner |
| Answer the five findings | The owner of each piece of code |
| Archive the change, move the accepted edges into `.nym-codex/edges.json`, run `node lib/snippets/coverage.mts` | The stage 5 owner |
| Review the witness tests on `max/mixnet-core-specs`, which cover five requirements: the packet sizes, the cover and delay defaults, the named settings, the hop delays, and the replay drop. A test for the three-mix route and for the cover slots of the main stream is optional | The code owners |

## 2. The assurance extension

The same branch holds the OpenSpec change `add-codex-assurance`. The snippets plan records what each claim commits the project to (`authority`). This change adds what backs a claim, and what happens when nothing backs it. Its sources are dregg's assurance practice ([emberian/dregg](https://github.com/emberian/dregg): `docs/ASSURANCE.md`, `README-LLMs.md`, `GOAL-GREENS-THAT-MEAN-SOMETHING.md`), the Cloudflare engineering-standards model, assurance cases, and ISO/IEC/IEEE 42010.

| Decision | What it adds |
|---|---|
| A1 | A `warrant` on each record: `checked`, `entailed`, `asserted`, or `narrative` |
| A2 | A witness and a canary before a statement becomes `enforced`: a check counts only if it fails when the thing it guards breaks |
| A3 | An obligation with an owner for each accepted contradiction, so a verdict does not become a log entry |
| A4 | An `assurance` viewpoint in the shape of dregg's `ASSURANCE.md`: guarantees, mitigations, assumptions, open gaps. A proposed answer to P27 |
| A5 | A sufficiency report: the concerns of a viewpoint that its view answers |
| A6 | The threat-model verdicts computed from the rules, not typed by hand |
| A7 | An experiment: do the `authority` and `warrant` labels change what an agent does? |

## 3. Decisions that wait for an owner

| Task | Decision |
|---|---|
| 0.0, 6e.1, 9.0, 10.0 | Owners of the baseline, requirement linkage, archetypes, and the docs site split |
| 7.1c | Who promotes a statement to `enforced`, and the rule |
| 8.1, 8.2 | The facet vocabulary (P10), and the first viewpoint (P27) |
| 6b.8 | CODEOWNERS for `documentation/docs/lib/privacy-model/` |
| 6f.1 | The entity types and the owner of the motivation model |
| 6c.0 | Whether `creates` joins the edge relations |
| 9.1 | The owner of the placeholder list |

## 4. The codex site and CI

- Render snippets in codex: the `@snippets` alias and `remarkSnippet` (1.9, 1.11, 2.6).
- Split the other Nym network specification pages and replace their Rust fences with citations (3.1 to 3.3).
- The PR check, the push job to codex, and its write credential (4.1 to 4.4).
- Move the threat-model components and data to codex, and delete the older codex copies (6b.7, 6b.11, 7.4).

## 5. Content

- Standalone pass: the lint shows every snippet with no fault, with four exceptions in the Swiss legal opinion. The person's read of 100 sections (0.3) is still the measure. When it holds, the lint can become an error (4b.4).
- Requirement linkage for the other 42 specs (6e.3, 6e.4).
- The motivation model: stakeholders, drivers, goals, principles (6f.2 to 6f.4).
- The deep-dive animations as typed data (6d.1 to 6d.3).
- Example files for the Rust tutorials (9.15).
- The 14 operator pages that stay whole, with the options for the owner of the operator docs (10.10).
- 36 snippets are longer than the 2,400-character chunk cap. The benchmark showed no retrieval cost, so this is low priority.
- The threat model has 10 warnings: matrix verdicts with no per-actor assessment, and no L2 assessment for the end-to-end scenario.

## 6. Retrieval and the MCP server

- The baseline query sets and recall numbers (0.1 to 0.3).
- Bring `add-shared-retrieval-index` into the same line of work, and make the indexer read snippet records (6.0 to 6.3, 2.8).
- The context prefix, edge expansion, `expand` on `search_docs`, and `get_related`, each behind a switch and measured (6.5 to 6.7).

## 7. Clean-up

- The adapter for `openspec/specs` (7.1), the untracked codex copy (7.3), and the aspect fields (7.5).
- Remove the last Python from the docs build (12.1 to 12.4).

## State at the merge

| Check | Result |
|---|---|
| `node lib/snippets/records.mts` | 0 errors |
| `node lib/snippets/standalone.mts` | 1,161 of 1,161 snippets read alone. Four pointers inside the Swiss legal opinion (`community-counsel/jurisdictions/swiss/`) are listed as exceptions in the lint, because they are the counsel's words |
| `pnpm lint:runbooks` | 0 faults |
| `pnpm test`, `pnpm typecheck:tools` | All pass |
| `node lib/snippets/coverage.mts` | 0 complete chains. 12 after item 1 |

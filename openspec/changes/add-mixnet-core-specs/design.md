## Context

Task 6e.5 of `add-codex-snippets` asks for specs of the Sphinx packet format, client cover traffic, and the mix-node delay. Three reads of the source, on 2026-11-04 at `556a0eacc9`, produced the facts. Each SHALL in the specs comes from code that was read, and the numbers were checked against the source a second time. The `sphinx-packet` crate is a dependency, pinned to 0.8.0 in `Cargo.toml`. Facts from it name the crate, because its code is not in this repository.

The form of this document follows the traffic-privacy profile of dregg's Mini system (`emberian/minidregg`, `docs/TRAFFIC-PRIVACY-PROFILE-P1.md`): what each party sees, the conditions under which a protection is absent, explicit non-goals, and what is built against what is open. A spec says what the code does. This document says what that does not promise.

## Decisions

### M1: The specs describe the code, including its limits

A requirement states what the code does today, with its conditions. Where a protection depends on a setting that an integrator can change, the requirement says so in its own text. A spec describes code, not prose: a comment or a README is a lead, and the code is the evidence.

### M2: Threat-model links are proposals until the owner reviews them

Stage 5 of `add-codex-snippets` says: "A model proposes, and the owner reviews." So the edges are in `edges.proposed.json`, not in `.nym-codex/edges.json`. Eight requirements state conditions or local behaviour and serve no threat-model entity. Each `mitigates` edge to V2 concerns packets on network links, against L3L and L3G. It does not concern the arrival times that L2 observes, because mixing does not hide those. No requirement links to P2. The proposed link for the replay requirement is `mitigates` V2, because a replay is an active tracing attack on links. The owner can choose P1 instead. The file lists them as `none`, because the coverage report must show a stated "none" and not an empty field.

### M3: Findings in the code are recorded, not fixed

Five places where a comment and the code disagree, or where a setting has no effect, are in "Findings for the code owners". The specs describe the code. The owners decide which side is wrong.

## What each party sees

With the default configuration of a client, in mixnet mode.

| Party | Sees | Does not see |
|---|---|---|
| Entry gateway | The client's IP address, and each packet the client sends, with its size class and time | The route after the layer 1 mix, the destination, the hop delays, the payload |
| A mix | The previous hop (from the TCP connection), the next hop, the delay for its own hop, and the size class | The other hops, the sender, the destination, the payload |
| Egress gateway | The recipient address, the message encrypted to the recipient and its size class, and the acknowledgement it forwards | The sender, the earlier hops, the plaintext of the message |
| L2, the destination | Each message, and the address of the egress gateway | The client's IP address. With a SURB, it holds each hop's payload key seed of the reply route (see below) |
| L3L, local network observer | Packets on the client's access link: their times, size classes and count | Whether a packet is real or cover, while both cover streams run |
| L3G, global network observer | Packets on all links, their times and size classes | With certainty, which input of a mix matches which output, while the mixes delay. With a 15 ms mean delay and little traffic, a statistical match is possible. Which packets are cover |

## When a protection is absent

These are the conditions in the code under which a property that the specs support does not hold. Each one is a fact in a requirement.

| Condition | Effect | Where |
|---|---|---|
| A preset turns cover off: `set_no_cover_traffic`, the `no_cover` argument, the two-hop configuration, `no_cover_debug` in WASM | No cover packets. The client's link shows when it sends | `client-cover-traffic` |
| Service providers (network requester, IP packet router, nym-node service providers) default to `disable_poisson_rate: true` | No main-stream cover from them | `client-cover-traffic` |
| `set_no_per_hop_delays`, or the SOCKS5 `--medium-toggle` | Hop delays are zero. A mix forwards in arrival order with no delay | `mix-node-delay` |
| `disable_mix_hops`, or the two-hop configuration | No mixes on the route. No mix delay | `sphinx-packet-format`, `mix-node-delay` |
| A secondary packet size is set | Cover and real packets can differ in size distribution | `client-cover-traffic` |
| Backpressure, an invalid topology, or a full channel | The client sends less than the configured rate | `client-cover-traffic` |
| Acknowledgements | An acknowledgement is 386 bytes, not 2,413, so its size class shows that it is an acknowledgement | `sphinx-packet-format` |
| Gateways | Neither gateway delays. Only the three mixes hold a packet | `mix-node-delay` |
| The node trusts the header delay | A sender that wants no delay can set zero. A node does not enforce a minimum | `mix-node-delay` |
| Replay protection | Off when `unsafe_disabled` is set. Bypassed for authorised network-monitor agents. A false-positive rate of 1e-5 by default. A tag is treated as not seen when the node holds a key but no filter for its rotation | `sphinx-packet-format` |
| The replay filter is busy | A packet can wait up to 50 ms (`maximum_replay_detection_deferral`), or until 100 packets are pending, before its delay starts. 50 ms is more than three times the mean hop delay | `mix-node-delay` |
| `unsafe_disable_noise` on a node | Node-to-node links do not use Noise | Not a requirement. `nym-node/src/cli/helpers.rs`, `nym-node/src/node/mod.rs` |
| A SURB holds each hop's payload key seed | The party that holds the SURB can compute the reply payload as it looks after each hop | Not a requirement. Read from the `sphinx-packet` crate (`SURB::use_surb`) |
| Single use of a SURB | No code enforces it. `ReplySurb::apply_surb` consumes the value in one process, but the bytes can be copied. A reused SURB gives the same replay tag at the first mix, so replay protection drops it, subject to the exceptions above. This last point is read from code, not tested | Not a requirement |

## Non-goals

The specs do not claim:

- that each Nym client sends cover traffic. Defaults turn it on. Named settings turn it off.
- a constant send rate, or any statistical property of the send times. No test checks the distribution.
- that cover and real packets are the same under every setting.
- that the MAC covers the payload, or that a node rejects an unknown Sphinx header version.
- that links between mixes are always encrypted. Node-to-node TCP is upgraded to Noise, but `unsafe_disable_noise` turns that off, and the fallback to plain TCP was not checked.
- that a node enforces a minimum delay, or drops a packet whose delay is too large.
- that the hop count of a forward route is configurable. `surb_mix_hops` is in the configuration, but no code in `common/client-core` or `sdk` reads it.

## What has a witness, and what does not

This section gives the state on 2026-11-04. The last paragraph gives what changed on 2026-11-10.

| Spec | Requirements with a test in this repository | Requirements with no test |
|---|---|---|
| `sphinx-packet-format` | Frame rejection of a future version, of an unknown size tag (the test uses 255), and of an unknown packet type (`header_decoding_will_reject_future_versions`, `decoding_will_fail_for_unknown_packet_size`, `decoding_will_fail_for_unknown_packet_type` in `common/nymsphinx/framing/src/packet.rs`) | The byte totals, the three-mix route, the replay drop, the MAC rejection, the rejection of the Outfox tags 6 and 7, and of a version below 7. `header_encoding_reserves_enough_bytes_for_full_sphinx_packet_` compares a capacity with the value that sets it, so it cannot fail and is not a witness |
| `client-cover-traffic` | The cover prefix check (`is_cover_*` tests in `common/nymsphinx/cover/src/lib.rs`) | The streams, the defaults, the presets, the send schedule |
| `mix-node-delay` | None in this repository. The `sphinx-packet` crate tests that hops decode their delays | All six |

On 2026-11-04, this change added tests for five requirements, each with a comment that names the requirement:

| Requirement | Test |
|---|---|
| Every packet of one size class has the same length | `packet_size_totals`, `outfox_size_tags_are_rejected` in `common/nymsphinx/params/src/packet_sizes.rs` |
| Both cover streams are on by default, and named settings turn them off | `traffic_defaults`, `no_cover_traffic_turns_both_streams_off` in `common/client-core/config-types/src/lib.rs` |
| The sender draws one exponential delay for each hop | `a_zero_mean_gives_zero_delays`, `one_delay_for_each_hop` in `common/nymsphinx/routing/src/lib.rs` |
| Named settings remove or reduce the delay | `named_settings_set_the_delays` in `common/client-core/config-types/src/lib.rs` |
| A node drops a packet whose replay tag it has seen | `a_seen_tag_is_reported_as_replayed` in `nym-node/src/node/replay_protection/bloomfilter.rs`. A second test, `a_tag_with_no_filter_for_its_rotation_is_reported_as_not_seen`, pins the behaviour of finding 1 |

A test for the three-mix route and for the cover slots of the main stream is optional. The route is three plain lines in `random_mix_path_nodes`, and a cited excerpt shows a change in the PR comment. The cover slot needs an async harness, which is a choice for the owners of `client-core`.

### What the tests prove, and what they do not

All the tests passed on 2026-11-04. Each one goes red when the fact it guards changes:

| Test | Goes red if a change... |
|---|---|
| `packet_size_totals`, `outfox_size_tags_are_rejected` | changes a size constant, the Sphinx header layout (for example in a `sphinx-packet` upgrade), the default size, or accepts the Outfox tags |
| `traffic_defaults` | changes a default delay, or turns a cover stream off by default |
| `no_cover_traffic_turns_both_streams_off`, `named_settings_set_the_delays` | changes what a named setting does |
| `a_zero_mean_gives_zero_delays`, `one_delay_for_each_hop` | changes the zero-mean case, or the number of delays |
| `a_seen_tag_is_reported_as_replayed` | breaks the check-and-set of the replay filter |
| `a_tag_with_no_filter_for_its_rotation_is_reported_as_not_seen` | changes the behaviour of finding 1. A fix to that behaviour updates this test on purpose |

The tests do not prove:

- that delays or send times follow an exponential distribution. They check counts and zeros only.
- the three-mix route, or the cover slot of the main stream.
- what a mix does with a packet on the wire: the hold for its delay, the 10 s cap, the MAC check. The replay test covers the filter, not the packet handler.
- that the network is private. They pin facts that the privacy argument depends on. The argument itself is in the threat model.

Five of the 21 requirements had a witness on 2026-11-04. On 2026-11-10, `add-spec-trace-gate` made the CI job fail on each requirement with no `verify` marker, and witness tests were added for the other 16. `tracey query rule <id>` lists the tests of each one. Each requirement can now become `enforced` under `add-codex-assurance` (A2) when a person promotes it and adds a canary date. The limits in "What the tests prove, and what they do not" still apply to the five tests above. The new tests have the same kind of limit: for example, the cover tests do not check the loop stream's timing, because tokio's paused clock is not available to `nym-client-core`, and the `delay.only-hop-reads` test checks that no delay is in clear on a link, which does not prove that the encryption keeps it secret.

So each requirement starts as `asserted` under `add-codex-assurance` (A1). That change asks for a witness and a canary before a statement becomes `enforced` (A2). These specs are its first input.

## Tracey references

Each requirement has a Tracey id (`r[...]` above its text). The code carries `// r[impl ...]` above each function that realises a requirement, and `// r[verify ...]` above each witness test. [Tracey](https://github.com/bearcove/tracey) reads them, and the docs coverage report asks Tracey for them. The markers are the only source of the code edges (`add-spec-trace-gate`, T1), so `edges.proposed.json` holds only the threat-model edges.

## Findings for the code owners

1. **Replay protection, a key with no filter.** The node checks the filter of the rotation of the key that unwrapped the packet. A version 7 frame carries no rotation, so the node tries its primary key and then its secondary key. In `ReplayProtectionBloomfiltersInner::batch_check_and_set` (`nym-node/src/node/replay_protection/bloomfilter.rs`), the comment says that a tag with no matching filter is "most likely ... replayed ... so mark it as such". The code inserts `false` for each tag, which the handler reads as "not replayed". This case arises only when the node holds a key whose filter it does not hold. A replay from an older rotation fails earlier, at key resolution or at the MAC check.
2. **`use_legacy_sphinx_format`.** The client configuration has this field (default `true`), but no packet builder in this tree reads it.
3. **`surb_mix_hops`.** The configuration has this field, but no code in `common/client-core` or `sdk` reads it.
4. **The comment on the delay cap.** The comment near `DEFAULT_MAXIMUM_FORWARD_PACKET_DELAY` in `nym-node/src/config/mod.rs` gives a mean delay of 50 ms. The client default is 15 ms (`DEFAULT_AVERAGE_PACKET_DELAY`).
5. **Hop delays and the injected generator.** `generate_delays` in the `sphinx-packet` crate draws from `rand::rng()`, not from the generator that the preparer receives. `deterministic_route_selection` therefore fixes the route and not the delays. This can be intended. The spec records it.

## Risks

| Risk | Answer |
|---|---|
| The code changes and a spec goes stale | Each requirement has a Tracey id, and its markers move with the code. The CI gate of `add-spec-trace-gate` fails on a marker that names an unknown id or an old version, and on a requirement with no code or no test |
| A reader takes a requirement with a condition as a guarantee | The condition is in the text of the requirement, and this document lists each one |
| The threat-model links are wrong | They are proposals. The owner reviews each one (task 2.1) |

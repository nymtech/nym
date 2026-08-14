# Nym routing for Product A (wallet/RPC) and Product B (messaging)

Source: documentation MCP server only. Every claim below traces to a URL.

## Product A: wallet syncing against a third-party RPC

### 1. Threat model
Primary adversary: **L2, the destination**. The docs' own worked example is this
exact product (`/network/threat-model/examples/wallet`): "The primary adversary is
**L2, the server**. The dominant vector is **V1 identity** paired with **V3 content**."

L2 is defined at `/network/threat-model/actors#actor-L2` as the party that observes
"the source IP of incoming connections", "the fine-grained arrival time of every
request", and "the complete contents of each request". Cost to become: "Cheap and
privileged."

Vectors open (`/network/threat-model/vectors`):
- **V1 session state** - source IP, TLS/connection state, app-layer session ids. Closed by transport (relaying, short-lived connections, exit rotation).
- **V3 content** - "the endpoints and resources requested, the parameters and query values". The addresses you query ARE the content. Observable only from L2. Closed ONLY by hygiene (padding, batching, decoy requests). No transport touches it.
- **V2 timing** - arrival times at the destination. Half transport (in-transit, vs network observers), half hygiene (vs the destination).

Properties at risk (`/network/threat-model/properties`): the stated concern splits
cleanly. "Linking the addresses we query to each other" = **P2** (request-request
unlinkability). "and to us" = **P1** (request-identity unlinkability). The wallet
example's invariants: A "No user identifier links to a balance" (P1 + V2/V3
discipline); B "The server must not group one wallet's transactions" (P2 + V3
content discipline).

Also applicable: an app-specific **L1** chain-only observer.

### 2. Configuration
**Proxy mode**, and the threat model forces it: end-to-end requires that "both ends
run Nym" (`/network/threat-model/configurations/end-to-end`). The RPC endpoint is
third-party and unchangeable, so a clearnet exit exists, so L2 exists. Per
`/network/threat-model/actors#l2-is-the-primary-adversary-for-most-applications`,
L2 is displaced only when "There is no destination."

The named configuration you want is **mixnet, rotating exits**
(`/network/threat-model/configurations/mixnet-rotating`): P1 yes, P2 yes "given
per-request exit rotation".

What you can actually ship is **mixnet, single exit**
(`/network/threat-model/configurations/mixnet`): P1 against L2 **yes**, P2 against
L2 **no**.

### 3. What the transport does not solve
The single largest finding: **the configuration that answers their stated concern is
not shipped.** `/network/threat-model/configurations/mixnet-rotating` opens with:

> "Per-request exit rotation is not available in the Nym SDKs yet. This
> configuration is planned for a future release. Until then a mixnet client uses a
> fixed exit, which behaves like a single exit at the destination."

Same warning on `/network/threat-model/configurations/dvpn-multi`. Corroborated in
code: `sdk/rust/nym-sdk/src/ip_packet_client/discovery.rs::get_best_ipr` picks one
performance-weighted IPR per client, not per request.

So today: P1 closes (endpoint sees an exit gateway IP), P2 does not. Residual risk
text from the single-exit page: "Per-packet unlinkability in transit does not become
request unlinkability at the destination."

Second residual: V3 is never closed by transport at all. The wallet example:

> "The mixnet buys strong identity protection against the server and the network. It
> does not, on its own, deliver transaction unlinkability at the destination. That
> needs exit rotation plus the content discipline of the hygiene layer."

Both halves. The docs' answer is `nym-swizzle` (`/developers/swizzle`), which says:

> "**Experimental - documentation in progress.** The `nym-swizzle` library provides
> the traffic-shape primitives (request delay and range). It is unreleased, and this
> page is a stub."

Third residual: app-layer identity. `/developers/limitations#what-the-transport-never-does`
- "Logins, session cookies, API tokens, account identifiers and the contents of a
request reach the destination exactly as you sent them." An RPC API key re-identifies
you regardless of transport.

Fourth: exit-service choice. `/developers/concepts/exit-security#which-exit-your-package-uses`
- `smolmix` / `mix-tunnel` family use the **IPR**, which sees only destination IP+port
and receives traffic "under an ephemeral sender tag, so the IPR never learns your Nym
address or your Entry Gateway". The **SOCKS5** path uses the Network Requester, which
"receives your **Nym address** on every connect request... stable across sessions...
a far stronger correlator". Anonymous replies are off by default. So for Product A,
prefer the IPR-backed packages, or pass `--use-anonymous-replies`.

Partial mitigation available now: pin or vary the exit
(`/developers/concepts/exit-security#pin-an-exit`) - `preferredIpr` in
`setupMixTunnel`, `Tunnel::new_with_ipr(ipr)` natively. `preferredIpr` confirmed as
a real field via the server's own `validate_sdk_config` tool. The docs do not claim
this restores P2.

**On the hybrid.** `choose-config#which-adversary` stars "★ Hybrid (recommended)" and
`/network/threat-model/configurations/hybrid` gives it P1 yes / P2 yes at L2. It does
not apply to Product A as described: the hybrid splits *bulk* traffic onto dVPN and
*small sensitive* requests onto the mixnet, and light-wallet sync is uniformly small
requests with no bulk leg to split off. It becomes relevant only if you also do a
heavy initial chain sync, which the limitations table puts at mixnet-No / dVPN-Yes.
Note also that the hybrid's P2 verdict reads "sensitive requests unlinkable to bulk
and to each other (per-request rotation)" - it inherits the same unshipped rotation
dependency, and unlike the two rotation pages it carries no availability banner. Its
"Requires" line is "both dVPN and mixnet stacks in one process", and the dVPN leg is
native-only (`nym-smoldvpn` has no wasm build), so it has no browser form at all.

### 4. Packages (`/developers#choosing-a-package`)
- Native Rust: **`smolmix`** (`TcpStream`/`UdpSocket` over the mixnet via IPR). Alternative `nym-sdk` SOCKS5 module, weaker by default per above.
- Browser: **`@nymproject/mix-fetch`** (built on `@nymproject/mix-tunnel`, wasm inlined, ESM only). `/developers/demos/ens` is the exact working recipe: swap `ethers` `FetchRequest.getUrlFunc` for `mixFetch`. That page names the two-question routing itself.

---

## Product B: messaging, both ends our software

### 1. Threat model
This is a go/no-go on one architectural fact, and you did not tell me which side of it
you are on. You said you control the client on both sides. You did not say there is no
server.

**If any server of yours routes messages by account, you have not built end-to-end and
the social graph survives.**
`/network/threat-model/examples/messaging` invariant A:

> "P1 everywhere, but that alone is not enough: a server routing by account learns
> the graph regardless of transport. Only end-to-end or a metadata-private protocol
> removes it."

and the protected-case reading: "The server (L2) still gets **P2 no**: it
authenticates your account and routes each message to a named recipient. So it
reconstructs the social graph regardless of transport."

**If clients address each other directly over the mixnet**, then per
`/network/threat-model/actors#l2-is-the-primary-adversary-for-most-applications`,
"End to end, the L2 adversary does not exist." The adversary that remains is
**L3G, the global network observer** ("also called the global passive adversary"),
plus **L3L** locally. Dominant vector per the messaging example is **V2 timing**:
"request and response bursts pair the two conversing clients". App-specific **L1**:
a directory observer reading public handles.

Properties (`/network/threat-model/configurations/end-to-end`): P1 vs L2 yes,
P2 vs L2 yes, P1 vs L3L yes, P1 vs L3G **partial**.

### 2. Configuration
**End to end.** Forced, because it is the only configuration in which no untrusted
destination exists, and because invariant A says only end-to-end (or a
metadata-private protocol) removes the server-side social graph. `choose-config`
row: "I talk to another Nym client, not a clearnet server | No L2 exists | End to
end." Nothing on `/developers/concepts/exit-security` applies: "Tools where both ends
run a Nym client... never exit the mixnet at all."

### 3. What the transport does not solve
- **L3G is only partial.** Residual risk on the end-to-end page: "Long bulk flows weaken per-packet correlation resistance (open question)." File attachments and media are that case.
- **Your own discovery/directory server reinstates L2, and this is where teams do it without noticing.** Clients cannot talk end-to-end until each knows the other's Nym address. If they fetch it from a server of yours keyed by account, that server sees who asked for whom, which is the social graph, and invariant A applies to it in full. The docs never say this outright. I derived it from invariant A; treat it as an inference, not a quote. It is the single decision that determines whether Product B gets the end-to-end verdicts at all.
- **The Nym address is a stable pseudonym.** `/network/reference/addressing#address-format`: the address embeds your gateway's identity key. `#privacy-considerations`: "For persistent identity across sessions, store your keypairs and re-register with the same Gateway." `/network/mixnet-mode/anonymous-replies#the-problem`: if Bob sends to Alice's address directly "he learns it... Bob now knows which Gateway node Alice's client is using." Mitigation the docs give: reply via **SURBs / `sender_tag`**, which is on by default in both SDKs. `/developers/rust/tour#reply-anonymously-with-surbs`: "The replying side never learns where the reply is going."
- **The Entry Gateway knows your IP.** `/network/infrastructure/nym-nodes#node-modes`: "Entry Gateways know the client's IP address but cannot see message contents or final destinations."
- **SURB hoarding.** `/network/mixnet-mode/anonymous-replies#security-considerations`: a malicious receiver can hoard SURBs and return them at once to correlate at the sender's gateway. Active, targeted, limited payoff.
- **Content still needs E2EE.** `/network/threat-model/comparisons#nym-vs-end-to-end-encryption`: "Nym and E2EE are complementary. E2EE protects the message content; Nym protects the metadata around it."
- **No ordering.** `/developers/typescript/quick-start#anonymous-replies-surbs`: "assumptions like 'the third message will arrive third' don't hold."

### 4. Packages
- Native Rust: **`nym-sdk`** - Mixnet module (`MixnetClient`, `send_plain_message`, `send_reply`), `MixnetClientBuilder` + `StoragePaths` for persistent identity, `ClientPool` for bursty traffic (framed as a latency feature, not a privacy one).
- Browser: **`@nymproject/sdk`** (`createNymMixnetClient`). Variants per `/developers/typescript#packages`: `sdk`, `sdk-full-fat` (inlined, tens of MB), plus CJS equivalents. `forceTls: true` required on HTTPS pages.
- Explicitly NOT the mix-* packages: "If you need HTTP, DNS, or WebSocket connections through the mixnet to third-party services, this isn't the right SDK."

---

## 5. What the docs warn you not to conclude

`/network/threat-model/two-layer-model#the-category-error`:

> "Mixing does not change this. Mixing delays and cover traffic change what a
> **network observer** can infer. The destination sees only what arrives, and when it
> arrives. No amount of mixing protects you against the server you are talking to."

and `/network/threat-model/actors#what-each-choice-buys`:

> "Selecting the L3G configuration because it sounds stronger, without the hygiene
> that L2 demands, leaves the destination-facing vectors wide open. That is the
> category error."

and the sharpest one, `/network/threat-model/two-layer-model`:

> "Same transport. Opposite outcomes. The mixing bought everything against the
> network and nothing, on content, against the destination."

Yes, my first instinct would have made it. For Product A I would have said "route the
RPC over the mixnet, the operator now sees an exit gateway, profiling solved." That is
wrong on both counts the customer actually named: the query contents (V3) arrive
untouched, and with a fixed exit the requests stay linkable to each other (P2 fails).

Secondary warning, `/developers/limitations#what-the-transport-never-does`: "A fixed
exit is a linking key."

## 6. Is either workload a fit?

`/developers/limitations#is-my-workload-a-fit`, quoted rows:

| You want to | Mixnet | dVPN |
|---|---|---|
| Fetch a page, call an API, submit a transaction | Yes | Yes |
| Send messages between two parties | Yes | No |
| Light-client sync: compact blocks, small repeated requests | Yes | Yes |
| Download a large file, sync a whole chain | No | Yes |
| Real-time gameplay, live position updates | No | No |
| Round-trip budget well under a second | No | Maybe |

**Product A: fits if it is light-client sync, does not fit if it is full-chain sync.**
That row is decisive and it is the row the docs wrote for exactly this case. Budget:
`#the-default-sending-rate` - "roughly 55 packets per second... the ceiling is on the
order of 100 KB/s", sent constantly whether idle or not.

**Product B: fits, with a latency floor.** `#latency-is-the-mechanism`: "Each mix node
holds a packet for an average of 15 ms (`DEFAULT_AVERAGE_PACKET_DELAY`)... and the path
is five hops... Remove the delay and you remove the protection: there is nothing to
tune, because the delay *is* the product." Asynchronous chat is fine; typing
indicators, read receipts, voice/video are not. Note dVPN is not an escape hatch here:
"dVPN is a tunnel to a destination. It does not connect two peers to each other."

Also `#it-costs-the-same-when-idle` and `#capacity-is-a-budget-not-a-limit-you-discover`
- if you run any always-on service client, it pays near-full bandwidth continuously,
and every reply comes out of one packet budget.

---

## 7. How I found this

23 tool calls. `L` = followed a link I had already been given. `G` = guessed a query
because nothing linked me there.

| # | Call | How |
|---|---|---|
| 1 | `--list` | - |
| 2 | search "threat model adversary linkability unlinkability" | G (cold start) |
| 3 | `get_section` `/threat-model/examples/wallet` | L (from #2) - returned 3 lines, intro only |
| 4 | search "choose a configuration end-to-end vs proxy mode" | G |
| 5 | search wallet sections | G (page-targeted, to recover anchors) |
| 6 | search messaging sections | G (same workaround) |
| 7 | search "V1 V2 V3 linkage vectors" | L (`/threat-model` hub named the page) |
| 8 | search "mixnet single/rotating exit configurations" | **G** - see below |
| 9 | search "threat actors L1 L2 L3L L3G" | L |
| 10 | search "what Nym cannot do / unsuitable workloads" | L (`/developers` intro) |
| 11 | search "nym-swizzle baseline hygiene" | L (two-layer-model) |
| 12 | search "package matrix" | L (`/developers`) |
| 13 | search "category error / common mistake" | L (actors page named it) |
| 14 | search "entry gateway / stable Nym address" | G |
| 15 | search "exit security, which exit your package uses" | L (`/developers` end-to-end-or-proxy) |
| 16 | search "blockchain RPC / JSON-RPC guide" | **G** - see below |
| 17 | search "client pool / rotate identity per request" | G (testing a hypothesis; it failed) |
| 18 | search "SURBs / sender tag" | L (end-to-end config page) |
| 19 | search "Nym vs other systems" | L |
| 20 | search "TypeScript SDK end-to-end messaging" | L (package matrix) |
| 21 | search "choose a defence matrix" | L |
| 22 | `search_code` "rotate exit per request" | G (verifying a docs claim against code) |
| 23 | search "hybrid configuration" + `validate_sdk_config` | L / verification |

### Did the docs lead me from "what am I defending against" to "which package"?

Mostly yes, and deliberately so. `/developers` opens with "**Decide what you are
defending against first.** Which package you build with follows from which network
configuration you need, and that follows from your threat model," and points at
`choose-config`. `/developers/limitations#if-none-of-this-rules-you-out` closes the
same loop in the other direction. That spine is real and it is better than most
projects manage.

**But the worked examples dead-end.** `/network/threat-model/examples/wallet#where-to-go-next`
and the messaging equivalent link only to choose-config, the two-layer model, and a
deep dive. Neither ever links to a package, or to `/developers` at all. The pages
written for exactly my two products are the pages that stop one hop short. I had to
re-enter the funnel at choose-config and walk down manually.

### Where I had to guess which page to look at next

- **The configuration pages (#8).** `choose-config#which-adversary` names "mixnet rotating" in a table cell as prose. I could not get from there to `/configurations/mixnet` or `/configurations/mixnet-rotating` except by guessing a query. Those two pages carry the single most important fact in this whole answer.
- **The ENS demo (#16).** `/developers/demos/ens` is the actual working RPC-over-mixnet recipe and it is the best page for Product A. Nothing in the wallet worked example, in choose-config, or in the package matrix links to it. I found it by guessing "blockchain RPC JSON-RPC". The link runs the other way only: the demo links back to `/developers` and `choose-config`.
- **Addressing and gateway visibility (#14).** For Product B the entry gateway matters, and no threat-model page pointed me at `/network/reference/addressing` or `/network/infrastructure/nym-nodes`.

## Where the docs failed me

**1. The most misleading thing in the docs, for Product A.**
`choose-config#which-adversary` answers "Stop the service linking my requests to each
other" with "Rotate exits: dVPN multi-exit, or mixnet rotating." No caveat. Both linked
pages open by saying the configuration is not available in the SDKs. A reader who
trusts the routing table walks away believing their exact problem is solved. It is not,
and you only find out one click later. The `/configurations/hybrid` page compounds it:
its P2 verdict depends on "per-request rotation" and it carries no banner at all.

**2. `nym-swizzle` is load-bearing and does not exist.**
It is the recommended answer for the V2/V3 vectors on `choose-config`, `vectors`,
`two-layer-model`, `mixnet-mode`, `dvpn-mode`, `limitations`, `exit-services`, and the
ENS demo. Its own page says: "Experimental - documentation in progress... It is
unreleased, and this page is a stub." For both products, the layer the docs insist you
owe is the layer you cannot install. The docs are honest about this on the stub page
and silent about it everywhere they recommend it.

**3. The entry gateway has no L-number.**
`/network/infrastructure/nym-nodes#node-modes` says "Entry Gateways know the client's
IP address." For Product B end-to-end, where L2 has been removed by construction, the
entry gateway is the most significant party that still knows something about you, and
the actors taxonomy (L1, L2, L3L, L3G) has no slot for it. This is a hole in the model,
not a lookup failure on my part.

**4. `get_section` on a bare page URL returns only the intro.**
Three lines for the wallet page. There is no tool to enumerate a page's anchors, so
recovering a page's sections means firing a page-targeted `search_docs` and hoping the
ranking surfaces them. Two calls per page, and no guarantee of completeness. I cannot
be certain I saw every section of the pages I "read".

**5. No worked example for the case Product B is actually in.**
`/examples/messaging` is written around a chat server. There is no worked example for
"both ends are ours, no server". I assembled Product B from the end-to-end configuration
page plus one sentence of messaging invariant A plus the addressing and SURB pages, none
of which link to each other for this purpose.

### Still cannot determine

- **Whether a persistent Nym address is a P1 or P2 problem.** `/network/reference/addressing#privacy-considerations` says to "store your keypairs and re-register with the same Gateway" for persistent identity and assesses none of it. Meanwhile `properties#the-asymmetry-matters` warns that "One attributed request anywhere in a pseudonymous profile retroactively attributes the whole profile." Nobody connects these two pages. For a messaging app, where a stable address is a product requirement, that gap is not academic.
- **Whether varying `preferredIpr` per session buys any P2 at all**, and at what frequency. The exit-security page frames pinning purely as operator/jurisdiction choice and availability trade, never as a linkability control. So I do not know whether re-pinning a different IPR each session is worth anything.
- **What "not available in the SDKs yet" means for timelines**, or whether an application can approximate rotation by cycling clients. `get_best_ipr` in `sdk/rust/nym-sdk/src/ip_packet_client/discovery.rs` picks one performance-weighted IPR per client, and `ClientPool` is documented as a latency feature with no privacy claim, so nothing in the docs authorises cycling clients as a rotation substitute.
- **Whether Product A's V3 problem is even solvable in principle** with a single fixed destination. A wallet's queries name the addresses whatever exit they arrive from. `vectors#vector-V3` offers "padding, batching, and decoy or overlapping requests" and the crate that implements them is unreleased, so this is unanswerable today.

Tools I did not use: `network_summary`, `circulating_supply`, `chain_status`,
`list_gateways`, `get_gateway`. None bore on the question.

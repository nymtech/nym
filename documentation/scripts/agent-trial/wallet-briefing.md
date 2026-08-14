# Nym for a desktop Rust wallet: what I found

I started knowing nothing beyond the name. Everything below comes from the docs MCP server only.

---

## 1. What I would tell you

### The one-line version

Nym is a mixnet: a 5-hop, fixed-size-packet, delay-and-cover-traffic network that hides **who is talking to whom** at the network layer. For your wallet it does exactly one thing well — it stops the server you sync against, and anyone watching your line, from tying a balance or a broadcast to your IP address. It does **not** make your wallet private on its own, and the docs are unusually blunt about that.

### The docs already contain your exact use case

`https://nym.com/docs/network/threat-model/examples/wallet` is a worked threat model for a light wallet syncing against `lightwalletd`. Its framing:

- The primary adversary is **L2, the server you sync against**. Not your ISP, not a global observer — the RPC/indexer endpoint.
- Two invariants the wallet owes its user:
  - **A. Identity to balance** — no user identifier links to a balance, even approximately.
  - **B. Transaction grouping** — the server must not group one wallet's transactions together.
- Unprotected today: the server sees your IP, every request, every broadcast. Both invariants fail with zero adversarial effort.

### What Nym buys you, precisely

| | With the mixnet | Still broken |
|---|---|---|
| Server learns your IP | Closed. It sees an exit gateway IP. | — |
| ISP / local observer learns what you're doing | Closed. Constant-size Sphinx packets at a Poisson rate plus cover traffic; it cannot see destination, volume, or activity. | — |
| Server groups your requests into one wallet history | **Not closed by a fixed exit.** A fixed exit behaves exactly like a single-exit VPN. | Needs **exit rotation per request**. |
| Server infers your resume height / birthday from block ranges | **Not closed at all.** | Needs request-shape discipline. |
| Server links a broadcast to the sync session seconds earlier | **Not closed at all.** | Needs delayed, decoupled broadcast. |

The docs call this **the two-layer model**: transport (mixnet) and baseline hygiene (what your app puts on the wire and when). Transport alone is explicitly declared insufficient, and they say so on the front door of the developer docs, which is a good sign.

### Which packages you'd actually use

The developer front door (`/docs/developers`) resolves package choice on two axes: runtime, and end-to-end vs proxy. Native Rust desktop, talking to a third-party server, puts you in one cell:

| Package | Version / status | Role |
|---|---|---|
| **`nym-smolmix`** | `1.21.5-rc.3`, MSRV 1.87 | The main one. Userspace smoltcp gives you `TcpStream` / `UdpSocket` implementing tokio `AsyncRead`/`AsyncWrite`, so `tokio-rustls`, `hyper`, `tokio-tungstenite` stack on top unmodified. Exits via the **IP Packet Router (IPR)**. |
| **`nym-sdk`** SOCKS5 module | `1.21.5-rc.3` | `Socks5MixnetClient` — a local `socks5h://127.0.0.1:1080` listener. Exits via the **Network Requester (NR)**. Zero-rewrite path if your wallet already has proxy settings. |
| **`nym-swizzle`** | **git only, not on crates.io** | The hygiene layer. `Delay` (decorrelate an action from its trigger) and `Range` (fetch an index range as overlapping, shuffled chunks with a jittered/snapped start). |
| **`nym-swizzle-zcash`** | git only | Chain-specific policy: quantised sync ranges on a grid never finer than one day of blocks, and broadcasts delayed by an exponential draw with mean 144 blocks / cap 576, matching ZIP-318. Ships a live example against a public `lightwalletd`. |
| **`nym-smoldvpn`** | native only | 2-hop WireGuard dVPN. Line-rate, no timing protection. The docs' answer for bulk transfer. **Requires zk-nym ticketbooks paid in NYM.** |
| `nym-sdk` Mixnet/Stream | | Only if you control the server too. Then there is no exit gateway and no L2 adversary at all — the strongest position the network offers. |

### The costs you must know before you commit

**Latency is the product, not a bug.** 15 ms average hold per mix node (`DEFAULT_AVERAGE_PACKET_DELAY`), five hops. The docs state flatly: remove the delay and you remove the protection, there is nothing to tune, and this is not an implementation that will improve.

**A resident client is expensive.** The default client emits ~50 real + ~5 cover packets per second, ~2 KB each, **constantly, whether or not you have anything to send**. That is ~100 KB/s. Deriving from their own named constants (`DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY`, `DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY`), a wallet left open all day costs roughly **8–9 GB/day of traffic while idle**. This is arithmetic on config constants, not a measurement — the docs deliberately quote no benchmarks. `disable_main_poisson_packet_distribution` and `disable_loop_cover_traffic_stream` exist, and the docs say using them gives back exactly the property you came for.

**Cold start is seconds.** `MixnetClient` creation does gateway handshake, key generation and topology fetch — "several seconds". Then the first request to a host pays TCP and TLS handshakes carried as IP packets over the mixnet, several sequential round trips. smolmix keeps the connection warm afterwards.

**Bulk sync is the weak case, twice over.** Slow, and the anonymity is weaker — a long sustained flow gives an observer correlated material, and how far such flows can be correlated is described as an open question. Their own fit table says "download a large file, sync a whole chain: **No**" for the mixnet, "Yes" for dVPN, while "light-client sync: compact blocks, small repeated requests" is a **Yes**.

**Anonymity set is small right now.** Live query: **850 bonded nodes, of which only 60 are mixnodes**, 80 entry and 100 exit gateways. Sixty mixing nodes across three layers is a thin crowd, and the docs note mixing degrades with low network traffic.

### What you have to build yourself — and the sharpest finding

**Exit rotation per request has no supported implementation on the native Rust path.** This is the thing I'd want you to see most clearly:

- Both the wallet page and the `mixnet-rotating` configuration page state the requirement as *rotate the exit per request* to restore request unlinkability at the destination.
- The native mechanism is constructing a fresh tunnel: `Tunnel::new()` auto-discovers a performance-weighted IPR (`get_best_ipr` picks with `choose_weighted` over exit gateways). `Tunnel::new_with_ipr(addr)` pins one.
- Creating the underlying `MixnetClient` takes several seconds, and `TunnelBuilder`'s own doc comment says: *"deeper builder integration with `MixnetClientBuilder` requires upstream SDK changes to expose `IpMixStream` internals."*
- `ClientPool` exists and solves exactly this latency problem — but it hands out `MixnetClient`s, and there is no documented path to feed a pooled client into a smolmix `Tunnel`.

So the docs prescribe a security property whose native implementation they have not provided. Per-request rotation means paying cold tunnel setup **plus** cold TLS handshake on every request, with no pooling. Budget for this, or accept single-exit and know that invariant B fails.

Other build-it-yourself items:

- **Destination splitting.** `nym-swizzle` explicitly refuses to do it: *"Sync from one server and broadcast through another. Never broadcast over the sync session."* Two endpoints, two tunnels, your routing code.
- **Range widening.** swizzle never extends the *end* of a range because it has no view of the chain tip. Widening is yours.
- **Deduplication.** Overlapping chunks re-fetch deliberately.
- **Idempotent retry.** The mixnet guarantees delivery but not ordering; packets arrive out of order by design. The Railgun demo's pattern generalises: fix the tx hash before broadcasting so a dropped response can be re-sent or detected as already-on-chain.
- **Persisting a broadcast schedule.** swizzle-zcash's delays run longer than a process lifetime, so the schedule is data you persist and resume; the transaction is built only when the delay fires.
- **Rate-limit handling.** Both demos warn that public RPC endpoints flag shared exit IPs with 403/429 and the remedy is a fresh exit.

### New secrets on disk

A persistent client writes `private_identity.pem`, `private_encryption.pem`, `ack_key.pem`, `gateway_shared.pem`, `persistent_reply_store.sqlite`, `credentials_database.db`. Your wallet already has a keystore, a backup story and probably an encryption-at-rest story; this key material has to fit into all three, or you use ephemeral clients and accept the several-second bootstrap each time. Note the flip side: a **persistent Nym address is itself a stable cross-session correlator**. It embeds your entry gateway identity, and with the SOCKS5/NR path the Network Requester receives it on every connect request unless you turn on anonymous replies, which are **off by default**.

### IPR vs Network Requester — pick deliberately

| | smolmix (IPR) | SOCKS5 (NR) |
|---|---|---|
| Exit sees | Destination IP + port | Destination **hostname** + port (with `socks5h://`) |
| Exit learns your Nym address | No — ephemeral sender tag | **Yes, by default.** Stable across sessions, names your entry gateway |
| DNS | You resolve your own, over the tunnel as UDP | NR resolves for you; plain `socks5://` leaks the lookup to clearnet |
| Effort | Rewrite the transport layer | Point existing config at a local proxy |

The NR default is a genuine trap: the docs say plainly that left off, *"the Network Requester can link every request you ever make through it, and can see which gateway you entered by."* For a wallet that is invariant B failing at the exit rather than the destination.

### No token needed — with one asterisk

zk-nym ticketbooks (75 NYM each, three types, 50 tickets, 7-day validity) are how gateway access gets paid for. But: *"SDK integrations currently connect to the Mixnet without requiring credentials."* So the mixnet path ships today with no token in your product. **`nym-smoldvpn` is different** — it funds ticketbooks by depositing NYM from a mnemonic. If your answer to bulk initial sync is "use dVPN mode", you have just added a token acquisition flow to a wallet, which is a product decision, not an engineering one.

The word "currently" is load-bearing and I could not find anything telling me what happens when that changes.

---

## 2. What I'd need to ask you

**Q1 — Which chain?** *Everything hinges on this.* If it's Zcash, `nym-swizzle-zcash` hands you a researched, ZIP-318-aligned policy layer for free. If it's Bitcoin, Ethereum, Monero or anything else, that evaporates and you inherit "design your own quantisation grid and your own broadcast-delay distribution" — cryptographic-adjacent design work, not integration work. The docs' tidiness on Zcash should not be read as generality; the page says outright that `nym-swizzle` "takes no view on what a good grid or a good delay is for your chain."

**Q2 — Do you control the server you sync against?** If yes, the end-to-end configuration removes the exit gateway and the L2 adversary entirely, and you'd use `nym-sdk`'s Stream module rather than smolmix. That is a strictly stronger and structurally different answer. If it's a public `lightwalletd` / RPC, you're in proxy mode.

**Q3 — Do you have two or more independent endpoints?** swizzle's destination-splitting requirement ("never broadcast over the sync session") is *impossible to satisfy with one provider*. If you have one, the recommendation has a hole in it that no library closes.

**Q4 — What port does your endpoint serve on, and does it speak gRPC?** See section 4 — I could not establish the exit policy from the docs, and I found no guidance at all on HTTP/2 or gRPC over the mixnet, which is what `lightwalletd` actually speaks.

**Q5 — Is the wallet resident or launch-and-sync?** ~100 KB/s of constant cover traffic is fine for a five-minute sync and hostile for an app that lives in the system tray. It changes whether you keep a client warm or tear it down.

**Q6 — Full initial sync, or incremental only?** Bulk chain sync is on the "No" side of their own fit table. If first-run sync is gigabytes, you need a two-mode design (dVPN for the backfill, mixnet for ongoing) and Q7 follows.

**Q7 — Is shipping a NYM-token flow acceptable?** Only bites if the answer to Q6 pulls in `nym-smoldvpn`.

**Q8 — Are you willing to accept an RC dependency and a git dependency?** See section 4.

**Q9 — Who is the adversary you actually care about?** If it's the server, exit rotation and hygiene matter and mixing delays buy you very little. If it's a network observer, the mixing is the whole point and the hygiene layer buys you little. The docs' actors page says these two answers "barely overlap" and calls picking the mixnet because it sounds stronger, without the hygiene, a category error. This is also the question that decides whether Tor is a better fit — Tor is faster and adds no mixing delay, and their own comparison page concedes it "may be a better fit for general browsing."

---

## 3. The route I took

Every step marked **GUESS** is one where nothing I had linked me onward and I invented a query.

| # | Where | How I got there |
|---|---|---|
| 1 | `--list` | Given in the brief |
| 2 | `/docs/operators/introduction`, `/docs/network`, `/network/mixnet-mode` | **GUESS**: "what is Nym mixnet overview introduction". Top hit was the *operator* guide — wrong audience for a dev |
| 3 | `/developers/rust`, `/developers` | **GUESS**: "Rust SDK integrate application with mixnet" |
| 4 | `/developers` (front door) | LINK from step 3 |
| 5 | `/network/threat-model/examples/wallet` | **GUESS**: "choose a defence which configuration threat model". The single most valuable page in the whole trial, and I hit it by luck |
| 6 | wallet page | LINK — returned intro only (see section 4) |
| 7 | wallet page anchors + `/developers/swizzle` | **GUESS**: "wallet sync lightwalletd adversary transaction broadcast IP" |
| 8 | `/developers/limitations` anchors | **GUESS**: "limitations bandwidth sending rate latency bulk transfer" |
| 9 | `/developers/swizzle` | LINK from front door |
| 10 | `/developers/smolmix` | **GUESS**: "smolmix TcpStream over mixnet userspace IP stack" |
| 11 | `/network/infrastructure/exit-services` | **GUESS**: "exit policy allowed ports blocked IPR network requester" — got the trust model, never got the ports |
| 12 | zk-nym pages, `/developers/nymvpncli` | **GUESS**: "bandwidth credentials ticketbook zk-nym NYM token required" |
| 13 | `/network/threat-model/configurations/mixnet-rotating` | **GUESS**: "rotate exit per request random IPR client pool" |
| 14 | `/developers/rust/client-pool` + tour | LINK from step 13's search results |
| 15 | tour `#persist-your-identity`, `/network/reference/addressing` | **GUESS**: "client storage persistent identity keys ephemeral" |
| 16 | operator iptables dumps | **GUESS**: "which ports are allowed through the exit gateway" — failed |
| 17 | `/developers/rust/socks5` | **GUESS**: "Rust SDK socks5 module embed proxy" |
| 18 | `/developers/concepts/exit-security` | LINK from step 17 — the page I most needed and had no route to before |
| 19 | `network_summary` (live) | Tool from `--list` |
| 20 | `/developers/mix-dns` | **GUESS**: "DNS resolution leak smolmix" — browser-only answer |
| 21 | (nothing) | **GUESS**: "gRPC HTTP/2 over mixnet tonic" — failed |
| 22 | `/network/mixnet-mode/anonymous-replies`, `/network/reference/acks` | **GUESS**: "reliability message loss retries SURB" |
| 23 | `/developers/limitations#it-costs-the-same-when-idle`, `/developers` matrix | **GUESS**: "desktop always-on bandwidth cost" |
| 24 | `/developers/demos/ens`, `/developers/demos/railgun` | **GUESS**: "blockchain RPC wallet privacy guide" |
| 25 | `validate_sdk_config` probe | Tool from `--list` |
| 26 | `/network/threat-model/comparisons` | **GUESS**: "Nym vs Tor VPN i2p" |
| 27 | `/network/threat-model/configurations/end-to-end` | **GUESS**: "end-to-end both sides run nym client" |
| 28 | (nothing) | **GUESS**: "gateway bandwidth cap daily allowance" — returned literally zero results |
| 29 | `/developers/rust/mixnet/troubleshooting` | LINK I'd been holding since step 23 and had not followed |

**Count: 19 guesses, 5 link-follows.** The linking is good *once you are inside a cluster* — `/developers` → swizzle → limitations, and socks5 → exit-security, are excellent. What is missing is any route from "I have a crypto wallet" to the wallet threat model. Nothing in the developer docs points at `/network/threat-model/examples/wallet`. I found the most important page by guessing.

---

## 4. Where the docs failed me

**A `get_section` on a page URL returns only the intro.** The wallet page, `/developers/limitations` and `/developers/swizzle` each gave me two or three sentences. Getting the actual content required guessing a `search_docs` query that happened to surface the anchors. There is no way to enumerate a page's sections. This is a retrieval-layer failure that makes deep-reading a page a matter of luck.

**The exit policy is not in the docs.** This is the tightest possible constraint — if your endpoint's port cannot leave the mixnet, nothing else matters — and I could not determine it. What I got instead was pages of raw `iptables` output pasted into an *operator* configuration page. That dump does allowlist Zcash 8232–8233, Monero 18080–18081, and 443, so chain ports are contemplated. But it is truncated, it is the **WireGuard** chain rather than the mixnet policy, and the authoritative list lives at an off-docs URL (`nymtech.net/.wellknown/network-requester/exit-policy.txt`) I am not permitted to fetch. **Actionable version for you: check your endpoint's port against that file yourself, because the docs site does not carry it.**

**No answer on whether a per-client bandwidth cap applies.** "gateway bandwidth cap per client daily allowance" returned **nothing**. The only trace is an operator changelog entry, "Add 1GB/day/user bandwidth cap", with test scenarios about bandwidth resetting at midnight and clients dropping off a peer list after 3 days. The language ("private IP", "peer list") reads like WireGuard accounting, not mixnet. But if it *does* apply to mixnet clients, an always-on wallet exhausts 1 GB in about two and a half hours of doing nothing, and the entire architecture flips to short-lived per-sync clients. I cannot resolve this and no non-changelog page addresses it.

**A live contradiction on credentials.** The zk-nym overview says "SDK integrations currently connect to the Mixnet without requiring credentials." The diagnostic tool page says its registration test "spends a credential, which is why it lives in a separate command." Both cannot be uniformly true, and nothing tells me which case I'm in.

**Nothing on gRPC or HTTP/2 over the mixnet.** The wallet threat model is built entirely around `lightwalletd`, which is gRPC over HTTP/2. Searching for it returned only a note on the **deprecated** TcpProxy module observing that ordering matters "whenever a parser cares about frame boundaries (gRPC over protobuf, HTTP, TLS)". The docs walk you to the wallet case and then have nothing to say about the protocol that case actually uses.

**An internal maintenance comment is rendered into the published swizzle page.** The output includes a `{/* MAINTENANCE NOTE, delete when it no longer applies. */}` block with a four-step checklist for Nym staff. Cosmetic, but it is a page shipped unfinished.

**Toolchain maturity is a single coherent risk, presented as three footnotes.** The tutorial pins `nym-sdk = "1.21.5-rc.3"` — a release candidate. smolmix warns its "API may still change between minor releases. Pin a version rather than tracking the latest." `nym-swizzle` and `nym-swizzle-zcash` are **git dependencies only**, not published. For a desktop wallet with a release process and users who expect reproducible builds, "the hygiene layer we tell you is mandatory is a `git = ...` dependency on `develop`" is a supply-chain problem the docs mention in passing and never treat as one.

**The docs prescribe exit rotation and do not provide it natively.** Detailed in section 1. `TunnelBuilder`'s own source comment concedes the missing SDK surface. This is the largest gap between what the security guidance requires and what the libraries deliver.

**`validate_sdk_config` is useless to you.** It validates `SetupMixTunnelOpts`, the TypeScript/wasm `setupMixTunnel` shape. There is no equivalent for the native Rust path. Similarly, `mix-dns` is the only page about DNS-over-mixnet and it is browser-only; for native smolmix the DNS story exists only as a line in an examples table.

**"A general integration guide is coming soon"** for `nym-smoldvpn` — an explicitly promised, absent page, and it sits on the exact path you'd need for bulk initial sync.

**Search leaks operator content badly.** Roughly a third of my results were VPS setup, node bonding, jurisdiction advice and changelog noise. Two queries returned nothing but `iptables` dumps.

---

## 5. What I think I might have missed

Things I now believe are probably documented somewhere and to which I never found a route:

- **Entry gateway selection and pinning.** I found thorough guidance on pinning the *exit* (`/developers/concepts/exit-security`). Your entry gateway is baked into your Nym address, is visible to the Network Requester, and is a stable correlator — but I found nothing on choosing, pinning, or rotating it. Given how carefully exit selection is treated, entry selection is almost certainly documented and I never got there.
- **What to do when the entry gateway WebSocket is blocked.** Clients connect to a gateway over WS/WSS on 9000/9001. I saw QUIC bridge references for smoldvpn and in operator config, never for the mixnet client. A censorship-circumvention story presumably exists.
- **Running your own `lightwalletd` or RPC behind a Nym client.** The docs establish that end-to-end is the strongest configuration and that it needs both sides running Nym. Nobody connects that to the wallet case, where "run your own indexer with a Nym client in front" would eliminate L2 entirely. That page ought to exist; I found no route to it.
- **How exit rotation interacts with a persistent client identity.** Rotating exits while keeping one entry gateway and one Nym address may leave a correlator that defeats the rotation. Not addressed anywhere I looked.
- **Nym's own Rust reference implementations.** NymVPN is a Rust desktop application doing all of this. The operator and CLI docs exist; whether there is developer-facing "here is how the real client structures its lifecycle" material, I never found.
- **The deep-dives section.** `/network/deep-dives/packet-anatomy` and `/network/deep-dives/mixing` were repeatedly linked from "where to go next" footers. I never opened them, and packet anatomy in particular likely bears on the 2 KB payload and fragmentation behaviour that determines how a gRPC frame maps onto Sphinx packets.
- **A cost or capacity model.** Nothing tells me what happens to 100 exit gateways if a wallet with N users all rotate exits per request. The docs note "few users per exit shrinks the anonymity set" and elsewhere that exits get rate-limited by RPC providers, but the two observations are never joined into guidance for a wallet shipping to many users.

**On your real question** — do these docs lead a developer to everything they need? Partially, and the gap is characteristic. The *conceptual* layer is genuinely excellent: the threat model is honest, "What Nym cannot do" exists and is prominent, and the two-layer model stops you thinking transport is the whole answer. But the wallet threat model, which is the page written for me, is unreachable from the developer docs. And the route from "here is the property you need" (exit rotation, destination splitting, per-chain policy) to "here is the code that does it" runs out well before you reach code. I got to the important places, but nearly always by guessing.

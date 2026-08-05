# Tasks: fix-dvpn-review-6953

## 1. Upstream bandwidth crates (SEPARATE PR against develop)

- [x] 1.1 `nym-bandwidth-fetcher`: add `allowed_ticketbook_types: Option<HashSet<TicketType>>` to `NyxdCredentialFetcher` + additive `with_allowed_ticketbook_types()` builder; guard between recovery and `make_deposit` in `fetch_ticketbooks` (recovered deposits always returned); new `NyxdFetcherError::TicketbookTypeNotAllowed { requested, allowed }` with `kind()` → `FetcherErrorKind::Other`
- [x] 1.2 `nym-bandwidth-controller`: add `with_managed_ticket_types(Vec<TicketType>)` builder on `BandwidthController` (field, not config); use the managed set in the periodic sweep (`controller.rs:158`) and fetcher-install restock (`controller.rs:253`); default = `AvailableTicketbooks::ticketbook_types()`
- [x] 1.3 Unit tests: vetoed type makes no deposit; recovery bypasses veto; unrestricted fetcher unchanged; scoped sweep spawns fetches only for managed types; readiness waiter on vetoed type errors instead of hanging
- [x] 1.4 Open the PR against `develop`, request Simon Wicky's review; after merge, rebase `feature/nym-sdk-dvpn` onto it

## 2. Session architecture (nym-sdk-session)

- [x] 2.1 Migrate `Session::new` to the running-controller pattern (client-core `start_bandwidth_controller` as reference): one-shot initial provisioning on the not-yet-running controller, then spawn `run()`, hold `BandwidthControllerRequestSender`; reconcile `ShutdownToken` vs `CancellationToken`; Session shutdown awaits the controller's cleanup path
- [x] 2.2 Add external-provider escape hatch (caller-supplied `BandwidthTicketProvider`; session spawns no controller)
- [x] 2.3 Add opt-in `with_automatic_topups(RestockPolicy)` mapping onto `BandwidthControllerConfig`, installing the fetcher into the running loop with allowed/managed types scoped to `V1WireguardEntry` (+ `V1WireguardExit` iff two-hop)
- [x] 2.4 Replace `ensure_ticketbooks` internals with `restock_ticketbooks` + `wait_for_ticketbooks` via the sender; remove the issuance `select!` cancellation wrappers (funds safety via fetcher recovery); propagate storage failures as `SessionError::Storage` (fixes review session.rs:174, :189)
- [x] 2.5 Route `register_dvpn` / `obtain_wireguard_credential` spending through the sender
- [x] 2.6 Derive `client_id` from a domain-separated hash of mnemonic entropy, computed before the nyxd client is built (removes `mnemonic.clone()`); wrap owned mnemonic in `Zeroizing` (review session.rs:40, :125; Cargo.toml:37)
- [x] 2.7 Fetch topology once in `register_two_hop_inner`, pass to both `select_inner` calls (review session.rs:339)
- [x] 2.8 Drop the declared entry/exit role restriction for dVPN eligibility in `gateway.rs:87-95` (review gateway.rs:94)
- [x] 2.9 Split `SessionError::Chain(String)` into `#[from] NyxdError` + API-error variant; remove or wire remaining never-constructed variants (review error.rs:11, :28)

## 3. Datapath blockers (nym-smol-dvpn)

- [x] 3.1 Route the metadata client through the tunnel: build it over `TunnelConnector`, thread into `run_topup` and the one-shot helpers (review topup.rs:123 — privacy blocker)
- [x] 3.2 Fix `set_mtu`: `biased` datapath `select!` with swap branch first + swap mutex serializing concurrent calls (review tunnel.rs:288 ×2)
- [x] 3.3 Classify transport recv errors fatal vs transient: fatal exits the datapath observably, transient logs with backoff (review tunnel.rs:430)
- [x] 3.4 IP-literal short-circuit in smol-core `tcp_connect_host`/`resolve` (`host.parse::<IpAddr>()`); strip IPv6 brackets in `TunnelConnector` (review connectors.rs:68)
- [x] 3.5 Prefer IPv4 bridge address for v4-only clients; fix `BridgeParams::addresses` doc contradiction (review bridge.rs:20, :46)
- [x] 3.6 Replace `BandwidthCredentialSource` with an adapter over `Arc<dyn BandwidthTicketProvider>` (+gateway id + ticket type); make gateway-side top-up default-on for session-built tunnels with disable/tune options
- [x] 3.7 Bandwidth events: split monitor from act in `run_topup`; `BandwidthEvent` on `broadcast` + latest-reading `watch`, exposed as `tunnel.bandwidth_events()`; monitor runs whenever a metadata URL is known

## 4. DNS hardening (common/smol-core/src/dns.rs)

- [x] 4.1 Random per-query transaction id; recv loop accepting only id-and-source-matched responses until match or timeout; delete global `TXN_ID` (review dns.rs:30, :96)
- [x] 4.2 Check `response_code()`: distinct errors for SERVFAIL/REFUSED vs NXDOMAIN/empty (review dns.rs:115)
- [x] 4.3 Skip the AAAA query while the stack is v4-only; note `join!` for future dual-stack (review dns.rs:58, :108)
- [x] 4.4 Extend smol-core tests: mismatched-id/source datagram discarded, SERVFAIL surfaced, IP-literal connect, v4-only resolution

## 5. Performance

- [x] 5.1 Datapath allocations pass: engine-owned reusable scratch buffers sized MTU+overhead replacing per-call 64 KiB zeroed vecs in `engine.rs` encap/decap/timer/handshake_init (incl. decap drain loop); right-size and reuse in `transport.rs` recv; reduce copies in `bridge.rs` send/recv and `framing.rs` (review body item B, transport.rs:79)
- [ ] 5.2 Sanity-check throughput with the `zcash-sync` example before/after

## 6. API polish

- [x] 6.1 `#[must_use]` on all `with_*` builders; warning doc on `with_ipv6` (no-op until dual-stack) (review stack.rs:62, :68, :119)
- [x] 6.2 Typed WG keys in `PeerConfig` via `boringtun::x25519` re-exports (review config.rs:22)
- [x] 6.3 `PeerConfig` `Debug` includes all fields with key material `<redacted>` (review config.rs:42)
- [x] 6.4 `cfg(target_os = "ios"/"android")` → `MtuConfig::MOBILE` default (review config.rs:74)
- [x] 6.5 Log swallowed `TunnResult::Err` throughout `engine.rs`; remove or wire `DvpnError::WireGuard` (review engine.rs:49, error.rs:11)
- [x] 6.6 Clarify `DEFAULT_EXIT_WG_CLIENT_PORT` doc (client source port inside the two-hop inner frame, not a node port) (review config.rs:15)
- [x] 6.7 Docs: remove bridges-publication TODO in `docs/design/sdk/smol-dvpn/design.md:67` (nym-bridges on crates.io since 7f3c80ce2); correct design §10 to describe the actual top-up model (gateway-side default-on, chain-side opt-in, events)

## 7. Review-thread replies & re-review

- [x] 7.1 Reply on top-up threads: gateway-side default-on (agrees with reviewer), chain-side opt-in with cost rationale (37.5 GB/ticketbook), ready-made source + events channel
- [x] 7.2 Reply on config.rs:15 (port 54001 semantics) and Cargo.toml:37 (workspace zeroize feature present; drop-zeroization added via `Zeroizing`)
- [x] 7.3 Reply on session.rs:252: single-fetch applied; ask about a role-filtered described-nodes endpoint in nym-api
- [x] 7.4 Resolve mfahampshire's design.md:67 thread (bridges published)
- [x] 7.5 Verify full build + `cargo test -p smol-core -p nym-smol-dvpn -p nym-sdk-session`; run one live example end-to-end; re-request review on #6953

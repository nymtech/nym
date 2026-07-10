# Design: fix-dvpn-review-6953

## Context

PR #6953 adds the `smol-dvpn` stack (`smol-core`, `nym-sdk-session`,
`nym-smol-dvpn`). jstuczyn's CHANGES_REQUESTED review raised 33 inline comments
plus two review-body items; all substantive claims were verified against the
code and confirmed. Independently, the branch was rebased onto the
bandwidth-controller rework (#6898/#6937/#6952): `BandwidthController` is now an
actor — the **single writer** to credential storage — driven by
`run(shutdown_token)`, with proactive restock (spend-triggered, periodic, and on
fetcher install), in-flight fetch dedup, and readiness waiters. Its cloneable
`BandwidthControllerRequestSender` implements `BandwidthTicketProvider`, the
trait `register_dvpn` consumes. `nym-bandwidth-fetcher`'s
`NyxdCredentialFetcher` records deposits in a sqlite pending-requests store, so
an interrupted issuance is recovered, not lost.

Constraints:
- NymVPN consumes `nym-bandwidth-controller`/`nym-bandwidth-fetcher` as
  currently integrated; **no existing call signatures may change** and no enum
  a consumer may match exhaustively (e.g. `FetcherErrorKind`) may grow variants.
- One ticketbook ≈ 37.5 GB and costs the implementer NYM; automatic on-chain
  purchasing must be opt-in (user decision).
- A dVPN-only session must **never** deposit for mixnet ticket types (hard
  requirement): the controller's restock sweep covers all types except
  `V1MixnetExit` (`ticketbooks.rs:173`), and `needs_restock` triggers on any
  type with 0 tickets — so an unscoped fetcher install would buy unneeded
  mixnet ticketbooks.

## Goals / Non-Goals

**Goals:**
- Resolve every actionable review comment on PR #6953 (fix, or reply with
  rationale where we push back).
- Move `nym-sdk-session` onto the running-controller pattern the rest of the
  codebase migrated to in #6940.
- Gateway-side top-up on by default (uninterrupted service, spends only
  already-purchased tickets); chain-side restock opt-in with scoped ticket
  types.
- Give implementers a bandwidth-event surface so they can prompt users to buy
  ticketbooks manually.

**Non-Goals:**
- Dual-stack (IPv6) support in `smol-core` — still deferred; this change only
  stops v4-only paths from producing unroutable v6 results.
- Batched deposits in the fetcher (upstream TODO, `credentials.rs:41`).
- A role-filtered described-nodes endpoint in nym-api (raised in the review
  thread; out of scope here — we apply the single-fetch mitigation).
- Changing NymVPN's integration in any way.

## Decisions

### D1: Ticket-type scoping enforced at the fetcher, scoped at the controller

> **Superseded during review (PR #6976).** Simon Wicky rejected the fetcher-level
> veto ("against the spirit of `CredentialFetcher` — it may fetch any type; the
> controller decides what to request"). The delivered design drops the fetcher veto
> entirely and puts the scoping solely on the controller:
> `BandwidthControllerConfig::managed_ticket_types` (a config field; `Copy` was
> dropped for `Clone`), gating the periodic sweep, the fetcher-install restock, **and**
> the post-spend top-up. An empty managed set disables proactive restock (which is how
> the session runs by default). The `auto_restock` flag was also dropped as redundant.
> The behavioural guarantee (a dVPN session never deposits for mixnet types) is
> preserved because the session sets `managed_ticket_types` to WireGuard-only and only
> ever requests those. See the reconciled `specs/bandwidth-type-scoping/spec.md`. The
> original plan below is kept for the decision trail.

The fetcher is the last gate before money leaves the account
(`make_deposit`), so the **hard guarantee** lives there:
`NyxdCredentialFetcher` gains `allowed_ticketbook_types:
Option<HashSet<TicketType>>` (default `None` = allow all) plus an additive
builder `with_allowed_ticketbook_types(...)`. The guard sits **between**
recovery and deposit in `fetch_ticketbooks`: recovered deposits are already
paid for and are always returned regardless of type; only fresh deposits are
vetoed. A vetoed type returns a new
`NyxdFetcherError::TicketbookTypeNotAllowed { requested, allowed }` rather
than `Ok(vec![])` — an error is surfaced to `wait_for_ticketbooks` waiters
(fail fast), whereas an empty `Ok` logs "fetched and stored" misleadingly and
parks waiters forever. Its `kind()` maps to the existing
`FetcherErrorKind::Other` (no new variant; `FetcherErrorKind` is not
`#[non_exhaustive]` and NymVPN may match it exhaustively).

The controller companion removes noise: `BandwidthController` gains
`with_managed_ticket_types(Vec<TicketType>)` (a builder method on the
controller — **not** a `BandwidthControllerConfig` field, which is `Copy` with
public fields and would break on adding a `Vec`). The periodic sweep
(`controller.rs:158`) and fetcher-install restock (`controller.rs:253`) iterate
the managed set instead of `AvailableTicketbooks::ticketbook_types()`; default
is the current list. Without this, the sweep would retry vetoed types every
3 h forever (the controller keeps no per-type failure memory).

*Alternative considered:* controller scoping only — rejected because any path
that bypasses the sweep (manual `restock_ticketbooks`, future code) could
still deposit; the fetcher veto makes violation impossible.

These land as a **separate PR against `develop`** (D6).

### D2: Session runs the controller; provisioning via the request sender

`Session::new` follows client-core's `start_bandwidth_controller`
(`base_client/mod.rs:799-825`): build the controller, take
`get_request_sender()`, spawn `run(shutdown_token)`, and use the sender
(which implements `BandwidthTicketProvider`) for all ticket spending,
including `register_dvpn` and gateway top-up.

Initial provisioning happens **one-shot on the not-yet-running controller**
(still a supported mode) *before* `run()` is spawned, and the credential
fetcher is only installed into the running loop when the caller opts into
automatic restock. This ordering avoids the install-triggered restock sweep in
the default (no-topups) mode.

An escape hatch mirrors client-core's `custom_bandwidth_provider`: a caller
already running a controller (e.g. an app embedding both a mixnet client and a
dVPN session over one credential store) passes its provider and Session spawns
nothing — preserving the single-writer invariant.

This dissolves two review comments structurally:
- `session.rs:189` (storage error swallowed → double issuance): the
  peek-then-fetch `ensure_one` disappears; readiness is
  `restock_ticketbooks` + `wait_for_ticketbooks`, and storage consistency is
  the controller's job. Where Session still touches storage-adjacent errors,
  they propagate as `SessionError::Storage`.
- `session.rs:174` (cancellation mid-deposit loses funds): funds are protected
  by the fetcher's pending-request recovery, not by making futures
  uncancellable. Session's own `select!` cancellation wrappers around issuance
  are removed; cancellation is checked between phases.

### D3: Two top-up layers, two policies

- **Gateway-side top-up** (spending *already-stored* tickets at the gateway
  `metadata` endpoint to extend a live tunnel's bandwidth): **on by default**
  for session-built tunnels. It costs nothing new and without it a long-lived
  tunnel dies while holding usable tickets. Callers can disable or tune it.
- **Chain-side restock** (depositing NYM for *new* ticketbooks): **opt-in**
  via `Session::with_automatic_topups(RestockPolicy)` mapping onto
  `BandwidthControllerConfig` (min tickets, restock amount, interval,
  soon-expiry) with the managed/allowed type set scoped to the session's WG
  types (`V1WireguardEntry`, plus `V1WireguardExit` iff two-hop).

This partially agrees with the reviewer's "make top-up default" (the free
layer is default-on) while honoring the cost constraint on the purchasing
layer; the rationale is posted on the review thread.

### D4: Top-up traffic must ride the tunnel; monitoring decoupled from acting

`metadata_client` currently builds a host-network HTTP client — either broken
(in-tunnel-only endpoint) or a deanonymizing leak. The metadata client is
constructed over the tunnel's `TunnelConnector` (hyper client with the tunnel
connector), and `run_topup` receives it instead of building its own.

The poll loop splits into *monitor* (query `available_bandwidth`, publish
state) and *act* (spend a ticket when below threshold). Monitoring emits
`BandwidthEvent`s — `Low { available, threshold }`, `ToppedUp { new_available
}`, `TopupFailed { reason }`, `Exhausted` — on a `tokio::sync::broadcast`
channel plus a `watch` holding the latest reading, exposed as
`tunnel.bandwidth_events()`. The monitor runs whenever a metadata URL is
known, independent of whether automatic top-up is enabled, so implementers can
prompt their users to purchase ticketbooks manually.

The credential source for top-up is a thin adapter binding
`Arc<dyn BandwidthTicketProvider>` + gateway id + ticket type;
`nym-bandwidth-controller` is a `common/` crate so `nym-smol-dvpn` may depend
on it without inverting the session↔datapath dependency direction. The
bespoke `BandwidthCredentialSource` trait is retired in favor of this.

### D5: Datapath correctness fixes

- **`set_mtu` race**: datapath `select!` becomes `biased` with the swap branch
  first, and `set_mtu` serializes behind a swap mutex (two concurrent calls
  currently desync stack vs channels). Chosen over ack-based swap for
  simplicity; the biased ordering guarantees the swap is observed before the
  closed-channel `None`.
- **Transport recv errors**: classified fatal vs transient. A closed bridge
  stream (or other fatal error) exits the datapath (tunnel reports down —
  observable via shutdown/JoinHandle and `BandwidthEvent`-adjacent state);
  transient UDP errors log with backoff. Ends the busy-loop.
- **IP-literal connect**: `tcp_connect_host`/`resolve` short-circuit on
  `host.parse::<IpAddr>()`; the connector strips IPv6 brackets from
  `Uri::host()`. Fixes the module's own doc example.
- **Bridge addressing**: prefer the IPv4 bridge address for v4-only clients
  (the directory lists IPv6 first and `nym_bridges` dials `addresses[0]`);
  field docs corrected to match actual dialing behavior.
- **DNS**: one coherent hardening pass — random per-query id validated against
  `response.id()` *and* source == configured server, recv-loop until match or
  timeout, RCODE-aware errors (SERVFAIL/REFUSED ≠ NXDOMAIN ≠ empty), global
  `TXN_ID` deleted, and the AAAA query skipped entirely while the stack is
  v4-only (also mooting the concurrent-A/AAAA suggestion until dual-stack).

### D6: Two PRs

Upstream crate changes (D1) go in a small standalone PR against `develop` so
Simon Wicky reviews changes to his fresh code in isolation and #6953's next
round stays about the review fixes. #6953 rebases on it once merged.

### D7: Assorted API decisions

- Typed WG keys in `PeerConfig` via `boringtun::x25519` re-exports
  (`StaticSecret`/`PublicKey`) — type safety with zero new dependencies,
  honoring the original decoupling intent.
- `client_id` derived from a **hash** of the mnemonic entropy (domain-separated)
  instead of raw entropy, generated before the nyxd client is built (removes
  the `mnemonic.clone()`); owned mnemonics wrapped in `Zeroizing` (bip39's
  `zeroize` feature adds the impl but not drop-zeroization). Safe to change
  derivation: the crate is new in this branch, no deployed state exists.
- `SessionError::Chain(String)` split into `Nyxd(#[from] NyxdError)` and an
  API-error variant (the two call sites carry different error types).
- Per-target MTU default: `cfg(target_os = "ios"/"android")` →
  `MtuConfig::MOBILE`.
- Datapath allocations: engine owns reusable scratch buffers sized to
  MTU+overhead (single-task, no locking needed); transport/bridge/framing
  reuse right-sized buffers instead of per-packet 64 KiB zeroed vecs.

## Risks / Trade-offs

- [Fetcher veto returns errors the controller retries every sweep] → paired
  controller scoping (D1) means managed types and allowed types coincide in
  practice; the veto is defense-in-depth, not the primary path.
- [Session spawns a background task; shutdown ordering matters] → adopt the
  token type `run()` expects and await the controller's cleanup path
  (`cancel_and_join` → `fetcher.cleanup` → `storage.close`) in
  `Session` shutdown rather than detaching; verify `ShutdownToken` vs
  `CancellationToken` compatibility at implementation time.
- [Gateway top-up default-on spends stored tickets without an explicit call] →
  they were purchased for exactly this; events channel makes every spend
  observable; callers can disable.
- [Biased select prioritizes swaps over data] → swaps are rare (manual MTU
  changes); negligible.
- [Buffer reuse in the datapath risks aliasing bugs] → the engine is
  single-task by design (documented in `engine.rs`); keep the pass mechanical
  and covered by the existing smol-core integration tests + examples.
- [Skipping AAAA changes resolver behavior for v6-only names] → such addresses
  were unroutable on the v4-only stack anyway (previously a confusing connect
  failure; now a clear no-records error).

## Migration Plan

1. Upstream PR (`nym-bandwidth-fetcher` + `nym-bandwidth-controller`,
   additive, defaults preserve behavior) → review by Simon Wicky → merge to
   `develop`.
2. Rebase `feature/nym-sdk-dvpn` on `develop`.
3. Land the tiers on the feature branch (session architecture first, then
   datapath blockers, DNS, performance, polish — each independently
   revertible).
4. Post review-thread replies (pushback rationale + questions) and re-request
   review on #6953.

Rollback: upstream changes are dormant unless the new builders are called;
reverting the feature-branch commits restores current behavior.

## Open Questions

- Exact shape of `RestockPolicy` field names vs `BandwidthControllerConfig`
  (`nb_ticket_restock` doubles as the trigger threshold; `min_nb_ticket_needed`
  feeds only the readiness check — confirmed intended). Map 1:1 and document.
- Whether the datapath's fatal-exit should also emit a terminal event on the
  bandwidth/events channel or a dedicated tunnel-state channel (lean: dedicated
  `watch<TunnelState>`; decide during implementation of D5).

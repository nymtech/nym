## 1. Remove the fetcher install/uninstall lifecycle

- [x] 1.1 In `sdk/rust/nym-sdk-session/src/session.rs`, delete `FetcherBuilder`,
  `NyxdFetcherBuilder`, and `OwnedController` (with `installed`, `auto_topup`,
  `ensure`, `provision`, `set_fetcher`, `set_fetcher_if_absent`, `remove_fetcher`,
  `wait_for`).
- [x] 1.2 Replace `Session::owned: Option<OwnedController<..>>` with a mode enum
  (design D1): `Running { sender, task }` and `OneShot { nyxd, store_path, fetcher_db,
  client_id, config, lock }`; keep `provider: Arc<dyn BandwidthTicketProvider>`.

## 2. Wire the controller per mode in `Session::new`

- [x] 2.1 Extract the shared setup from `spawn_controller`: nyxd URL, `derive_client_id`,
  `DirectSigningHttpRpcNyxdClient` (consumes the mnemonic), store path (default
  `data_path/credentials.db`, parent dir created), `fetcher-requests.db` path, and the
  scoped `BandwidthControllerConfig` (`managed_ticket_types = needed_ticket_types(two_hop)`,
  thresholds from `RestockPolicy` when set, defaults otherwise).
- [x] 2.2 Running mode (`automatic_topups: Some`): open the store with
  `PersistentStorage::init` (map errors to `SessionError::Storage`), build
  `TimeoutFetcher::new(NyxdCredentialFetcher::new(..))`, construct
  `BandwidthController::new(storage).with_config(cfg).with_credential_fetcher(fetcher)`,
  spawn `run(ShutdownToken::new_from_tokio_token(cancel))`, provider =
  `Arc::new(sender.clone())`.
- [x] 2.3 One-shot mode (`automatic_topups: None`): open the store, construct the
  non-running provider `BandwidthController::new(storage)
  .with_credential_public_data_fetcher(NyxdGlobalDataFetcher::new(nyxd.clone()))` as
  `Arc<dyn BandwidthTicketProvider>`; store the one-shot provisioning inputs in the mode
  enum. Do not spawn anything.
- [x] 2.4 `Session::shutdown`: Running → cancel + await task (as today); OneShot →
  `provider.close().await`. Keep the best-effort cancel in `Drop`.

## 3. One-shot provisioning

- [x] 3.1 Add `pub(crate) async fn provision_once<St: Storage, F: CredentialFetcher +
  'static>(storage: St, fetcher: F, config: BandwidthControllerConfig, types:
  Vec<TicketType>) -> Result<(), SessionError>` per design D3: build the non-running
  controller; read stock via `AvailableTicketbooks::try_from(storage.get_ticketbooks_info())`;
  `fetch_ticketbook` for each `typ` with `needs_restock(typ, &config)`, stopping at the
  first error; on success re-read stock and require `contains_minimal_tickets` for every
  requested type; ALWAYS tear down by dropping the controller and calling `cleanup` on a
  retained clone of the fetcher (`TimeoutFetcher` made a cloneable `Arc` handle, with
  `reset` failing as `FetcherStillShared` while shared). The first cut used a second store
  handle torn down via `run(<cancelled token>)`; live runs showed the pool guard waiting
  2 s with a warning on every call while the session's handle stayed open, so the
  provisioning controller now shares the session's store handle.
- [x] 3.2 In `ensure_ticket_types` OneShot arm: race `lock.lock()` against the cancel
  token (existing pattern); then `tokio::spawn` a task that takes a clone of the
  session's store handle, builds a fresh
  `TimeoutFetcher<NyxdCredentialFetcher>`, and calls `provision_once`; race the
  `JoinHandle` against the cancel token and `PROVISIONING_TIMEOUT` (never abort the task);
  map `JoinError` to `SessionError::Issuance`.
- [x] 3.3 Document on `ensure_ticketbooks` that in one-shot mode a cancelled or timed-out
  call leaves the issuance task running to completion so a deposit is never dropped
  mid-flight, and that the task closes its own handles.

## 4. Running-mode provisioning

- [x] 4.1 In `ensure_ticket_types` Running arm: `sender.wait_for_ticketbooks(types)`; on
  `BandwidthControllerError::TicketbooksUnavailable` send `restock_ticketbooks(types)`
  once and wait again; map remaining errors to `SessionError::Issuance`; keep the
  cancel race and `PROVISIONING_TIMEOUT` around the whole arm (safe: deposits run in
  the controller task).
- [x] 4.2 Comment why the restock request is used only as the `Unavailable` fallback
  (the controller documents it as a manual safety valve; the startup sweep covers the
  normal case).

## 5. Docs

- [x] 5.1 Rewrite the `session.rs` module docs and the `lib.rs` crate docs to describe
  the two modes (running-with-fetcher vs non-running inline issuance) and remove
  references to installing/uninstalling the fetcher and to the fetcher builder.
- [x] 5.2 Update `SessionConfig::automatic_topups` / `bandwidth_provider` / `two_hop`
  doc comments in `config.rs` where they mention the fetcher lifecycle or "spawns its
  own controller" unconditionally.

## 6. Tests (`sdk/rust/nym-sdk-session/src/session_tests.rs`)

- [x] 6.1 Delete the five `OwnedController` tests and `spawn_owned`; keep and adapt the
  `RecordingFetcher` double (counts fetches and cleanups; fails after cleanup) and add a
  variant that stores a ticketbook into the ephemeral store on fetch, or seed the store
  directly, so stock can be made sufficient / insufficient per type.
- [x] 6.2 `provision_once` skips a type whose stock does not need restock: no
  `fetch_ticketbooks` call for it.
- [x] 6.3 `provision_once` fetches exactly once per low type and the fetcher's
  `cleanup` count is exactly 1 afterwards (teardown ran), for a successful run.
- [x] 6.4 `provision_once` with a failing fetcher returns `Issuance` and still cleans the
  fetcher up exactly once.
- [x] 6.5 `provision_once` returns an error when a requested type is still below the
  minimum after the fetch (fetcher returns an empty batch).
- [x] 6.6 Retain the "queued caller behind the held one-shot lock is cancellable" test
  against the new mode enum.
- [x] 6.7 Running mode: with a real controller over ephemeral storage and a
  `RecordingFetcher` installed at construction whose first fetch fails, the Running arm's
  `Unavailable` fallback sends exactly one restock and waits again (assert two fetch
  calls, or one restock request observed).

## 7. Verification

- [x] 7.1 `cargo test -p nym-sdk-session` (unit + `tests/signer_fault_http.rs`) and
  `cargo test -p nym-bandwidth-controller` pass; `git diff --stat -- common/` is empty.
- [x] 7.2 `cargo build -p nym-smoldvpn --examples --tests` compiles unchanged call sites.
- [x] 7.3 `cargo fmt` and `cargo clippy --all-targets` clean on `nym-sdk-session` and
  `nym-smoldvpn`.
- [x] 7.4 Live sandbox check (2026-09-04, `envs/sandbox.env`, funded sandbox mnemonic, healthy
  pair `6imWS…` entry / `6qidVK…` exit per `docs/smoldvpn/two-hop-exit-handshake-findings.md`):
  - `two-hop-ip` (stocked store): both types skipped ("sufficiently stocked; no issuance
    needed"), cached registrations reused, both hops up, IP relocated — PASS, ~1 s.
  - `smoldvpn-config` (fresh store): one-shot issued exactly one entry ticketbook (1 NYM
    deposit, wallet aggregated, stored), the register path's second `ensure` skipped, WireGuard
    config exported — PASS.
  - `zcash-sync --blocks 500` (low store): issued entry + exit, 500 blocks synced through the
    tunnel — PASS.
  - `two-hop-ip --one-hop --gateway 6qidVK`: stale cached registration detected, invalidated and
    re-registered by the example's retry, then PASS.
  - `live_bringup` gated tests (`SMOLDVPN_ENTRY`/`SMOLDVPN_EXIT` pinned): single-hop PASS;
    two-hop failed once on a stale cached registration (the test has no invalidate-and-retry,
    unlike the examples), PASS after clearing `smoldvpn/live-two/registrations.json`.
  - `quic-probe` against the sandbox QUIC bridge (`194.182.161.94:4443`): raw and pinned OK.
    (Its built-in defaults point at non-sandbox hosts and time out.)
  - `two-hop-quic`: provisioning and registration succeed, but every QUIC-capable sandbox entry
    (`Gejc2`, `Bzcq`, `33g9y`) is a known non-forwarding entry, so the exit hop never
    establishes; the bounded retry fails after 3 attempts as documented in the findings.
    Environment, not this change.
  - `smoldvpn-grpc --gateway 6qidVK --target grpcb.in:9000`: provisioning, registration and the
    WireGuard session succeed; the plaintext gRPC connect through the tunnel then hangs (the
    example has no connect timeout). `grpcb.in` is IPv4-only, so this is egress to a
    non-standard port through the gateway, not provisioning. No plaintext gRPC health target
    on a standard port was available.
  - `smoldvpn-topup --metadata-url http://85.217.184.106:51830/`: stock check skipped issuance
    as expected; the host-network metadata query times out because the sandbox gateways serve
    the metadata endpoint in-tunnel only (the announced port is not reachable from the host).
  - First cut of this change surfaced a real defect during these runs (2 s pool-guard wait plus
    a warning per `ensure_ticketbooks`); fixed by sharing the store handle (task 3.1).
- [x] 7.5 `openspec validate rework-dvpn-session-controller-modes --strict` passes.

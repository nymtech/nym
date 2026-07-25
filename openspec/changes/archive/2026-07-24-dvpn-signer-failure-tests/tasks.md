# Tasks: dvpn-signer-failure-tests

## 1. TimeoutFetcher decorator (Layer 1)

- [x] 1.1 Add `async-trait` to `nym-sdk-session` dependencies (workspace dep;
      needed to implement the fetcher traits on the decorator)
- [x] 1.2 Implement `TimeoutFetcher<F>` in `sdk/rust/nym-sdk-session`
      (new `fetcher.rs` module or within `session.rs`): implements
      `CredentialPublicDataFetcher` by wrapping the three public-data fetches
      in `tokio::time::timeout` (15s per call), and `CredentialFetcher` by
      plain delegation — `fetch_ticketbooks`, `cleanup`, `reset` are NOT timed
- [x] 1.3 Define a `SignerTimeout` error implementing
      `nym_bandwidth_controller::FetcherError` so an elapsed timeout surfaces
      through the controller's `FetchFailed` readiness reporting with a
      message naming unresponsive ecash signers
- [x] 1.4 Wire the decorator into `Session::spawn_controller`
      (`session.rs:324`): wrap the `NyxdCredentialFetcher` in `TimeoutFetcher`
      before `with_credential_fetcher`

## 2. Outer provisioning timeout (Layer 2)

- [x] 2.1 Add `SessionError::ProvisioningTimeout` variant with a message
      identifying unresponsive ecash signers as the likely cause and noting
      that any deposit made is recoverable from the pending-request store;
      check whether `SessionError` is `#[non_exhaustive]` and note
      source-compat in the changelog if not
- [x] 2.2 Add a `tokio::time::timeout` bound (initial budget: 5 minutes)
      around the work arm of the existing `tokio::select!` in
      `Session::ensure_ticketbooks` (`session.rs:373`), mapping elapse to
      `ProvisioningTimeout`; preserve the existing cancellation race and its
      funds-safety comment

## 3. Tier 1 — decorator unit tests (virtual clock)

- [x] 3.1 Add unit tests for `TimeoutFetcher` using
      `#[tokio::test(start_paused = true)]` + `tokio::time::advance`, with a
      configurable inner stub fetcher
- [x] 3.2 Test: hanging inner `fetch_expiration_date_signatures`
      (`std::future::pending()`) returns `Err(SignerTimeout)` once the clock
      advances past the per-call bound (same for vk and coin-index fetches)
- [x] 3.3 Test: slow-but-under-threshold inner fetch succeeds; just-over-
      threshold fails — boundary behavior around the 15s bound
- [x] 3.4 Test: inner fetch returning `Err` immediately passes the error
      through unaltered (no timeout wrapping of genuine errors)
- [x] 3.5 Test: a hanging `fetch_ticketbooks` is NOT timed — advance the
      clock well past the per-call bound and assert the future is still
      pending

## 4. Tier 2 — end-to-end recovery tests (real controller, ephemeral store)

- [x] 4.1 Add dev-dependencies to `nym-sdk-session`:
      `nym-credential-storage` (ephemeral store) and the compact-ecash /
      `nym-credentials` crates needed for the ticketbook fixture
- [x] 4.2 Write a shared test-support helper that fabricates a valid
      threshold-signed `IssuedTicketBook` using the compact-ecash primitives,
      mirroring `common/credentials/src/ecash/bandwidth/issuance.rs`
      (`generate_keypair_user_from_seed`, `withdrawal_request`,
      `issue_verify`, `aggregate_wallets`); keep construction isolated in one
      function
- [x] 4.3 Write `FlakyFetcher` implementing both fetcher traits with a mode
      enum — `Hang`, `Slow(Duration)`, `Error`, `Partial` (vk + coin-index
      Ok, expiration-date signatures fail) — and an atomic call counter on
      `fetch_ticketbooks`; model on
      `common/bandwidth-controller/tests/managed_ticket_types.rs`
- [x] 4.4 Bug-reproduction test (no decorator): controller + ephemeral
      storage + `FlakyFetcher::Hang` → assert `wait_for_ticketbooks` does not
      resolve within a short real deadline AND storage holds zero ticketbooks
      (reproduces the observed mainnet `creds.db` state)
- [x] 4.5 Recovery test (with decorator): modes `Hang` and `Partial` → assert
      the fabricated ticketbook IS persisted to storage and
      `wait_for_ticketbooks` resolves `Ok`
- [x] 4.6 Money-safety test: provision twice against the same storage with
      the decorator; assert the second run makes zero additional
      `fetch_ticketbooks` calls (no re-deposit for a stocked type)
- [x] 4.7 Degraded-spend test: with a persisted ticketbook but expiration
      signatures still unavailable (`Partial` mode), assert a spend attempt
      (`get_ecash_ticket` via the provider) returns a bounded error rather
      than hanging

## 5. Verification & documentation

- [x] 5.1 Run `cargo test -p nym-sdk-session` and
      `cargo clippy -p nym-sdk-session --tests` — all green, no warnings
- [x] 5.2 Confirm no `common/` production code changed
      (`git diff --stat` touches only `sdk/rust/nym-sdk-session` and
      openspec files)
- [x] 5.3 Document the behavior in `nym-sdk-session` rustdoc: per-call
      timeout on public-data fetches, ticketbook-persisted-despite-outage
      semantics, the new `ProvisioningTimeout` error, and the degraded
      (unspendable-until-signers-return) window
- [x] 5.4 Optional live validation on mainnet (documented, manual): one
      provisioning run must complete without hanging and persist the
      ticketbook to `data/<example>/mainnet/creds.db` — validated 2026-07-23:
      `zcash-sync` provisioned in ~1m47s (was: infinite hang), both
      SignerTimeout warnings fired at 15s, and both wireguard ticketbooks
      (entry + exit, 50 tickets each) persisted to creds.db

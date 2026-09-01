## 1. Controller wiring in the session

- [ ] 1.1 In `spawn_controller` (`sdk/rust/nym-sdk-session/src/session.rs`), set
  `managed_ticket_types = wireguard_ticket_types()` in both the `Some(policy)` and
  `None` arms (drop the empty-vec default arm).
- [ ] 1.2 Stop installing the credential fetcher at construction via the builder;
  keep the constructed `fetcher` accessible for on-demand install (e.g. store it on
  `OwnedController`, alongside `sender`/`task`), and record whether automatic top-ups
  are enabled.
- [ ] 1.3 Track fetcher install state with a local flag on `OwnedController` (the
  session is the single writer, so no controller-side query is needed); initialise it
  to "not installed" (nothing is installed at construction).

## 2. Fetcher-lifecycle provisioning

- [ ] 2.1 Rewrite `ensure_ticket_types` to early-return on empty `types`, then branch
  by mode, using the running controller's request sender (never the owned inline
  `fetch_ticketbook`), all inside the existing `PROVISIONING_TIMEOUT` + cancellation
  `select!`, mapping errors to `SessionError::Issuance` / `ProvisioningTimeout`.
- [ ] 2.2 Default / one-shot mode (`automatic_topups: None`):
  `set_credential_fetcher(fetcher)` → `wait_for_ticketbooks(types)` →
  `unset_credential_fetcher()` **unconditionally, on every exit path (success, error,
  timeout, cancellation)** — capture the wait outcome, always run the best-effort
  unset, then propagate the outcome. The fetcher must never be left installed after a
  failed provision.
- [ ] 2.3 Automatic-top-up mode (`Some(policy)`): install the fetcher only if the local
  flag says it is not already present (`set_credential_fetcher`, set the flag),
  `wait_for_ticketbooks(types)`, leave it installed; a later call skips re-install
  while the flag is set.
- [ ] 2.4 Remove the now-unused `restock_ticketbooks` call from the provisioning path;
  keep `wait_for_ticketbooks` as the readiness gate.
- [ ] 2.5 Retry handling: in auto-top-up mode, re-set the fetcher (unset then set) to
  re-trigger a fetch when a required type is not ready after a failure, instead of
  skipping the re-install; skip only when the required types are already ready. Default
  mode already re-triggers on the next call via the always-unset-on-failure rule.
- [ ] 2.6 Consider making transient fetch failures retryable in the fetcher layer
  (`TimeoutFetcher`/`NyxdCredentialFetcher`) as the first line of resilience — evaluate
  scope; may be a separate change since it is outside the session's controller usage.

## 3. Comments and docs

- [ ] 3.1 Update the stale comment at `session.rs:~244` ("the default leaves it
  empty, so the session provisions on demand") to describe the fetcher lifecycle.
- [ ] 3.2 Update the stale comment at `session.rs:~291` ("Explicit restock request
  (works regardless of the auto-restock setting)") to reflect the new mechanism.

## 4. Tests

- [ ] 4.1 Add a deterministic `nym-sdk-session` test (ephemeral store, no network /
  funds where possible, following the signer-fault-harness style) proving a
  default-mode session's `ensure_ticketbooks` provisions the WireGuard types and that
  no background deposit is made after provisioning (fetcher uninstalled).
- [ ] 4.2 Add/adjust a test proving auto-top-up mode leaves the fetcher installed and
  restocks per policy (and that a second provisioning call skips re-install).
- [ ] 4.3 Add a test proving default mode removes the fetcher even when provisioning
  fails — force a failing/erroring fetch and assert the fetcher is uninstalled
  afterward (no lingering install → no background deposit).
- [ ] 4.4 Run the smoldvpn gated integration tests against sandbox and confirm both
  pass: `set -a; source envs/sandbox.env; source .claude/.secrets/sandbox.env; set +a;
  MNEMONIC="$NYX_ACCOUNT_MNEMONIC" cargo test -p nym-smoldvpn --test live_bringup --
  --ignored --test-threads=1`.

## 5. Verification

- [ ] 5.1 `cargo test -p nym-sdk-session` and `cargo test -p nym-bandwidth-controller`
  pass (the controller is unchanged; its `managed_ticket_types` suite must still pass).
- [ ] 5.2 Run `cargo fmt` and `cargo clippy` on every touched target.
- [ ] 5.3 `openspec validate fix-dvpn-session-fetcher-restock --strict` passes.

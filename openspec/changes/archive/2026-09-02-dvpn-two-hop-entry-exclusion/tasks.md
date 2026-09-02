## 1. Generalize gateway selection exclusion (`gateway.rs`)

- [x] 1.1 Change `select`'s `exclude` parameter from `Option<&ed25519::PublicKey>` to `&[ed25519::PublicKey]`, and change the `excluded` predicate to `exclude.contains(id)`
- [x] 1.2 Update the doc comment on `select` to describe the exclusion *set* (and that a pinned identity in the set is never substituted)
- [x] 1.3 Update in-crate call sites: entry selection passes `&[]` (or the avoid set); exit selection passes `std::slice::from_ref(&entry_gw.identity)` instead of `Some(&…)`; single-hop passes `&[]`
- [x] 1.4 Update existing selection unit tests to pass slices (`None` → `&[]`, `Some(&id)` → `&[id]`)
- [x] 1.5 Add a test helper building a minimal WireGuard-capable `NymNodeDescriptionV2` keyed by a chosen identity (follow the `nym-gateway-probe` construction pattern; fall back to extracting the eligibility predicate if construction is too heavy)
- [x] 1.6 Add a unit test asserting a `Random` selection over `{a, b}` excluding `[a]` always returns `b`, and that excluding both yields `NoWireguardGateway`

## 2. Entry-avoidance two-hop registration (`session.rs`)

- [x] 2.1 Add an `avoid_entries: &[ed25519::PublicKey]` parameter to `register_two_hop_inner`, applied to the entry `select` call
- [x] 2.2 Add public `Session::register_two_hop_avoiding_entries(entry, exit, avoid_entries)` that calls the inner path
- [x] 2.3 Keep `register_two_hop` / `register_two_hop_quic` signatures unchanged, delegating with an empty avoid set
- [x] 2.4 Confirm `register_single_inner` still compiles with the `&[]` exclusion (no behavior change)

## 3. Retry policy: exclude implicated substitutable entries (`smoldvpn/examples/common/mod.rs`)

- [x] 3.1 In `connect`, track a `failed_entries: Vec<ed25519::PublicKey>` across attempts
- [x] 3.2 Compute `entry_substitutable = matches!(cli.entry, GatewaySpec::Random | GatewaySpec::Country(_))`
- [x] 3.3 On escalation (`prev_exit_down && status.entry`): for a substitutable entry, invalidate and push `reg.entry.gateway_identity` into `failed_entries`; for a pinned entry, do neither (retry the same pin)
- [x] 3.4 Route registration through `register_two_hop_avoiding_entries(&cli.entry, &cli.exit, &failed_entries)` for the two-hop non-QUIC path; leave single-hop/QUIC paths on their existing registration calls
- [x] 3.5 Ensure the bounded-attempts error and per-hop status message are unchanged

## 4. Verification

- [x] 4.1 `cargo build -p nym-sdk-session --lib` and `cargo build -p nym-smoldvpn --examples`
- [x] 4.2 `cargo test -p nym-sdk-session --lib` (selection + exclusion unit tests pass)
- [x] 4.3 `cargo fmt` and `cargo clippy -p nym-sdk-session -p nym-smoldvpn --examples --tests` clean
- [x] 4.4 Live re-test against the sandbox: random selection recovers off a non-forwarding entry; a pinned bad entry retries then fails without switching; a healthy pinned pair still passes

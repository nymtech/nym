## 1. Free-tier credential: wire + gateway seam (v1)

- [x] 1.1 Add `BandwidthCredential::FreeTier { token: String }` to `common/credentials-interface`; update every match site (only 3 exhaustive matches needed arms: gateway `handle_final_credential_claim`, `wireguard-private-metadata` server topup + shared v2 downgrade; `authenticator-requests`/`registration` use `.into_zk_nym()` wildcard, no change). Wire-format round-trip test added.
- [x] 1.2 Add the `FreeTier` arm to `handle_final_credential_claim` (`gateway/src/node/wireguard/new_peer_registration/mod.rs`): offline ed25519 JWT verification via new `nym-free-tier-check` crate against the reused (upgrade-mode) attester key, skip ecash verification, seed the free allowance (`seed_free_tier_bandwidth`, mirrors the testnet free-bandwidth path). Gated on `free_tier_enabled`.
- [x] 1.3 Keep `BandwidthClaim.kind` set to an existing wireguard `TicketType`; confirmed `insert_wireguard_peer` persists an ordinary wireguard `ClientType` (kind flows through `process_new_peer` unchanged) and no new `TicketType` variant was added, so nym-api / issuance is untouched
- [x] 1.4 Added the free allowance constant (`FREE_TIER_BANDWIDTH_ALLOWANCE_BYTES`, 100 MB placeholder) to `common/network-defaults`. Attester key REUSED from `upgrade_mode.attester_public_key` (same-signer decision) rather than a separate key/url. Added a nym-node `free_tier` config section (`enabled` + `debug.pool_bandwidth_per_second`, human-readable via bytesize, e.g. "10 MB").
- [x] 1.5 Confirmed both transports (LP `on_final_lp_request` and legacy `on_final_authenticator_request`) funnel through the shared `process_new_peer` -> `handle_final_credential_claim`, so the `FreeTier` arm covers both
- [x] 1.6 Added a required `purpose` claim (`FreeTierPurpose::{NewUser,Renewal}`) to `FreeTierClaims`; the gateway arm rejects `Renewal` tokens until the walled garden exists (forward-compat for the renewal-to-garden flow, task 5.7). Dropped the now-redundant `tier` marker (a required, typed `purpose` already disambiguates from the co-signed upgrade-mode JWT)

## 2. Client-side free-tier credential (`common/bandwidth-controller`) (v1)

- [x] 2.1 Added `NymCredential::FreeTrialToken { jwt, expiration }` + `store_free_trial_token` wired into `store_fetched`. Storage is a DEDICATED `free_trial_token` table (not the emergency-credential family - that's a network-fallback concept; free tier is promotional access): new migration + `StoredFreeTrialToken` model + `Storage` trait methods (`store`/`get`/`clear`) on both `EphemeralStorage` + `PersistentStorage`; single-row replace; `MalformedFreeTrialToken` dropped (token stored as `TEXT`)
- [x] 2.2 Added `get_free_trial_token() -> Option<String>` to `BandwidthTicketProvider` + `BandwidthControllerRequest`/dispatch/sender/`Box`-impl/mock/controller plumbing. Expiry-awareness lives in the storage layer: BOTH backends filter out expired tokens on read (sqlite via SQL, memory via a `now` check), fixing the memory-backend gap that upgrade-mode still has. Storage round-trip/replace/expiry test green
- [x] 2.3 Explicit `free_tier: bool` threaded through the registration flow, parallel to `upgrade_mode_enabled` (gateway-sourced) but caller-sourced. Present `FreeTier` only when set; ERROR (`NoFreeTierToken`) if set but no valid token (no ecash fallback); the token is never auto-presented (paid-reconnect guard is structural - `get_free_trial_token` is only called inside `if free_tier`). Done for: LP core (`register_dvpn`/`finalise_dvpn_registration`), the nested/exit session, `nym-sdk-session` (`register_single_hop`/`register_two_hop`/`register_two_hop_quic`), and the legacy authenticator path (`produce_bandwidth_claim`/`register_wireguard`). Deferred (paid-only + TODO): the older `LpBasedRegistrationClient` and mixnet `MixnetBasedRegistrationClient` higher-level flows. Token FETCH/injection stays out of scope (external vpn-client via the `CredentialFetcher`/`store_fetched` seam)
- [x] 2.4 `PreparedCredential` untouched. Storage tests done (store + retrieve-fresh `Some` + retrieve-expired `None` + single-row replace). Claim-production coverage (`free_tier`+token->`FreeTier`; +none->error; paid path unaffected) is deferred to the mock lifecycle harness (task 7.2): the gating lives in heavyweight client methods (`produce_bandwidth_claim` needs a full authenticator client with no test harness; LP `finalise_dvpn_registration` needs a live transport session), so isolated unit tests are low-value vs the branch being trivial + compile-verified

## 3. Free-tier state and metering (v1)

- [ ] 3.1 Add a per-public-key free-tier record to `nym-gateway-storage`: `{ last_claimed_at: OffsetDateTime, session_start, is_free }` - store the last claim as an absolute timestamp (NOT a `claimed_today` bool), so the guard reads elapsed time and nothing ever needs a scheduled daily reset. (Consider whether `last_claimed_at` subsumes `session_start`.)
- [ ] 3.2 Enforce the rolling single-claim guard at registration: reject a fresh allowance when `now - last_claimed_at < claim_window` (network constant, e.g. 24h); otherwise grant and update `last_claimed_at = now`
- [ ] 3.3 Seed the volume allowance from the network-defaults constant (reuse the existing byte accounting path)
- [ ] 3.4 Add the session-time clock; check elapsed time at the existing bandwidth-flush cadence (coarse; no sub-second precision)
- [ ] 3.5 Trigger the exhaustion transition on whichever limit (bytes or time) is reached first
- [ ] 3.6 Entry-gateway per-IP Sybil filter: count free-tier tokens per client source IP per day and reject new-user tokens over a configurable cap (e.g. 5/day). LP/dVPN transport only (needs the client source IP - verify it is plumbed into the free-tier registration path); exempt `Renewal` tokens. Source IP may be v4 or v6 - count per-`/64` prefix for v6 (a single `/64` defeats per-exact-address limiting)

## 4. Rate limiting via `tc` (v1)

- [ ] 4.1 Add a traffic-control manager (nym-node) that shells out to `tc`; one-time HTB root + shared free pool on `nymwg` (egress) and the ingress path (police / IFB) for bidirectional shaping
- [ ] 4.2 Add/remove a peer to/from the pool via a per-peer classify filter keyed on the peer IP - DUAL-STACK: match both the peer's v4 and v6 tunnel address; the off-switch removes both
- [ ] 4.3 Implement the off-switch: removing a peer's filter drops it to the default unlimited class without disconnecting (reused by garden and upgrade)
- [ ] 4.4 Node config for the pool size; rebuild pool membership from state on startup

## 5. Walled garden via `iptables` (v1)

- [ ] 5.1 Extend `scripts/nym-node-setup/network-tunnel-manager.sh` to pre-create an empty `NYM-GARDEN` chain and its jump scaffolding next to `NYM-EXIT` - in BOTH `iptables` and `ip6tables`
- [ ] 5.2 Add a garden manager (nym-node) that inserts/deletes `-s <peerIP> -j NYM-GARDEN` in its own chain only, never touching operator rules - DUAL-STACK: rule for both the peer's v4 and v6 tunnel address (iptables + ip6tables); allowlist covers the endpoint's v4 and v6 addresses
- [ ] 5.3 Node config for the purchase-endpoint allowlist; populate the garden chain's allow/deny logic
- [ ] 5.4 On exhaustion: leave the tc pool (full speed) and add the peer's garden rule instead of removing the peer
- [ ] 5.5 Reconcile the garden chain from free-tier state on startup; do not persist the node's rules; verify fail-closed behavior on crash
- [ ] 5.6 Reconnect-to-upgrade: when a formerly-free peer presents an ecash credential, clear the garden rule, clear the rate limit, and set `is_free = false`
- [ ] 5.7 Renewal tokens (`purpose = Renewal`): on registration route the peer straight into the walled garden - no bandwidth seeded, no per-IP limiting - replacing the v1 reject-renewal guard added in task 1.6
- [ ] 5.8 Make the LP initial-request handler free-tier-aware: `check_existing_lp_peer`/`lp_peer_to_final_response` currently returns `CompletedRegistration` for ANY existing peer, so an exhausted/garden free peer is wrongly told it is fully connected. Return the config with a restricted/purchase-only status marker (mirror the existing `upgrade_mode` flag on `success_dvpn`), NOT a plain completed registration and NOT `RequiresCredential` (the garden peer has a working restricted tunnel it needs for checkout)

## 6. Metrics (v1)

- [ ] 6.1 Expose a gauge for the number of active free-tier users (pool members, excluding garden)
- [ ] 6.2 Expose the configured free-tier pool allowance (mb/s)
- [ ] 6.3 Wire both into the existing `NymNodeMetrics` / prometheus exposition

## 7. Test harness (v1, kept lean)

- [x] 7.1 Layer 0 DONE: new lean crate `common/free-tier-enforcement` (the future home for the tasks-4/5 managers) with `tests/datapath.rs` - a netns integration test that builds a `node`-forwards-`client` topology (node in its own ns so teardown = delete namespaces), applies the `tc` HTB pool + the `NYM-GARDEN` `iptables` allowlist, and asserts: baseline reaches both endpoints, garden reaches ONLY the allowlisted (purchase) endpoint (other dropped), tc pool present + coexists. Gated by `#[ignore]` AND the `NYM_FREE_TIER_NETNS_TESTS` env var (CI runs `--ignored`, so the env var is the real guard) + a root check. Runner: `netns/{Dockerfile,run.sh,README.md}` (privileged Docker, Apple-`container` fallback). Verified green in a privileged container; self-skips in a CI-style `--ignored` run
- [ ] 7.2 Layer 1: exercise the peer-controller lifecycle (register -> free -> garden -> cleared) via the existing `mock` feature (`MockEcashManager`) against a real kernel interface. Extend the netns harness to cover IPv6 too (`ip6tables` garden + v6 tc classifier), so the dual-stack enforcement (tasks 4/5) is actually validated, not just the v4 path the 7.1 smoke tests cover
- [ ] 7.3 Integration: run a single real gateway in a container pointed at mainnet but never bonded; drive `LpRegistrationClient` + `smol-dvpn` `PeerConfig` directly with the node's IP + keys (bypass topology selection); mint test free-tier JWTs signed with a throwaway attester key
- [ ] 7.4 Keep the harness minimal; document how to run it (container runtime: Apple `container` on macOS, privileged netns on Linux CI)

## 8. Deferred to v2

- [ ] 8.1 Seamless in-session upgrade: buy without reconnecting (top up + clear rate limit + clear `is_free` on the live session via the `TopUp` path)
- [ ] 8.2 Abuse-protection refinements (coordinated with the external VPN-API issuance limits)
- [ ] 8.3 Per-region / per-campaign generosity (may move the allowance into signed token claims)

## 9. Docs, config, finalization (v1)

- [ ] 9.1 Operator docs: the `NYM-GARDEN` scaffolding, enabling the free tier, and the config knobs (pool size, attester key/url, purchase allowlist)
- [ ] 9.2 Confirm the full config surface and defaults; free tier is off unless configured
- [ ] 9.3 `cargo build` / `cargo test` clean across the touched crates

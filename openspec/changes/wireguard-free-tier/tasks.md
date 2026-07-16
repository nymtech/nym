## 1. Free-tier credential: wire + gateway seam (v1)

- [x] 1.1 Add `BandwidthCredential::FreeTier { token: String }` to `common/credentials-interface`; update every match site (only 3 exhaustive matches needed arms: gateway `handle_final_credential_claim`, `wireguard-private-metadata` server topup + shared v2 downgrade; `authenticator-requests`/`registration` use `.into_zk_nym()` wildcard, no change). Wire-format round-trip test added.
- [x] 1.2 Add the `FreeTier` arm to `handle_final_credential_claim` (`gateway/src/node/wireguard/new_peer_registration/mod.rs`): offline ed25519 JWT verification via new `nym-free-tier-check` crate against the reused (upgrade-mode) attester key, skip ecash verification, seed the free allowance (`seed_free_tier_bandwidth`, mirrors the testnet free-bandwidth path). Gated on `free_tier_enabled`.
- [x] 1.3 Keep `BandwidthClaim.kind` set to an existing wireguard `TicketType`; confirmed `insert_wireguard_peer` persists an ordinary wireguard `ClientType` (kind flows through `process_new_peer` unchanged) and no new `TicketType` variant was added, so nym-api / issuance is untouched
- [x] 1.4 Added the free allowance constant (`FREE_TIER_BANDWIDTH_ALLOWANCE_BYTES`, 100 MB placeholder) to `common/network-defaults`. Attester key REUSED from `upgrade_mode.attester_public_key` (same-signer decision) rather than a separate key/url. Added a nym-node `free_tier` config section (`enabled` + `debug.pool_bandwidth_per_second`, human-readable via bytesize, e.g. "10 MB").
- [x] 1.5 Confirmed both transports (LP `on_final_lp_request` and legacy `on_final_authenticator_request`) funnel through the shared `process_new_peer` -> `handle_final_credential_claim`, so the `FreeTier` arm covers both

## 2. Client-side free-tier credential (`common/bandwidth-controller`) (v1)

- [ ] 2.1 Add `NymCredential::FreeTrialToken { jwt, expiration }` and its store/retrieve arm (mirror `UpgradeModeToken`, using the emergency-credential path with a new `FREE_TRIAL_JWT_TYPE`)
- [ ] 2.2 Add `get_free_trial_token() -> Option<String>` to `BandwidthTicketProvider` plus the `BandwidthControllerRequest`/sender plumbing (mirror `get_upgrade_mode_token`)
- [ ] 2.3 Add a `FreeTrialFetcher` that obtains the token from the VPN-API and populates the store; it is not a `CredentialFetcher` (that trait is ticketbook-typed)
- [ ] 2.4 Leave `PreparedCredential` untouched; add unit tests for issuance/store/retrieve and the mock provider

## 3. Free-tier state and metering (v1)

- [ ] 3.1 Add a per-public-key free-tier record to `nym-gateway-storage`: `{ claimed_today, session_start, is_free }`
- [ ] 3.2 Enforce the daily single-claim guard at registration: a public key that already claimed today does not get a fresh allowance
- [ ] 3.3 Seed the volume allowance from the network-defaults constant (reuse the existing byte accounting path)
- [ ] 3.4 Add the session-time clock; check elapsed time at the existing bandwidth-flush cadence (coarse; no sub-second precision)
- [ ] 3.5 Trigger the exhaustion transition on whichever limit (bytes or time) is reached first

## 4. Rate limiting via `tc` (v1)

- [ ] 4.1 Add a traffic-control manager (nym-node) that shells out to `tc`; one-time HTB root + shared free pool on `nymwg` (egress) and the ingress path (police / IFB) for bidirectional shaping
- [ ] 4.2 Add/remove a peer to/from the pool via a per-peer classify filter keyed on the peer IP
- [ ] 4.3 Implement the off-switch: removing a peer's filter drops it to the default unlimited class without disconnecting (reused by garden and upgrade)
- [ ] 4.4 Node config for the pool size; rebuild pool membership from state on startup

## 5. Walled garden via `iptables` (v1)

- [ ] 5.1 Extend `scripts/nym-node-setup/network-tunnel-manager.sh` to pre-create an empty `NYM-GARDEN` chain and its jump scaffolding next to `NYM-EXIT`
- [ ] 5.2 Add a garden manager (nym-node) that inserts/deletes `-s <peerIP> -j NYM-GARDEN` in its own chain only, never touching operator rules
- [ ] 5.3 Node config for the purchase-endpoint allowlist; populate the garden chain's allow/deny logic
- [ ] 5.4 On exhaustion: leave the tc pool (full speed) and add the peer's garden rule instead of removing the peer
- [ ] 5.5 Reconcile the garden chain from free-tier state on startup; do not persist the node's rules; verify fail-closed behavior on crash
- [ ] 5.6 Reconnect-to-upgrade: when a formerly-free peer presents an ecash credential, clear the garden rule, clear the rate limit, and set `is_free = false`

## 6. Metrics (v1)

- [ ] 6.1 Expose a gauge for the number of active free-tier users (pool members, excluding garden)
- [ ] 6.2 Expose the configured free-tier pool allowance (mb/s)
- [ ] 6.3 Wire both into the existing `NymNodeMetrics` / prometheus exposition

## 7. Test harness (v1, kept lean)

- [ ] 7.1 Layer 0: Linux network-namespace integration test (cargo test, linux + root gated) that applies the tc pool and the garden `iptables` allowlist to a bare `nymwg`-style interface and asserts reachability - baseline reaches the open internet, garden reaches only the allowlisted endpoint
- [ ] 7.2 Layer 1: exercise the peer-controller lifecycle (register -> free -> garden -> cleared) via the existing `mock` feature (`MockEcashManager`) against a real kernel interface
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

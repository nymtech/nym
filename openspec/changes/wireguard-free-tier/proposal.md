## Why

Nym has no free or trial path today: a WireGuard (dVPN / fast-mode) client can only reach a gateway by presenting a paid zk-nym ecash credential. This blocks the most effective way to convert users in heavily censored regions, where the product's core value - reaching the open internet when other VPNs are blocked - is exactly what a skeptical user needs to see for themselves before paying. A free, rate-limited tier lets a user connect with no payment and minimal friction, experience that reachability, and then buy a subscription. The building blocks already exist in this monorepo: the gateway already accepts a non-ecash JWT credential (upgrade mode), already meters bandwidth per client, and already runs a privileged WireGuard exit datapath. This change assembles those into a free tier rather than inventing new machinery.

## What Changes

- Add a free-tier capability token: a credential-proxy-signed JWT presented in place of an ecash credential at registration, mirroring the existing upgrade-mode JWT end to end. It is a capability marker only; the byte/time allowance is a network-wide constant, not encoded in the token.
- Gateway: add a `BandwidthCredential::FreeTier` variant matched in `handle_final_credential_claim`, verified locally by ed25519 against a configured attester public key (no nym-api, no chain, no JWKS), that skips ecash verification and seeds a small allowance. No new `TicketType` (which would force nym-api changes); reuse an existing wireguard ticket type for the persisted client kind.
- Client (`common/bandwidth-controller`): add `NymCredential::FreeTrialToken`, a `get_free_trial_token` provider method, and a `FreeTrialFetcher`, mirroring the upgrade-mode plumbing; leave the ecash-only `PreparedCredential` untouched.
- Meter free sessions by both volume (reusing existing byte accounting, seeded from the network-defaults constant) and time (a new coarse session clock checked at the existing bandwidth-flush cadence); whichever is exhausted first ends the free allowance. Track a per-public-key free-tier record holding the daily-claim marker, session start, and free flag.
- Enforce a shared, bidirectional bandwidth cap for all free users on a gateway via Linux traffic control (`tc`, HTB pool) on the `nymwg` interface. Always admit free users and degrade under load rather than rejecting; expose the number of active free users and the pool allowance as metrics so the app can warn users.
- On exhaustion, move the peer into a walled garden instead of disconnecting: drop it from the rate-limit pool (full speed) and confine its egress to a purchase-endpoint allowlist via `iptables`, integrated with the operator's `scripts/nym-node-setup/network-tunnel-manager.sh` chain model. Full speed is safe because the allowlist confines it to the checkout path.
- Add a lean test harness: Linux network-namespace integration tests for the enforcement datapath, plus a single-real-gateway integration setup that points an unbonded node at mainnet.
- Reduced unlinkability for the free tier only is an accepted, scoped trade-off; paid tiers are unaffected. Issuance-side per-user rate-limiting is enforced externally at the VPN-API; as defense-in-depth the entry gateway ALSO applies a per-IP daily cap on free-tier tokens at redemption.
- The token carries an explicit `purpose` claim (new-user trial vs subscription renewal): a new-user token grants the free allowance and is IP-limited, whereas a renewal token grants no free bandwidth, is confined straight to the purchase walled garden, and is not IP-limited. The claim is required now (WIP, nothing deployed) so the format is fixed; the renewal-to-garden behavior lands with the walled garden and until then renewals are rejected.

## Capabilities

### New Capabilities

- `free-tier-access`: a credential-proxy-signed capability JWT accepted in place of an ecash credential, verified locally at the gateway, granting a network-constant free allowance across both registration transports, with no new ticket type and no nym-api dependency; client-side issuance/storage mirrors the upgrade-mode token.
- `free-tier-metering`: per-public-key free-tier state with a daily single-claim guard, plus combined volume and time metering that ends the allowance on whichever limit is reached first.
- `free-tier-rate-limiting`: a shared, bidirectional `tc` bandwidth pool for free users with bounded aggregate cost, graceful degradation instead of rejection, a rate-limit off-switch that does not disconnect the peer, and exposed free-tier metrics.
- `free-tier-walled-garden`: exhaustion routes the peer to a full-speed, allowlist-confined mode reachable only to the purchase endpoint, enforced by a node-managed `iptables` chain that is separate from operator-managed rules, rebuilt from state on start, unpersisted, and fail-closed.

### Modified Capabilities

<!-- None with an existing OpenSpec capability. The gateway credential path, bandwidth accounting, and WireGuard peer lifecycle are modified in code but have no prior capability spec; see Impact. -->

## Impact

- **Client:** `common/bandwidth-controller` (new `NymCredential` variant, provider method, `FreeTrialFetcher`); `common/credentials-interface` (new `BandwidthCredential::FreeTier`); the wire/registration crates that match on `BandwidthCredential` (`common/wireguard-private-metadata`, `common/authenticator-requests`, `common/registration`).
- **Gateway / node:** `gateway/src/node/wireguard/new_peer_registration` (new credential arm, free-tier state seeding), `common/wireguard` (peer handle: time metering, rate-limit toggle, garden transition), `nym-node` (traffic-control + garden managers, metrics), `nym-gateway-storage` (per-public-key free-tier record).
- **Config / constants:** `common/network-defaults` (free allowance constant, attester public key, pool default); node config (attester url/key, pool size, purchase allowlist).
- **Operator tooling:** `scripts/nym-node-setup/network-tunnel-manager.sh` gains a pre-created empty `NYM-GARDEN` chain plus jump scaffolding; the node manages that chain's per-peer contents at runtime.
- **New dependencies:** none beyond shelling out to `tc`/`iptables`, already present via `iproute2` alongside the existing `ip` calls.
- **Platform:** enforcement (tc/iptables, kernel forwarding) is Linux-only; the credential and metering logic is cross-platform.
- **Out of scope:** VPN-API token issuance and its issuance-side per-IP limiting (separate repo; the gateway's redemption-side per-IP cap IS in scope); the seamless in-session upgrade (v2); fixing the currently-broken full localnet (the mixnet contract now requires a node-families address the orchestrator does not deploy).

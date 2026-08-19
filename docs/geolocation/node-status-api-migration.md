# Deferred: migrating node status API onto the contract

Node status API keeps its own geolocation today. Moving it onto the contract is a **separate change**, with its own deltas against the `node-status-api-monitoring` and `node-status-api-http` capabilities. This note records what was already done for it, what it must decide, and the two things that constrain it.

## What exists today

Geolocation lives in `nym-node-status-api/src/monitor/geodata.rs`: an ipinfo client, a `moka` `Cache<NodeId, Location>` with a 24 hour TTL, and a serial sweep at step 9 of the monitor's strictly-ordered cycle. Results reach consumers by two routes that can disagree. The dVPN directory reads them out of the persisted `explorer_pretty_bond` JSONB, frozen at gateway-write time, while `/explorer/v3/nym-nodes` reads the live in-memory cache. Failed lookups are never cached, so a node whose addresses cannot be resolved is retried every cycle against a metered API.

## What is already in place

The adapters are written and tested, because the payload had to be designed against this surface anyway (see the payload-width constraint below):

- `impl From<geo::Location> for Location` in `http/models/mod.rs` maps the contract payload onto the public dVPN shape field for field.
- `impl From<geo::Asn> for Asn` and `impl From<geo::AsnKind> for AsnKind` do the same for the ASN record, deriving the public two-value form via `Asn::classify()` in the contract-common crate, which applies the same `"isp"` test the API applies today.
- Absent coordinates render as `0.0` at the HTTP boundary, preserving current behaviour, while absence stays explicit on chain because `0.0, 0.0` is a real location off West Africa rather than a missing one.

So the migration is not a data-shape problem. It is a **sourcing and policy** problem.

## What the migration has to decide

**Which entry to serve.** The contract stores opinions, not a verdict: a node may carry one entry per measuring agent, plus its own signed declaration, plus an admin override. Nothing picks between them for you. Whatever policy this API adopts becomes the de facto public answer, so it deserves to be stated explicitly rather than falling out of an iteration order.

**What to serve when there is no entry.** This is the substantive change, and it is where the cliff below moves to.

**Whether to verify.** The API can read the contract like any other consumer, or it can verify the digest and serve only what it has proven complete. The client verification flow in the [README](./README.md) applies unchanged.

## Constraint 1: payload width was frozen in advance

The key layout and leaf framing are a wire format committed to by the digest, so changing them is a breaking migration that has to re-fold every entry. That is why the payload was designed wide enough to serve this API's existing public surface *before* the migration was written: anything discarded at write time can only be recovered by re-measuring the whole network against a metered provider.

Two encodings deviate deliberately from strict parity, both settled in the contract's favour:

- **ASN stores the provider's raw type**, not the derived `residential | other`. Storing the derived form would permanently collapse `hosting`, `business` and `education` into `other`, and datacenter concentration is a decentralisation metric worth being able to ask about later. Consumers derive the two-value form; the adapter is a one-line match.
- **Coordinates are `Option`.** Every other field has an unambiguous absent form, but `0.0, 0.0` is a valid location, and nym-node currently carries only `location: Option<Country>`, so self-declared entries will essentially never have coordinates. Under a non-optional encoding every one of them would plot in the Gulf of Guinea.

One field **cannot** be reproduced: `geoip.ip_address`. No IP address is written on chain in any form, deliberately. Source it from the node's own announced addresses when present and leave it as the empty string otherwise, which is already this endpoint's no-data convention and which correctly leaves it empty for operators who announce only a hostname.

A related rule this API inherits: the payload module is behind a non-default `payload` feature because CosmWasm rejects floating-point instructions at upload and `Location` carries two `f64` coordinates. **No contracts-workspace member may take `payload` as a normal dependency.** Every payload consumer, this API included, lives in the root workspace and so cannot switch the feature on for the wasm build. `cosmwasm-check` in CI is the backstop that makes a violation loud rather than silent.

## Constraint 2: the cold-start cliff at `http/state.rs:431`

The dVPN directory filters gateways on the country code:

```rust
// 6. filter out nodes without valid country codes
if dvpn_gw.location.two_letter_iso_country_code.len() != 2 {
    warn!("Invalid country code: {}", dvpn_gw.location.two_letter_iso_country_code);
    continue;
}
```

Today a failed lookup yields `Location::empty()`, whose country code is the empty string, so that filter **silently removes the gateway from the dVPN directory**. A lookup failure and a genuine absence of data are indistinguishable at this point, and both present as a missing gateway. Separately, because the cache is in memory, a restart makes `/explorer/v3/nym-nodes` serve `geoip: null` until the sweep refills it.

Both disappear as accidents once the source is the contract, and that is the point:

- **Restart is no longer a cliff.** Entries live on chain, so there is nothing to refill and no window during which the API knows less than it did a minute ago.
- **An empty country is never written.** The geolocator rejects a provider response with no country rather than committing a claim that asserts nothing, and a failed lookup submits nothing at all, leaving the previous entry and its `checked_at` untouched.

The consequence is that "no location" becomes explicit rather than a placeholder that happens to fail a length check. The migration therefore has to *choose* what a node with no entry means for the dVPN directory, instead of inheriting an answer from a filter that was written to catch malformed data. It also now has the information to tell the cases apart: an entry with a recent `checked_at` and no entry at all are different facts, where today both arrive as an empty string.

## Sequencing

Nothing here blocks the geolocation change. The contract, the geolocator service and the adapters are complete and tested; this API keeps its current behaviour until the follow-up change lands.

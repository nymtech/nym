## Why

Node geolocation today is a side effect of the node status API's monitor loop: a `pub(crate)` ipinfo client behind an in-memory `moka` cache, swept serially once per cycle, with results persisted only incidentally inside each gateway's `explorer_pretty_bond` JSON blob. Nothing about it is verifiable, nothing outside `nym-node-status-api` can consume it, geolocation failures are never cached so they burn a metered third-party quota on every cycle, and a process restart empties the cache so `/explorer/v3/nym-nodes` serves `geoip: null` for every node until it refills. An empty country code silently removes a gateway from the dVPN directory, which makes an unverifiable, single-vendor, in-memory lookup load-bearing for dVPN routing.

The verifiable-directory work established a pattern for exactly this problem: hold the data in a CosmWasm contract that maintains an `nym-lthash` digest over its own state, so any client can pull the full set from an untrusted source and cryptographically prove it is complete and untampered. Node location is a good fit, and moving it there turns a private cache into a public, attributable, independently verifiable record.

## What Changes

- A new **`nym-geolocation-contract`** CosmWasm contract storing `(subject_class, subject_id, source) -> location` entries under a global LtHash digest, following the checklist in the directory contract's verifiable-digest pattern (canonical domain-separated leaves, a digest-maintenance wrapper no mutation bypasses, the collapsed digest at a fixed raw storage key, and paged enumeration for client-side recompute).
- A whitelist of authorised measurement agents, itself digest-committed so a client can verify who was permitted to write, with per-agent permissions separating measurement writes from self-declaration relays.
- A new standalone **geolocator service** that periodically queries the mixnet contract for bonded nodes, discovers their addresses from their HTTP endpoints, performs geolocation lookups, and submits batched results to the contract.
- Two measurement cadences: a regular sweep (monthly by default) and an explicit re-test triggered when a node's announced addresses change.
- An authenticated re-test HTTP endpoint on the service, accepting either a NYM-held bearer token (unlimited) or a request signed by the target node's identity key (burst-limited).
- Node self-declared location becomes independently attestable: a new signed `NymNodeLocation` artifact the node serves over HTTP and the geolocator relays, so the on-chain self-declaration carries the node's own signature rather than an agent's word.

## Capabilities

### New Capabilities

- `node-geolocation-contract`: the on-chain store. Entry key space, the closed `SubjectClass` and `Method` enums, the three sources and their authorisation rules, the opaque versioned payload, digest maintenance and the canonical leaf encoding, the agent whitelist, batched submission, `declared_at` monotonicity for relayed self-declarations, admin override, unbond cleanup, and the paged query surface a verifying client needs.
- `node-geolocation-service`: the off-chain agent. Subject discovery from the mixnet contract, address discovery from node HTTP endpoints, the regular and change-triggered measurement cadences, batching and submission, the re-test endpoint with both authentication modes and the burst limit, and the operational handling of the metered lookup quota.

### Modified Capabilities

None. This change is additive and touches no existing spec's requirements. The node status API migration is deliberately deferred (see Impact).

## Impact

**New code**: a `contracts/geolocation/` contract plus its `common/cosmwasm-smart-contracts/geolocation-contract/` shared types crate, a new service crate, and query/signing traits in `nym-validator-client`. The digest machinery is copy-and-adapted from the directory contract rather than extracted into a shared crate: the blast radius is small, the two contracts' `domain_tag`s keep their leaves apart, and generic cw-storage-plus wrappers cost more than they save at two consumers.

**Merge order**: this change lands before `feat/node-directory-publishing`, which is then rebased and remade on top of it. `nym-lthash` is already on develop, and the digest machinery is copy-and-adapted rather than imported, so the contract and service depend on nothing unmerged. The reference implementation they mirror (`contracts/directory/`, `common/nym-directory-client/`) lives only on that branch and is read, not linked. Two consequences follow: end-to-end client verification is gated on `nym-directory-client` reaching develop and is sequenced after the directory merge, and `NodeInformation.location` is dropped during the directory rebase, which makes this contract the single home for a node's self-declared location.

**Deferred follow-up, and the constraint it imposes now**: a subsequent change will replace the node status API's ipinfo client with reads from this contract, deleting the `moka` geodata cache, the in-cycle serial sweep, and the required `IPINFO_API_TOKEN` configuration, and carrying spec deltas against `node-status-api-monitoring` and `node-status-api-http`. That follow-up is out of scope here, but it constrains the payload designed here rather than merely following it, so the constraint is discharged by construction: every entry, whether measured, node-declared or admin-overridden, carries the same `Location` payload that node status API already serves on its dVPN surface, complete with the ASN record (`asn`, `name`, `domain`, `route`, `kind`). A narrower payload of country plus coordinates would look sufficient today and strand the migration behind a contract migration later. The one field that cannot be reproduced is `geoip.ip_address`, since IP addresses are deliberately kept off-chain, and it will be sourced from the node's own announced addresses or left empty.

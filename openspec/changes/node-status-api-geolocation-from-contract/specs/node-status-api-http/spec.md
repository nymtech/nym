## MODIFIED Requirements

### Requirement: `/explorer/v3/nym-nodes` SHALL return described nodes enriched with stake, geo and family data

`GET /explorer/v3/nym-nodes` MUST return `PagedResult<ExtendedNymNode>` built from the set of nodes that have a stored self-description (nodes without one MUST be absent), with exactly these fields:

```json
{
  "node_id": 0,
  "identity_key": "<base58 ed25519>",
  "uptime": 0.0,                      // nym-api performance as f64 0..1, 0.0 when unknown
  "total_stake": "0",                 // Decimal as string, "0" when unbonded
  "original_pledge": 0,               // u128 number, 0 when unbonded
  "bonding_address": null,            // owner address, null when unbonded
  "bonded": false,                    // true iff bond info is present
  "node_type": "nym_node",            // legacy_mixnode | legacy_gateway | nym_node
  "ip_address": "",                   // first declared host IP, "" when none
  "accepted_tnc": false,
  "self_description": { },            // full stored self-description (NymNodeDataV2 shape)
  "rewarding_details": { },           // mixnet-contract NodeRewarding, or null when unbonded
  "description": { "moniker": "", "website": "", "security_contact": "", "details": "" },
  "geoip": { "city": "", "country": "", "ip_address": "", "latitude": "", "longitude": "", "org": "", "postal": "", "region": "", "timezone": "" },
  "family_data": { "id": 0, "name": "", "description": "", "family_stake": 0, "members": 0 }
}
```

`geoip` and `family_data` MUST be `null` when the node has no resolved entry in the geolocation snapshot or belongs to no family. The whole response MUST be built against a single snapshot, so every node's `geoip` in one response comes from one height rather than from independently expiring per-node lookups.

`geoip.ip_address` MUST NOT be sourced from geolocation data, because no IP address is written on chain in any form. It MUST carry the node's first declared host IP, the same value as the top-level `ip_address` field, and the empty string when the node declares none. This correctly leaves it empty for operators announcing only a hostname.

`latitude`/`longitude` inside `geoip` MUST be strings (stringified floats), unlike the numeric `latitude`/`longitude` in the dVPN `location` object. An entry without coordinates MUST render them as the stringified zero value, preserving the shape a consumer sees today. `description` MUST fall back to an all-empty-strings object when no scraped description exists. A failure while aggregating the list MUST return 500.

`GET /explorer/v3/nym-nodes/{node_id}/delegations` MUST return a bare JSON **array** of `{ "amount": { "denom": "", "amount": "" }, "cumulative_reward_ratio": "", "block_height": 0, "owner": "", "proxy": null }`. It MUST return 404 with `No delegation data for node_id={node_id}` only when the node has no entry in the delegations cache (not bonded, or its chain query failed); a bonded node with zero delegations MUST return 200 with `[]`. A non-numeric `{node_id}` MUST be rejected by the path extractor with 400.

#### Scenario: Node with no delegators

- **GIVEN** a bonded node whose delegation query succeeded and returned nothing
- **WHEN** its delegations are requested
- **THEN** the response is 200 with `[]`

#### Scenario: Unbonded node

- **GIVEN** a node id absent from the delegations cache
- **WHEN** its delegations are requested
- **THEN** the response is 404 with the body `No delegation data for node_id=<id>`

#### Scenario: Undescribed node hidden

- **GIVEN** a bonded node with no stored self-description
- **WHEN** `GET /explorer/v3/nym-nodes` is called
- **THEN** the node is absent from the list and from `total`

#### Scenario: One response is built from one snapshot

- **GIVEN** a geolocation refresh that completes while a response is being aggregated
- **WHEN** the response is served
- **THEN** every node's `geoip` in it comes from the same snapshot, rather than some nodes reflecting the refresh and others not

#### Scenario: geoip carries the declared IP, not a geolocated one

- **GIVEN** a node with a resolved geolocation entry and one declared host IP
- **WHEN** its `geoip` is rendered
- **THEN** `geoip.ip_address` is that declared IP, and it is the empty string for a node declaring only a hostname

### Requirement: The dVPN directory SHALL apply this exact filter, enrich, and sort pipeline

The dVPN gateway list MUST be built from the cached gateway list by, in order: (1) dropping gateways with `bonded=false`; (2) dropping gateways with `performance == 0`; (3) dropping gateways with no matching row in the nym-nodes table (matched by base58 ed25519 identity); (4) attaching family, staking and SOCKS5 percentile data, and resolving the gateway's location from the geolocation snapshot by node id; (5) dropping gateways whose `explorer_pretty_bond` or `self_described` JSON is missing or unparsable (a missing `build_information` therefore removes the node); (6) dropping gateways whose resolved `location.two_letter_iso_country_code` is not exactly 2 characters; (7) sorting by `(two_letter_iso_country_code, identity_key)` ascending. Each item MUST be:

```json
{
  "identity_key": "<base58 ed25519>",
  "name": "<moniker>",
  "description": "<details>",
  "ip_packet_router": { "address": "" },        // or null
  "authenticator": { "address": "" },           // or null
  "location": {
    "two_letter_iso_country_code": "CH",
    "latitude": 0.0, "longitude": 0.0,
    "city": "", "region": "", "org": "", "postal": "", "timezone": "",
    "asn": { "asn": "", "name": "", "domain": "", "route": "", "kind": "residential" }
  },
  "last_probe": { "last_updated_utc": "", "outcome": { } },   // or null
  "ip_addresses": ["1.2.3.4"],
  "mix_port": 1789,
  "role": "EntryGateway",                       // or {"Mixnode":{"layer":1}} | "ExitGateway" | "Standby" | "Inactive"
  "entry": { "hostname": null, "ws_port": 9000, "wss_port": null },  // or null
  "bridges": { "version": "", "transports": [ { "transport_type": "quic_plain", "args": { } } ] },  // or null
  "performance": "0.95",
  "performance_v2": { "last_updated_utc": "", "score": "high", "mixnet_score": "high", "load": "low", "uptime_percentage_last_24_hours": 0.95 },
  "lewes_protocol_details": { "content": { "enabled": true, "control_port": 41264, "data_port": 51264, "x25519": "", "kem_keys": { } }, "signature": "" },
  "family_data": { "id": 0, "name": "", "description": "", "family_stake": 0, "members": 0 },
  "staking_data": { "total_stake": 0, "total_delegations": 0, "total_bond": 0, "delegations": 0 },
  "build_information": { },                     // full build-info object, same 10 fields as /v2/status/build_information
  "ports_check": { },                           // or null
  "last_ports_check_utc": ""                    // or null
}
```

The `location` object MUST be derived from the resolved geolocation entry, never from the persisted `explorer_pretty_bond` row. A gateway with no resolved entry MUST yield an empty two-letter country code and therefore MUST be dropped at step (6), which is the same outcome a failed geolocation lookup produced before this change. Coordinates are optional on chain, because `0.0, 0.0` is a real location rather than a missing one; an entry without them MUST render `latitude` and `longitude` as `0.0`, preserving what consumers see today.

`performance` MUST be the nym-api mixnet performance rendered as a fixed 2-decimal string (`performance / 100`, e.g. `"0.95"`, and `"2.55"` for an out-of-range stored value of 255). `location.asn.kind` MUST be `residential` when the ASN type recorded on chain equals `isp` case-insensitively, otherwise `other`; the contract stores the provider's raw type rather than the derived two-value form, and the derivation MUST apply that same test. `name` MUST be the moniker and `description` MUST be the details string from the scraped description (both `NA` when unscraped, and `description` is always present rather than null). `role`, `entry`, `ip_addresses` and `mix_port` MUST come from the matched nym-nodes row (so `role` uses externally-tagged enum encoding), whereas `ip_packet_router`, `authenticator`, `lewes_protocol_details` and `build_information` MUST come from the gateway's own stored self-description - `build_information.build_version` is also what the read-time version filter parses.

#### Scenario: Gateway missing from nym-nodes is dropped

- **GIVEN** a bonded gateway with nonzero performance that has no nym-nodes row
- **WHEN** the dVPN list is built
- **THEN** it is excluded and the discrepancy is logged as critical

#### Scenario: Invalid country code drops the node

- **GIVEN** a gateway whose resolved country code is empty or longer than 2 characters
- **WHEN** the dVPN list is built
- **THEN** it is excluded

#### Scenario: Gateway with no contract entry is dropped

- **GIVEN** a bonded gateway with nonzero performance that has no resolved entry in the geolocation snapshot
- **WHEN** the dVPN list is built
- **THEN** its country code is empty and it is excluded, exactly as a gateway whose geolocation lookup failed was excluded before this change

#### Scenario: Entry without coordinates still renders

- **GIVEN** a gateway whose resolved entry carries a country but no coordinates
- **WHEN** the dVPN list is built
- **THEN** it is retained, and its `latitude` and `longitude` are rendered as `0.0`

#### Scenario: Deterministic order

- **WHEN** the dVPN list is served
- **THEN** items are ordered by country code then identity key, so `countries` responses can rely on adjacency

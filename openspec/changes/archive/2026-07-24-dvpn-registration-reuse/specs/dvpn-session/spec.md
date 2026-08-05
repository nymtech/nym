# Delta: dvpn-session — registration persistence and reuse

## ADDED Requirements

### Requirement: Gateway registration persistence and reuse

`nym-sdk-session` SHALL persist each successful gateway registration —
client WireGuard keypair, the gateway-returned WireGuard configuration
(assigned addresses, gateway public key, optional PSK, endpoint), gateway
identity, hop role, and registration time — in the session's data
directory, keyed by (network name, gateway identity, role). When a
registration is requested against a gateway and role for which a usable
cached entry exists (and reuse has not been disabled), the session SHALL
return a registration assembled from the cached state WITHOUT contacting
the gateway and WITHOUT spending a ticket. A fresh gateway exchange (and
its one-ticket spend) SHALL occur only for hops with no usable cached
entry — including partially cached two-hop requests, where only the missing
hop is registered. The session SHALL provide an invalidation operation
removing a cached entry by (gateway, role), enabling callers to fall back
to fresh registration when a cached peer no longer works; reuse SHALL be
default-on with a `SessionConfig` opt-out for callers requiring a fresh
peer per connection (the linkability trade-off of reuse SHALL be
documented on that option). The persisted file SHALL be written atomically,
created with owner-only permissions on unix, and treated as absent (never
a crash) when missing or unparseable. Entries older than a conservative
maximum age MAY be treated as absent and pruned.

#### Scenario: Repeat connection to a known gateway spends nothing

- **WHEN** a session registers with gateways it has previously registered
  with (same network, same roles) and the cached entries are usable
- **THEN** the returned registration is served from the cache, no LP
  exchange takes place, and no ticket is spent — verified by an unchanged
  `used_tickets` count

#### Scenario: Cache survives process restart

- **WHEN** a process registers, exits, and a new process creates a session
  over the same data directory
- **THEN** the new session finds and reuses the persisted registration

#### Scenario: Partial cache registers only the missing hop

- **WHEN** a two-hop registration finds a cached entry for one hop but not
  the other
- **THEN** only the uncached hop performs a gateway exchange and spends a
  ticket; the cached hop is reused

#### Scenario: Invalidation enables funds-safe fallback

- **WHEN** a caller invalidates a (gateway, role) entry after a cached
  registration failed to establish, and registers again
- **THEN** the entry is removed from the persistent cache and the new
  registration performs a fresh exchange (spending one ticket), which is
  then persisted in its place

#### Scenario: Opt-out forces fresh peers

- **WHEN** a session is configured with registration reuse disabled
- **THEN** every registration performs a fresh gateway exchange and neither
  reads nor is served from the cache

#### Scenario: Network isolation of cached entries

- **WHEN** the same data directory is used against a different network
- **THEN** cached entries recorded under another network name are never
  reused

#### Scenario: Corrupt or missing cache degrades to fresh registration

- **WHEN** the cache file is absent, unreadable, or fails to parse
- **THEN** the session behaves as if no registrations are cached (fresh
  exchange, then persist), and never fails or panics on the cache's account

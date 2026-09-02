## MODIFIED Requirements

### Requirement: Gateway selection by identity, country, or random

The session SHALL select gateways by one of: exact identity key, two-letter ISO 3166
country code, or uniformly random, filtered to nodes that support the required
WireGuard role. Selection SHALL accept a set of **excluded** gateway identities that
are never returned. A pinned `Identity` spec SHALL never be silently substituted: if
the pinned identity is in the excluded set, selection fails with the
distinct-gateways error rather than returning a different gateway. `Country` and
`Random` specs SHALL skip every excluded identity when choosing among eligible
candidates; if excluding leaves no eligible candidate, selection fails with the
no-match error for that spec.

#### Scenario: Select by identity key
- **WHEN** the caller specifies a gateway identity key
- **THEN** the matching gateway is selected, or an error is returned if not found or
  it lacks WireGuard support

#### Scenario: Select by country code
- **WHEN** the caller specifies a two-letter country code
- **THEN** a WireGuard-capable gateway located in that country is chosen (randomly
  among matches, skipping any excluded identities), or an error is returned if none
  match

#### Scenario: Select randomly
- **WHEN** the caller requests a random gateway
- **THEN** a WireGuard-capable gateway is chosen uniformly at random from the eligible
  set

#### Scenario: Random selection skips an excluded set
- **WHEN** the caller requests a random gateway and passes a set of excluded
  identities
- **THEN** the chosen gateway is never one of the excluded identities; if every
  eligible gateway is excluded, selection fails with the no-gateway error

#### Scenario: Excluded pinned identity is not substituted
- **WHEN** the caller pins a gateway by identity that is also in the excluded set
- **THEN** selection fails with the distinct-gateways error and never returns a
  different gateway

#### Scenario: Two-hop selects distinct gateways
- **WHEN** a two-hop tunnel is registered
- **THEN** the entry gateway is excluded from the exit selection so the two hops never
  resolve to the same gateway; if the exit spec can only match the entry gateway,
  registration fails with a distinct-gateways error

## ADDED Requirements

### Requirement: Two-hop registration can avoid implicated entry gateways

The session SHALL provide a two-hop registration path that accepts a set of **entry**
gateway identities to avoid, excluding them from entry selection so a caller that has
implicated an entry gateway (for example one that does not forward the tunnelled exit
handshake) can re-register onto a different entry. The exit selection SHALL continue
to exclude the chosen entry so the two hops stay distinct. When the entry spec is a
pinned identity that is in the avoid set, registration SHALL fail with the
distinct-gateways error rather than substituting a different gateway. The existing
two-hop registration entry point SHALL behave exactly as before, equivalent to
passing an empty avoid set.

#### Scenario: Random entry avoids an implicated entry
- **WHEN** a two-hop registration is requested with a random entry spec and an avoid
  set containing a previously-implicated entry identity
- **THEN** the selected entry gateway is not in the avoid set

#### Scenario: Pinned entry in the avoid set fails without substitution
- **WHEN** a two-hop registration is requested with a pinned entry identity that is
  in the avoid set
- **THEN** registration fails with the distinct-gateways error and no other gateway
  is substituted

#### Scenario: Empty avoid set preserves existing behavior
- **WHEN** a two-hop registration is requested with an empty avoid set
- **THEN** entry and exit are selected and registered exactly as the existing
  two-hop registration path does

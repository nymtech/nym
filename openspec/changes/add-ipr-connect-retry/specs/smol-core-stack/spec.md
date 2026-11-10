# smol-core-stack Spec Delta

## ADDED Requirements

### Requirement: Discovery retains all eligible exits in preference order

IPR auto-discovery SHALL return every eligible exit as an ordered candidate list, in performance-weighted order, rather than collapsing the set to a single pick. The existing eligibility filters (exit-IPR role, version at or above the minimum supported release, and a parseable recipient address) SHALL be unchanged. The directory SHALL be fetched once per establishment; rotation SHALL reuse the retained list without re-fetching. An empty candidate list SHALL surface as a distinct no-eligible-exits error, not as a connection timeout.

#### Scenario: All eligible exits are retained

- **WHEN** auto-discovery runs and several exits pass the eligibility filters
- **THEN** every passing exit is returned as an ordered candidate list, highest-performance-weighted first, rather than a single pick

#### Scenario: No eligible exits is a distinct error

- **WHEN** no exit passes the eligibility filters
- **THEN** establishment returns a no-eligible-exits error, distinguishable from a handshake timeout

### Requirement: Fast-fail IPR establishment with exit rotation

On the auto-discovery path (no pinned IPR), tunnel establishment SHALL attempt the IPR handshake against candidates in preference order and SHALL rotate to the next candidate on handshake failure. Rotation SHALL be bounded by a maximum attempt count and by the candidate list length. Each attempt SHALL run the protocol-version fallback (v10 then v9), because a node's advertised directory version can lead its running process; a per-exit budget that strangles the fallback would abandon exits that are reachable only on the older version. The per-version timeout SHALL bound each version attempt (so one exit may spend up to two, a v10 probe then a v9 connect, before rotating), rather than a single outer timeout across the whole handshake. The per-version timeout and the maximum attempt count SHALL be tunable, with defaults of 6 seconds and 5 attempts, and SHALL take effect only on the auto-discovery path.

#### Scenario: An exit reachable only on the older version still connects

- **WHEN** a candidate advertises the newer protocol version but its running process answers only the older one
- **THEN** the newer-version probe times out, the attempt falls through to the older version, and the exit connects rather than being abandoned

#### Scenario: A dead exit is abandoned and the next is tried

- **WHEN** a candidate answers neither protocol version within their per-version budgets
- **THEN** that exit is abandoned and the handshake is attempted against the next candidate

#### Scenario: Establishment succeeds on a later candidate

- **WHEN** an earlier candidate fails and a later candidate completes its handshake
- **THEN** the tunnel is established against the later candidate

#### Scenario: Rotation is bounded

- **WHEN** attempts reach the maximum attempt count while further candidates remain
- **THEN** fast rotation stops rather than walking the entire list

### Requirement: Pinned IPR is not rotated

When a specific IPR is pinned, establishment SHALL keep the existing single-attempt behaviour with the full connect timeout and cross-version fallback, and SHALL NOT rotate, because a single fixed exit offers nothing to rotate to. Fast-fail rotation is an auto-discovery behaviour only.

#### Scenario: Pinned IPR keeps existing behaviour

- **WHEN** a specific IPR address is pinned and its handshake is slow
- **THEN** establishment waits out the full connect timeout against that one exit and does not rotate

### Requirement: Failed attempts are visible

Each failed establishment attempt SHALL be logged with the exit address, the elapsed time, and the failure reason, and the final outcome SHALL be logged, so a black-holing exit is observable rather than hidden inside a single silent stall.

#### Scenario: A black-holing exit is logged

- **WHEN** an exit accepts the connection but never answers and the attempt times out
- **THEN** the exit address, elapsed time, and timeout reason are logged before rotation

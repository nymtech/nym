## ADDED Requirements

### Requirement: Every attested contract is produced together

nym-api SHALL produce the snapshots of every contract it attests together: one production per cadence height `H` covering every attested contract, in which the mixnet bond set at `H` is read once and supplied to every contract's verified read at `H` as its node-identity map. It SHALL sign and retain the snapshots at `H` only when the bond read, the `app_hash` the shared chain anchor trusts for `H`, and every contract's verified read at `H` all succeeded, and SHALL retain them as one entry holding every contract's snapshot at `H`. When any of them fails, it SHALL retain nothing for `H`, MUST log each failure naming its contract, and the next poll SHALL try the newest cadence height again. The startup production SHALL follow the same rule height by height, oldest first, logging and skipping a height it cannot produce for every contract.

The retained entries SHALL be persisted in one file, loaded at startup; a file that fails to load SHALL be treated as an empty cache, which the startup production refills. Each contract SHALL keep its own route tree, serving its snapshot out of the shared entries.

#### Scenario: One contract's failure stops the height for all

- **WHEN** the geolocation contract's verified read at `H` fails while the directory's and performance's succeed
- **THEN** no contract retains a snapshot for `H`, the failure is logged naming the geolocation contract, and the next poll tries `H` again

#### Scenario: Every retained height serves every contract

- **WHEN** a nym-api has produced snapshots over several cadence heights
- **THEN** every height it retains holds a snapshot of every attested contract, and every contract's latest snapshot is at the same height

#### Scenario: The bond set is read once per height

- **WHEN** a nym-api produces the cadence height `H`
- **THEN** it reads the mixnet bond set at `H` once, and every contract's snapshot at `H` carries the hash of that one identity map

#### Scenario: A restart keeps every contract's heights together

- **WHEN** nym-api restarts within one cadence interval of its last snapshot
- **THEN** it serves every contract's retained snapshots from the one loaded file, together with what the startup production adds

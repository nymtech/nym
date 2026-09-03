## ADDED Requirements

### Requirement: Snapshot cadence parameter

The contract SHALL store a `snapshot_interval` (a positive number of blocks) set at instantiate and mutable only by the admin, and SHALL expose it via a query. It dictates the heights at which attestation producers snapshot the directory, so independent producers converge on identical snapshot heights. The interval SHALL be stored as a plain configuration item: it SHALL NOT be committed into the global integrity digest, SHALL NOT form or alter any digest leaf, and SHALL NOT affect entry storage. Setting or updating it SHALL leave the global digest unchanged. An interval of zero SHALL be rejected.

#### Scenario: Interval set at instantiate

- **WHEN** the contract is instantiated with a positive `snapshot_interval`
- **THEN** the interval is stored and returned by its query, and the global digest is unaffected

#### Scenario: Admin updates the interval

- **WHEN** the admin submits an update to the snapshot interval with a positive value
- **THEN** the stored interval changes to the new value and the global digest is unchanged

#### Scenario: Non-admin update rejected

- **WHEN** a non-admin account attempts to update the snapshot interval
- **THEN** the contract rejects the message and the interval is unchanged

#### Scenario: Zero interval rejected

- **WHEN** an interval of zero is supplied at instantiate or in an update
- **THEN** the contract rejects it

#### Scenario: Interval is not part of the digest

- **WHEN** the snapshot interval is set or updated
- **THEN** recomputing the global digest over the full set of stored entries still equals the stored digest (the interval contributes no leaf)

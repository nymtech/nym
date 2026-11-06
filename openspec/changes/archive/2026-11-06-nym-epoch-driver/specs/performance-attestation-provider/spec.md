## MODIFIED Requirements

### Requirement: Every nym-api produces attested performance snapshots

nym-api SHALL produce signed snapshots of the performance contract's committed set on every instance, whatever its configuration. No setting SHALL turn the producer off. The performance contract's address SHALL be assumed present in the network details, and network details that do not carry it MUST fail startup, as a missing directory contract address does.

#### Scenario: A default configuration produces snapshots
- **WHEN** nym-api starts with a default configuration and network details carrying the performance contract's address
- **THEN** it produces performance snapshots and serves them under `/v1/performance`

#### Scenario: A missing address fails startup
- **WHEN** the network details carry no performance contract address
- **THEN** startup fails, whatever the configuration

## MODIFIED Requirements

### Requirement: Every attested contract is produced together

nym-api SHALL produce the snapshots of every contract it attests together: one production per cadence height `H` covering every attested contract, in which the node details of every bonded node at `H` are read once and the node-identity map derived from them is supplied to every contract's verified read at `H`. It SHALL sign and retain the snapshots at `H` only when the node details read, the `app_hash` the shared chain anchor trusts for `H`, and every contract's verified read at `H` all succeeded, and SHALL retain them as one entry holding every contract's snapshot at `H`. When any of them fails, it SHALL retain nothing for `H`, MUST log each failure naming its contract, and the next poll SHALL try the newest cadence height again. The startup production SHALL follow the same rule height by height, oldest first, logging and skipping a height it cannot produce for every contract.

Each contract's verified read at `H` SHALL first establish the digest trusted at `H`. When that digest equals the one signed in the same contract's snapshot of the newest retained entry below `H`, the read SHALL take that snapshot's records as the records at `H` and MUST NOT request them again; otherwise, or when no entry below `H` is retained, it SHALL read every record at `H` and verify them against the digest. A snapshot built from reused records SHALL be signed for `H` with the identity map read at `H`, like any other.

The retained entries SHALL be persisted in one file, loaded at startup; a file that fails to load SHALL be treated as an empty cache, which the startup production refills. Each contract SHALL keep its own route tree, serving its snapshot out of the shared entries.

#### Scenario: One contract's failure stops the height for all

- **WHEN** the geolocation contract's verified read at `H` fails while the directory's and performance's succeed
- **THEN** no contract retains a snapshot for `H`, the failure is logged naming the geolocation contract, and the next poll tries `H` again

#### Scenario: Every retained height serves every contract

- **WHEN** a nym-api has produced snapshots over several cadence heights
- **THEN** every height it retains holds a snapshot of every attested contract, and every contract's latest snapshot is at the same height

#### Scenario: The node details are read once per height

- **WHEN** a nym-api produces the cadence height `H`
- **THEN** it reads the node details at `H` once, makes no separate bond query, and every contract's snapshot at `H` carries the hash of the one identity map derived from them

#### Scenario: An unchanged contract is not read again

- **WHEN** the geolocation digest trusted at `H` equals the digest of the geolocation snapshot retained at the previous cadence height
- **THEN** the geolocation snapshot at `H` holds that snapshot's records, is signed for `H` with the identity map read at `H`, and no geolocation record is requested at `H`

#### Scenario: A changed contract is read in full

- **WHEN** the directory digest trusted at `H` differs from the one retained at the previous cadence height, or no height below `H` is retained
- **THEN** every directory record is read at `H` and verified against the digest trusted at `H`

#### Scenario: A restart keeps every contract's heights together

- **WHEN** nym-api restarts within one cadence interval of its last snapshot
- **THEN** it serves every contract's retained snapshots from the one loaded file, together with what the startup production adds

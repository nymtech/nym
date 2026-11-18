## MODIFIED Requirements

### Requirement: Startup builds its caches before spawning tasks or serving HTTP

nym-api SHALL run the attested snapshot producer's startup production of every attested contract, together with the mixnet contract state and node families data read beside each height, and SHALL wait until a snapshot has settled, every route's node, epoch and family data being read from it, before spawning any other task (the DKG controller, the ecash background cleaner and the signers cache refresher) and before serving HTTP. A settled snapshot loaded from disk SHALL count. While waiting, nym-api MUST log that it is waiting for a settled snapshot, and the wait SHALL have no time limit: nothing correct can be served before one exists. A shutdown signal during the startup production SHALL end it early, so that startup stops at the wait.

There SHALL be no other startup cache: nym-api has no mixnet contract cache, describe cache or node families cache to build.

The signers cache is exempt: its refresher probes this nym-api's own HTTP after a start delay, so it SHALL be spawned after the wait, letting that delay run from when HTTP comes up. Caches filled on demand are exempt too.

#### Scenario: A restart with a settled snapshot on disk starts at once
- **WHEN** nym-api restarts and the retained heights loaded from its file include a settled one
- **THEN** it spawns its tasks and serves HTTP without waiting for a production

#### Scenario: A fresh install serves nothing until a snapshot settles
- **WHEN** nym-api starts with no persisted snapshots
- **THEN** it serves no route and spawns no other task until a snapshot has settled, logging that it is waiting for one

#### Scenario: The signers cache does not report this nym-api as down
- **WHEN** a signer nym-api starts from a fresh install whose startup production outlasts the signers cache's start delay
- **THEN** the signers cache's first probe of this nym-api runs after its HTTP is served

#### Scenario: A shutdown during the warm-ups is not held until they finish
- **WHEN** a shutdown signal arrives while the snapshot producer's startup production is retrieving a retained height
- **THEN** the production stops at the height being retrieved and nym-api shuts down without spawning its other tasks

## REMOVED Requirements

### Requirement: A failed first build is retried every 30 seconds

**Reason**: The requirement governed the refreshers of the mixnet contract, describe and node families caches, none of which exists any more; their data is read inside the snapshot production.

**Migration**: A height the startup production cannot produce is skipped, and the snapshot producer's poll (every `[directory.debug].polling_interval`, 30 seconds by default) tries the newest cadence height again, so a transient failure at startup still costs seconds.

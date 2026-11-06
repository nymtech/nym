## MODIFIED Requirements

### Requirement: Startup builds its caches before spawning tasks or serving HTTP

nym-api SHALL start the refreshers of its startup caches (the mixnet contract cache, the describe cache and the node families cache), with the attested snapshot producer running its startup production of every attested contract beside them, and SHALL wait until each of those three caches holds a value and a performance snapshot has settled, every node's published performance being read from it, before spawning any other task (the DKG controller, the ecash background cleaner, key rotation and the signers cache refresher) and before serving HTTP. A value loaded from a persisted file still within its age limit SHALL count, and so SHALL a settled snapshot loaded from disk. While waiting, nym-api MUST log the caches still pending, and the wait SHALL have no time limit: nothing correct can be served before the caches exist. A shutdown signal during the startup production SHALL end it early, so that startup stops at the wait.

The signers cache is exempt: its refresher probes this nym-api's own HTTP after a start delay, so it SHALL be spawned after the wait, letting that delay run from when HTTP comes up. Caches filled on demand are exempt too.

#### Scenario: A restart with fresh persisted caches starts at once
- **WHEN** nym-api restarts and every startup cache's persisted file is within its age limit
- **THEN** it spawns its tasks and serves HTTP without waiting for any refresh

#### Scenario: A fresh install serves nothing until its caches are built
- **WHEN** nym-api starts with no persisted caches
- **THEN** it serves no route and spawns no other task until the describe pass has completed and a performance snapshot has settled, logging the caches still pending

#### Scenario: The signers cache does not report this nym-api as down
- **WHEN** a signer nym-api starts from a fresh install whose describe pass outlasts the signers cache's start delay
- **THEN** the signers cache's first probe of this nym-api runs after its HTTP is served

#### Scenario: A shutdown during the warm-ups is not held until they finish
- **WHEN** a shutdown signal arrives while the snapshot producer's startup production is retrieving a retained height
- **THEN** the production stops at the height being retrieved and nym-api shuts down without spawning its other tasks

### Requirement: A failed first build is retried every 30 seconds

While a startup cache holds no value, its refresher SHALL retry a failed build every 30 seconds, and SHALL refresh at its normal interval once the cache holds one, a value loaded from a persisted file counting, so that a transient failure at startup costs seconds rather than a whole refresh interval.

#### Scenario: A transient failure delays startup by seconds
- **WHEN** the node families cache's first build fails once and its next attempt succeeds
- **THEN** the retry runs 30 seconds after the failure, not after the cache's 600-second interval, and startup continues once it succeeds

#### Scenario: The normal interval resumes after the first success
- **WHEN** a startup cache's first build has succeeded and a later refresh fails
- **THEN** the next attempt waits for the cache's normal refresh interval

#### Scenario: A persisted value is refreshed at the normal interval
- **WHEN** a startup cache was loaded from a persisted file within its age limit and its first refresh fails
- **THEN** the next attempt waits for the cache's normal refresh interval

## REMOVED Requirements

### Requirement: No cache serves a placeholder before its first build
**Reason**: Its subject, the node annotation cache, is removed: each node's performance is read per request from the latest settled performance snapshot, which startup waits for.
**Migration**: None; the cache's persisted file is no longer read or written.

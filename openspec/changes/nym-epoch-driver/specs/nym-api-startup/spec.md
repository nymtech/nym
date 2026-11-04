## MODIFIED Requirements

### Requirement: Startup builds its caches before spawning tasks or serving HTTP

nym-api SHALL start the refreshers of its startup caches (the mixnet contract cache, the describe cache, the node families cache and the node annotation cache), with the attested snapshot producer running its startup production of every attested contract beside them, and SHALL wait until each of those four caches holds a value before spawning any other task (the DKG controller, the ecash background cleaner, key rotation and the signers cache refresher) and before serving HTTP. A value loaded from a persisted file still within its age limit SHALL count. While waiting, nym-api MUST log the caches still pending, and the wait SHALL have no time limit: nothing correct can be served before the caches exist. A shutdown signal during the startup production SHALL end it early, so that startup stops at the wait.

The signers cache is exempt: its refresher probes this nym-api's own HTTP after a start delay, so it SHALL be spawned after the wait, letting that delay run from when HTTP comes up. Caches filled on demand are exempt too.

#### Scenario: A restart with fresh persisted caches starts at once
- **WHEN** nym-api restarts and every startup cache's persisted file is within its age limit
- **THEN** it spawns its tasks and serves HTTP without waiting for any refresh

#### Scenario: A fresh install serves nothing until its caches are built
- **WHEN** nym-api starts with no persisted caches
- **THEN** it serves no route and spawns no other task until the describe pass and the first annotation refresh have completed, logging the caches still pending

#### Scenario: The signers cache does not report this nym-api as down
- **WHEN** a signer nym-api starts from a fresh install whose describe pass outlasts the signers cache's start delay
- **THEN** the signers cache's first probe of this nym-api runs after its HTTP is served

#### Scenario: A shutdown during the warm-ups is not held until they finish
- **WHEN** a shutdown signal arrives while the snapshot producer's startup production is retrieving a retained height
- **THEN** the production stops at the height being retrieved and nym-api shuts down without spawning its other tasks

# nym-api-startup Specification

## Purpose

Defines how nym-api starts: it builds the caches every route, reward and selection reads (the mixnet contract, describe, node families and node annotation caches) before it spawns any other task or serves HTTP, retries a failed first build every 30 seconds, and never serves a placeholder from a cache it has not built, so that a restart or fresh install never publishes, rewards or selects from data that does not exist yet.
## Requirements
### Requirement: Startup builds its caches before spawning tasks or serving HTTP

nym-api SHALL start the refreshers of its startup caches (the mixnet contract cache, the describe cache, the node families cache and the node annotation cache), with every attested contract's snapshot producer running its startup production beside them, and SHALL wait until each of those four caches holds a value before spawning any other task (the performance contract window, the DKG controller, the ecash background cleaner, the network monitor and its uptime updater, the epoch operations, key rotation and the signers cache refresher) and before serving HTTP. A value loaded from a persisted file still within its age limit SHALL count. While waiting, nym-api MUST log the caches still pending, and the wait SHALL have no time limit: nothing correct can be served or rewarded before the caches exist. A shutdown signal during the producers' startup production SHALL end it early, so that startup stops at the wait.

The signers cache is exempt: its refresher probes this nym-api's own HTTP after a start delay, so it SHALL be spawned after the wait, letting that delay run from when HTTP comes up. The performance contract window on a nym-api with the switch off is exempt too, its load being best effort, as are caches filled on demand.

#### Scenario: A restart with fresh persisted caches starts at once
- **WHEN** nym-api restarts and every startup cache's persisted file is within its age limit
- **THEN** it spawns its tasks and serves HTTP without waiting for any refresh

#### Scenario: A fresh install serves nothing until its caches are built
- **WHEN** nym-api starts with no persisted caches
- **THEN** it serves no route and spawns no other task until the describe pass and the first annotation refresh have completed, logging the caches still pending

#### Scenario: Legacy rewarding never reads an unbuilt annotation cache
- **WHEN** an epoch ends while nym-api is still waiting for its startup caches, with `use_performance_contract_data` off
- **THEN** the epoch operations run only after the annotation cache is built, rather than paying every node zero and selecting from no candidates

#### Scenario: The signers cache does not report this nym-api as down
- **WHEN** a signer nym-api starts from a fresh install whose describe pass outlasts the signers cache's start delay
- **THEN** the signers cache's first probe of this nym-api runs after its HTTP is served

#### Scenario: A shutdown during the warm-ups is not held until they finish
- **WHEN** a shutdown signal arrives while a snapshot producer's startup production is retrieving its retained heights
- **THEN** the production stops at the height being retrieved and nym-api shuts down without spawning its other tasks

### Requirement: A failed first build is retried every 30 seconds

While a startup cache holds no value, its refresher SHALL retry a failed build every 30 seconds, and SHALL refresh at its normal interval once the cache holds one, a value loaded from a persisted file counting, so that a transient failure at startup costs seconds rather than a whole refresh interval. The node annotation refresher SHALL retry every 30 seconds until its first refresh succeeds, whether its sources, its timer or a refresh request triggered it.

#### Scenario: A transient failure delays startup by seconds
- **WHEN** the node families cache's first build fails once and its next attempt succeeds
- **THEN** the retry runs 30 seconds after the failure, not after the cache's 600-second interval, and startup continues once it succeeds

#### Scenario: The normal interval resumes after the first success
- **WHEN** a startup cache's first build has succeeded and a later refresh fails
- **THEN** the next attempt waits for the cache's normal refresh interval

#### Scenario: A persisted value is refreshed at the normal interval
- **WHEN** a startup cache was loaded from a persisted file within its age limit and its first refresh fails
- **THEN** the next attempt waits for the cache's normal refresh interval

#### Scenario: A failed requested refresh keeps the annotation retry
- **WHEN** the annotation cache was loaded from a persisted file, its first refresh has not succeeded, and a requested refresh fails
- **THEN** the annotation refresher still retries 30 seconds later

### Requirement: No cache serves a placeholder before its first build

The node annotation cache SHALL be uninitialised until it is loaded from a persisted file within its age limit or refreshed once, and MUST NOT start from an empty map, which every route would serve as performance 0. Its refresher SHALL refresh as soon as the caches it reads hold values, a value loaded from a persisted file counting: in legacy mode the mixnet contract and describe caches, in contract mode the mixnet contract cache and the performance snapshots. Its first refresh MUST NOT wait for its rate limit.

#### Scenario: A stale annotation file is not served as empty
- **WHEN** nym-api starts with a persisted annotation file older than its age limit
- **THEN** the annotation cache stays uninitialised until the first refresh, and no route serves performance 0 from it

#### Scenario: The first annotation refresh uses persisted describe data
- **WHEN** the describe cache was loaded from a persisted file within its age limit, in legacy mode
- **THEN** the first annotation refresh runs as soon as the mixnet contract cache holds a value, without waiting for a new describe pass

#### Scenario: Contract mode needs no describe data
- **WHEN** `use_performance_contract_data` is on
- **THEN** the first annotation refresh waits only for the mixnet contract cache and the performance snapshots


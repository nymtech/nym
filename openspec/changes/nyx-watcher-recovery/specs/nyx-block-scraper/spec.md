## ADDED Requirements

### Requirement: Blocks are processed in strict height order and a failed block holds back every block behind it

The block processor shared by the persisting scraper and the watcher SHALL hand each block to storage and to the registered modules in strictly increasing height order, one height at a time, with no height left out. A block that arrives ahead of the next expected height MUST be held in a backlog until every height before it has been processed. A block whose processing fails at any step (obtaining its details, persisting it, or a module returning an error) MUST NOT be recorded as processed, and NO block of a greater height may be processed until it has succeeded. This holds whether the block arrived as the next expected one or was taken from the backlog.

A failed or missing block MUST be requested again by height. The processor checks every 30 seconds, and when nothing was processed for 15 seconds it requests every height between the last processed one and the lowest height in the backlog, or up to the current chain height when the backlog is empty.

#### Scenario: A block that fails in the backlog stops the ones behind it
- **WHEN** blocks 101, 102 and 103 are in hand, 101 is processed, and processing 102 fails
- **THEN** 103 is not processed and the last processed height stays 101

#### Scenario: A failed block is fetched again and the backlog continues after it
- **WHEN** block 102 has failed and block 103 waits in the backlog
- **THEN** the next missing-block check requests block 102 by height
- **AND** once 102 is processed, 103 is processed after it

#### Scenario: A silent subscription is covered by fetching
- **WHEN** no block arrives over the subscription and the backlog is empty
- **THEN** the processor obtains the current chain height and requests every height after the last processed one up to it

### Requirement: A persisting scraper resumes from the height its storage recorded

A scraper built over a storage backend SHALL take the last processed height recorded in that storage as its own when it starts. When that height is below the current chain height it MUST request every block between them at startup, except that with a pruning strategy in force it MUST NOT request blocks older than the ones that strategy would keep. A scraper whose storage records no height MUST start from its configured starting height when one is set, and otherwise from the first block it observes.

#### Scenario: Blocks produced while the scraper was down are fetched
- **WHEN** a scraper that stopped after block 1000 is started again while the chain is at height 1500
- **THEN** it requests blocks 1001 through 1500 and processes them in order

#### Scenario: A first start has nothing to resume
- **WHEN** a scraper starts over an empty storage with no starting height configured
- **THEN** the first block it observes becomes its starting point

### Requirement: A watcher starts at the block after a height its caller supplies

The watcher, which persists nothing and only feeds its modules, MAY be given the height that its modules' state already reflects. When given one, it MUST treat the block after that height as the next one to process and MUST NOT take the first block it observes as its starting point; blocks between the supplied height and the first one the subscription delivers MUST be fetched by height like any other missing block. When given none, which suits a caller with no loaded state, the first block it observes SHALL become its starting point.

#### Scenario: Blocks produced between the state load and the first delivered block reach the modules
- **WHEN** a watcher is started after height 500 and the first block its subscription delivers is 503
- **THEN** 503 waits in the backlog while 501 and 502 are fetched by height
- **AND** the modules are handed 501, 502 and 503 in that order

#### Scenario: A watcher given no height starts at the first block it observes
- **WHEN** a watcher is started with no height and the first block its subscription delivers is 503
- **THEN** the modules are handed 503 first, and no earlier block is fetched

### Requirement: A watcher that cannot advance gives up, and a persisting scraper never does

A watcher SHALL consider itself stalled when BOTH of the following hold: a block newer than the last processed one is known to exist, and no block was processed for 2 minutes. A newer block is known to exist when the backlog is not empty, or when the current chain height (see the next requirement) is above the last processed height. A stalled watcher MUST cancel itself, which stops its subscription, its block fetching and its processing, so that whoever owns it can observe the end and start a new one. When no newer block is known, which covers a halted chain as well as a period in which no endpoint answers, the watcher MUST keep waiting however long that lasts.

A persisting scraper MUST NOT give up on a stall. It keeps retrying the block it is held on, because moving past it would leave a hole in stored history.

Giving up is not the only way a watcher ends: as before, it also ends when its websocket subscription has exhausted its reconnection attempts.

#### Scenario: A watcher held behind an unobtainable block gives up
- **WHEN** block 102 waits in the backlog, block 101 cannot be processed, and 2 minutes pass without any block being processed
- **THEN** the watcher cancels itself

#### Scenario: A halted chain does not end the watcher
- **WHEN** no block was processed for 2 minutes, the backlog is empty, and no endpoint reports a height above the last processed one
- **THEN** the watcher keeps running

#### Scenario: Unreachable endpoints do not end the watcher
- **WHEN** no block was processed for 2 minutes, the backlog is empty, and no endpoint answers the height query
- **THEN** the watcher keeps running

#### Scenario: A persisting scraper keeps waiting
- **WHEN** a persisting scraper is held behind a block that keeps failing for longer than 2 minutes
- **THEN** it does not cancel itself and goes on retrying that block

### Requirement: The current chain height is the highest any configured endpoint reports

When the engine needs the current chain height it SHALL ask every configured RPC endpoint and use the highest height reported. An endpoint that fails to answer MUST be ignored as long as at least one other answers, and the query MUST fail only when none does. Every other query keeps using the endpoints in order of preference, moving to a later one only once an earlier one has failed.

#### Scenario: A backend that stopped advancing does not hide newer blocks
- **WHEN** the first endpoint reports height 900 and a fallback endpoint reports height 950
- **THEN** the current chain height is 950

#### Scenario: One answer is enough
- **WHEN** the first endpoint is unreachable and a fallback endpoint reports height 950
- **THEN** the current chain height is 950

#### Scenario: A single endpoint has nothing to be compared against
- **WHEN** only one endpoint is configured
- **THEN** its answer is the current chain height, and an endpoint that has stopped advancing is not detected

### Requirement: A cancelled watcher stops fetching at once and finishes only the block in hand

When a watcher is cancelled, by itself or by its owner, its block fetching MUST stop without completing the range it was working on, and its processor MUST finish at most the block it is already processing. Stopping a watcher MUST wait until all of its tasks have ended, so that no block is handed to a module after the stop has returned.

#### Scenario: Stopping mid-range does not wait for the range
- **WHEN** the owner stops a watcher while a range of 30 blocks is being fetched
- **THEN** no further block of that range is requested
- **AND** the block being processed, if any, completes, and no module is called after the stop has returned

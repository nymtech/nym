## Context

`common/nyxd-scraper-shared` follows the nyx chain block by block. A `ChainSubscriber` receives new blocks over a websocket, a `BlockRequester` fetches blocks by height, and a `BlockProcessor` hands them to storage and modules in height order, holding out-of-order blocks in a backlog. The crate has two front-ends: `NyxdScraper`, which persists every block (used by `nyx-chain-watcher`, `nym-validator-rewarder` and `nym-data-observatory`), and `NyxdWatcher`, which persists nothing and only calls its modules (used only by nym-node).

nym-node uses the watcher to keep its authorised network-monitor agent set current. That set is held in three `ArcSwap`-backed structures behind `AuthorisedAgentsView`: a routing set, a noise-key map shared with the topology refresher, and the announced client identities.

What goes wrong today:

- **A block that fails as the next expected one is retried forever.** Everything behind it accumulates in the backlog, which has no bound, and the modules see nothing more.
- **A block that fails while the backlog is drained is skipped.** The loop logs the error and carries on, and the next success moves the processed height past the failed block.
- **A watcher that ends is never started again.** The subscriber gives up after its reconnection attempts and cancels the watcher's token. nym-node drops the watcher handle at startup, so nothing observes this.
- **Startup has a hole.** nym-node loads the agent set, then builds its network refresher, then starts the watcher, which adopts the first block the websocket delivers. Changes made on chain in between are never seen.

The first part of this work (PR #7425) removed the common trigger, per-tx lookups racing the tx indexer, and added an ordered list of RPC endpoints. This change makes the watcher recover from whatever still stalls it.

Constraints: the processor is shared with the persisting scrapers, which must never skip a block. The default mainnet RPC is a sticky load balancer in front of several backends, so two queries to one URL can be answered by backends at different heights, and a backend can keep answering after it has stopped advancing.

## Goals / Non-Goals

**Goals:**

- Memory stays bounded when the watcher cannot advance.
- The watcher gets back to delivering blocks without a node restart.
- A consumer of the watcher is never left with a stale view without that being recovered or logged.
- The persisting scrapers stop skipping blocks.

**Non-Goals:**

- Guaranteed delivery of every block to a watcher module. Consumers that must see every transaction use the persisting scraper.
- Bounding the backlog of a persisting scraper that is held on a block that never succeeds.
- A periodic poll of the contract.
- Restarting in place of a long backfill. Any gap keeps being fetched by height.
- New configuration fields, a new config version, or any change to the websocket URL in existing configs.
- Changing the `BlockModule`, `TxModule` or `MsgModule` traits.
- Detecting a stalled backend on a node configured with a single endpoint.

## Decisions

### 1. Consumers rebuild from their source of truth; recovery is a restart

A watcher that cannot advance gives up and exits. Its owner loads its state again from the source of truth and starts a new watcher. The watcher itself has no notion of rebuilding.

Alternatives considered:

- **Deliver every block eventually.** Rejected: a block that an endpoint cannot serve may never arrive, and waiting for it is the bug being fixed.
- **A rebuild notice delivered in line with blocks, or a rebuild method on the module traits.** Rejected: it changes what a module is, and with several modules in one watcher each would rebuild at a different height unless the watcher coordinated them. A restart gives every consumer the same height for free.

### 2. Boot and recovery are one height-first sequence

Obtain the current height N, load every consumer's state pinned at N, start the watcher with N as its last processed height. The existing missing-block fetch fills in from N + 1. This closes the startup hole and makes recovery the same code path as boot.

The start height is optional on `WatcherConfig`. Without one the watcher starts at the first block it observes, as before, which suits a caller with no loaded state such as the crate's example. nym-node always passes one.

Alternative considered: subscribe first and buffer blocks while state loads. Rejected: it needs buffering and replay logic that the processor's backlog and fetch-by-height already provide once it knows its start height.

### 3. The watcher gives up when a newer block is known and nothing was processed for 2 minutes

"A newer block is known" means the backlog is not empty, or the chain height is above the last processed one. The rule is on for the watcher and off for the persisting scraper. It is checked on the processor's existing 30 second tick, so a watcher gives up between 2 and 2.5 minutes after its last processed block.

The "newer block is known" half is what keeps a halted chain, or a period with every endpoint down, from ending every watcher in the network: with no evidence of a newer block there is nothing a restart could fetch.

Alternatives considered:

- **A retry count on the failing block.** Rejected: retries are driven by a 30 second tick, so a count is a time limit with an extra variable.
- **Recognising chain-upgrade messages** to tolerate a planned halt. Rejected: the newer-block condition already covers a halt without parsing governance messages.

### 4. The current height is the highest any configured endpoint reports

`current_block_height()` asks every endpoint and keeps the highest answer. With "first endpoint that answers", a backend that responds but has stopped advancing, serving both the websocket and the first RPC endpoint, leaves the watcher convinced there is nothing newer. Every caller is inside the crate, and the persisting scrapers build their client with one URL, so nothing changes for them. The query only runs when nothing was processed for 15 seconds and the backlog is empty. Cost: the endpoints are asked in turn, so a hanging fallback adds up to its 30 second request timeout to that check.

### 5. A failed block stops the drain, and is not put back in the backlog

The drain loop returns on a failed block, as the direct path already does. The failed block is dropped from the backlog on purpose: the missing-block check requests `last + 1..lowest queued height`, which then includes it. Left at the head of the backlog it would make that range empty and never be retried.

This applies to the persisting scrapers too. Skipping a block there was a bug, not a feature.

### 6. nym-node supervises the watcher and never restarts itself

A `NyxWatcherSupervisor` task owns the watcher handle. It waits for the watcher's cancellation token, stops the watcher (which waits for its tasks), waits, and runs the height-first sequence again.

The wait starts at 30 seconds, doubles after every attempt up to 15 minutes, and returns to 30 seconds once a watcher has stayed up for 15 minutes. Each wait is drawn at random between half of its value and the value itself. The concern this answers is every node in the network noticing the same chain or RPC failure at the same time and then reloading in step.

The agent set a node holds stays in use until a load succeeds. The loaded set is published before the watcher start is attempted, so with RPC up and the websocket down the node still refreshes its set on every attempt, at worst every 15 minutes.

`NyxdWatcherBuilder::with_custom_shutdown` is removed: it stores a token nothing reads. The supervisor stops the watcher on node shutdown.

### 7. The reload tries endpoints in order and takes height and state from the same one

The supervisor holds one query client per URL of `nyxd_urls` followed by `nyxd_fallback_urls`. An attempt asks an endpoint for the height and then for the agent set pinned at that height, and moves on only when either fails. A query pinned to a height the answering backend has not reached fails with "cannot query with height in the future", which is why the two must not be split across endpoints and why a failure moves on rather than retrying in place.

The node's signing client is not used: it picks one `nyxd_urls` entry at random for the node's lifetime and knows nothing of the fallbacks.

The agents query gains a height-pinned variant in the validator client, in the same shape as the directory and geolocation ones.

### 8. A reload publishes each derived structure with a single swap

All three structures are `ArcSwap`-backed, so the reload builds each new one aside and stores it once. A reader never sees an empty or half-built set. It is three swaps, not one, which is what a single authorisation event already does. The new noise-key map is the current nym-node entries plus the loaded agents, built under the map's update permit so the topology refresher cannot interleave.

Alternatives considered: adding the new agents and then removing the stale ones (a window in which revoked agents are still accepted, and more code), or clearing and re-adding (a window in which every agent is rejected).

### 9. Boot keeps today's rule, with an opt-out flag

Without a working watcher the node refuses to start, as today. Pinning adds a new way for the first load to fail (a lagging backend), so boot makes 3 attempts 10 seconds apart. Each attempt reads a fresh height, so the pause does not wait out a lagging backend; it gives a briefly unavailable endpoint time to come back.

`--allow-nyx-watcher-startup-failure` (`NYMNODE_ALLOW_NYX_WATCHER_STARTUP_FAILURE`) makes a single attempt and, on failure, starts the node with whatever agent set that attempt published (none when the load failed, the loaded set when only the watcher could not start) and lets the supervisor keep trying. It is a run-only flag like `--deny-init` and `--local`, not a config field, and it is visible in `--help`: it is an operator's escape hatch, not a tuning knob.

### 10. A contract message the node cannot parse stays logged and skipped

`NetworkMonitorAgentsModule` logs and ignores a network-monitors `ExecuteMsg` it cannot deserialise. Failing the block instead would force a give-up and a reload, which sounds right, but on an old binary the reload's query can fail on the same schema change, and then no watcher runs at all. Keeping today's behaviour loses one event; the alternative can lose all of them.

A failure to decode the `MsgExecuteContract` envelope itself still fails the block. Under this design that leads to a give-up and a reload each time, so such a node degrades to reloading its set at the supervisor's capped wait.

### 11. Timings are constants

The stall limit (2 minutes) lives beside the processor's existing constants. The supervisor's first wait (30 seconds), cap (15 minutes), boot attempts (3) and boot pause (10 seconds) live in the supervisor. None is a config field. They can become fields later without touching the design.

### 12. Naming

New names say "nyx": the supervisor, its flag, its metric, the new capability. "nyxd" is the daemon, and its use in chain-related names is legacy. Existing names (`NyxdWatcher`, the `nyxd_watcher` module, `nyxd_urls`) are not renamed by this change.

### 13. What gets a kept test

One test per behaviour, with no live sockets:

- a failed block stops the drain;
- a processor given a start height waits for the block after it;
- a stalled processor gives up (paused time);
- a processor with no newer block in sight keeps waiting;
- a processor not told to give up keeps waiting;
- replacing the agent set drops what is gone, adds what is new, and keeps nym-node noise entries;
- the supervisor's wait doubles, stops at the cap, and starts over after a healthy run.

Checked by hand instead, because a test would need chain endpoints and a websocket or would test a dependency: the supervisor loop and both boot paths, the highest-height query across endpoints, the pinned agents query, and the flag's parsing.

## Risks / Trade-offs

- **A persisting scraper held on a block that never succeeds grows its backlog without bound.** → Unchanged from today for failures outside the drain; now also true for failures inside it. Accepted: skipping is worse for a scraper. A held scraper is not cancelled, so the validator rewarder keeps running: it pays the epoch in which the hold began from the blocks stored before it, and every later epoch fails for lack of blocks and is not replayed.
- **A watcher task that panics does not cancel the watcher's token**, so the supervisor would not notice. → In nym-node this does not arise: its panic hook cancels the node's root token on any panic, which stops every node task, the supervisor included, as a panic in any other task already does. Not addressed here.
- **A node with one configured endpoint cannot detect a backend that stopped advancing.** → The v15 config migration of part one gives default mainnet configs a fallback. Operators with a custom single endpoint keep today's behaviour.
- **A reload from an endpoint behind the node's current state can bring back a revoked agent.** The reload takes the first endpoint that answers, at whatever height it reports. If that backend is behind a revocation the previous watcher already applied, the reloaded set has the agent again until the new watcher fetches that block by height, about 30 seconds; while no watcher can be started, until a reload lands on an endpoint past the revocation. → Accepted: refusing loads below the last loaded height would still miss whatever the watcher applied after its start.
- **Modules can be about 30 seconds behind right after a watcher start**, because the blocks produced during the load are fetched by the existing 30 second check. → Accepted; today the same blocks are lost outright.
- **A reload jumps over the blocks between the stall and the new height.** → That is the design: the loaded state already reflects them. It is why this mechanism suits a consumer that mirrors contract state and not one that must act on every transaction.
- **A failing boot takes about 20 seconds longer to fail** (two 10 second pauses between its three attempts). → Accepted for riding out a lagging backend.
- **With the flag set and no reachable endpoint, a node rejects every monitor agent** and so fails liveness probes until its first load. → The flag's help text and the error log say so.
- **The restart counter is opportunistic.** If wiring it needs more than a few lines it is dropped, and the sentence requiring it is removed from the spec delta in the same task.

## Migration Plan

No data or configuration migration. The change ships with a node release. Rolling back is a binary downgrade: no persisted state is introduced, and the flag is simply unknown to an older binary.

The persisting scraper users pick up the drain fix and the resume fix when they are next built. On the first restart after that, a scraper resumes from its stored height and fetches the blocks produced while it was down; if its endpoint no longer holds them, the scraper stops with an error naming the first missing block and its program exits. Blocks skipped by earlier restarts stay missing; a separate backfill is needed to recover those.

Moving operators' websocket URL back to the load-balanced default is a later rollout decision that this change makes safe but does not make.

## Open Questions

None.

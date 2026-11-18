## Why

A nym-node learns which network-monitor agents may probe it from a chain watcher, and that watcher can stop advancing with nothing noticing: a block it cannot process is retried forever while later blocks pile up in memory, a block that fails while the backlog is drained is silently skipped, and a watcher that gives up on its websocket is never started again. The node then keeps an agent set that drifts from the contract until an operator restarts the process, which is what the NMv3 stale-agent incident looked like from the outside. The same block processor serves the persisting scrapers, where a skipped block is a permanent hole in stored chain history.

## What Changes

- **A failed block holds back every block behind it**, in the watcher and in the persisting scrapers. Today a block that fails while the backlog is drained is logged and passed over, and the next success moves the processed height past it.
- **A persisting scraper resumes from the height its storage recorded.** Since #6561 the stored height was read and dropped, so a restarted scraper started from the first block it happened to observe. Already fixed on this branch (`9b8e3dc2cf`); recorded here so the spec matches the code.
- **The watcher is told where to start.** Its caller supplies the height its modules' state already reflects, and processing starts at the block after it. Today the watcher adopts whichever block the websocket delivers first, which leaves a hole between loading state and the first block.
- **The watcher gives up when it cannot advance**: a newer block is known to exist and nothing was processed for 2 minutes. It then stops itself so its owner can start afresh. A persisting scraper never gives up, because it must not skip a block.
- **"Is there a newer block" asks every configured RPC endpoint** and uses the highest answer, so a backend that responds but has stopped advancing does not hide the stall.
- **nym-node supervises its watcher.** When the watcher ends, a supervising task waits, reloads the whole authorised-agent set from the contract pinned at one height, publishes it, and starts a new watcher from that height. The wait starts at 30 seconds, doubles up to 15 minutes and is randomised, so a chain-wide stall does not send every node to the same endpoints at once. The node process never restarts for this.
- **Boot runs the same height-first sequence**, which closes the hole between the initial load and the first websocket block.
- **A reload replaces the agent set as a whole**: each of the three derived structures is built aside and published in a single swap, so readers never see an empty or half-built set.
- **A new run-only flag, `--allow-nyx-watcher-startup-failure`** (`NYMNODE_ALLOW_NYX_WATCHER_STARTUP_FAILURE`), lets a node start when the watcher cannot be started. Without it the node still refuses to start, as today, though it now makes three attempts first.
- **Restart attempts are logged and counted** in the node's prometheus metrics.

No configuration file changes, no new config version, and the websocket URL in existing configs is left alone.

## Capabilities

### New Capabilities

- `nyx-block-scraper`: the shared engine in `common/nyxd-scraper-shared` that follows the nyx chain block by block, as the persisting scraper and as the non-persisting watcher: processing order, what happens to a block that fails, where processing starts and resumes, when a watcher gives up, and how the current chain height is determined. Nothing specifies this engine today, although three services and nym-node depend on it.

### Modified Capabilities

- `nym-network-monitor`: the requirement on how network nodes learn the authorised-agent set no longer says the set is loaded once at startup and only re-synced by a node restart. It gains the pinned-height load, the reload on watcher restart, the whole-set replacement, the startup rule with its opt-out flag, and the supervised restart with its growing wait.

## Impact

- **`common/nyxd-scraper-shared`**: block processor (drain, start height, give-up rule), RPC client (current height across endpoints), block requester (stops when cancelled), watcher builder (**BREAKING** for its one in-repo caller: `WatcherConfig` gains a required start height and `with_custom_shutdown` is removed).
- **Persisting scraper users** (`nyx-chain-watcher`, `nym-validator-rewarder`, `nym-data-observatory`): no API change. Behaviour changes in two ways: a block that fails is retried until it succeeds where it could previously be skipped, and a restart resumes from the stored height. A scraper held by a block that never succeeds grows its backlog without bound, as it already does when the failure happens outside the drain.
- **`common/client-libs/validator-client`**: a height-pinned variant of the paged network-monitor agents query.
- **`nym-node`**: a supervisor task owning the watcher, startup wiring, the new CLI flag and env variable, a replace-everything method on the authorised-agents view, one metric.
- **`common/authorised-network-monitors`**, **`common/nymnoise`** (read only), **`nym-node` routing filter**: whole-structure replacement of the announced identities and of the routing set.
- **Operators**: a node that today would fail to start on a transient RPC failure now retries for about ten seconds first. A node started with the new flag and no reachable chain endpoint rejects every monitor agent until its first successful load.
- **No contract, wire format or config changes.**

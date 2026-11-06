# nym-epoch-driver

Progresses the mixnet epoch. Once an epoch is over it produces a network view from the verified directory, begins the transition, rewards the rewarded set by the performance contract's figures, reconciles the pending events, then selects and assigns the next rewarded set. It signs as the mixnet contract's rewarding validator, keeps no database and calls no nym-api. One driver runs per network.

## Setup

```sh
nym-epoch-driver init \
  --mnemonic "<rewarding validator mnemonic>" \
  --metrics-bearer-token "<token>"
```

`init` writes `~/.nym/nym-epoch-driver/config/config.toml`. The mnemonic and the metrics token are required. The nyxd URL defaults to the network environment's endpoint, mainnet unless `--config-env-file` names another network; `--nyxd-url` chooses another. The contract addresses always come from the network environment.

```sh
nym-epoch-driver run
```

Before its loop starts, `run` checks, in order, that its account is the mixnet contract's rewarding validator, that the network details carry the directory and performance contract addresses, that the performance contract has received a submission (rewarding and selection have no other figure), and that a network view can be produced. Any failure stops the start.

## Configuration

The file's values are overridden by environment variables, and those by flags:

| File | Flag | Environment |
|---|---|---|
| `nyxd_url` | `--nyxd-url` | `NYM_EPOCH_DRIVER_NYXD_URL` |
| `mnemonic` | `--mnemonic` (`init` only) | `NYM_EPOCH_DRIVER_MNEMONIC` |
| `[http] bind_address` (`0.0.0.0:8080`) | `--http-bind-address` | `NYM_EPOCH_DRIVER_HTTP_BIND_ADDRESS` |
| `[metrics] bearer_token` | `--metrics-bearer-token` | `NYM_EPOCH_DRIVER_METRICS_BEARER_TOKEN` |

The `[debug]` section holds tuning knobs, overridable by environment and hidden from `--help`; a zero period is refused at startup:

| Knob | Default | Environment |
|---|---|---|
| `settle_lag`: blocks a directory snapshot boundary must lie below the tip before the network view is produced at it | `10` | `NYM_EPOCH_DRIVER_SETTLE_LAG` |
| `epoch_check_interval`: how often the current interval is read while more than this is left of the epoch | `2m` | `NYM_EPOCH_DRIVER_EPOCH_CHECK_INTERVAL` |
| `epoch_operations_retry`: how long after a failed epoch operation it is retried | `30s` | `NYM_EPOCH_DRIVER_EPOCH_OPERATIONS_RETRY` |

## Metrics

`GET /v1/metrics/prometheus` serves the metrics to requests carrying `Authorization: Bearer <token>` and rejects any other. The driver refuses to start with an empty token. Each series appears at its first report:

- `nym_epoch_driver_current_epoch_end_timestamp_seconds`: the end of the epoch in progress as the loop last read it. Alert on `time() - x` above a threshold: an ended epoch that does not advance, whether its operations keep failing or the loop hangs, shows as this falling ever further behind.
- `nym_epoch_driver_epoch_operations_duration_seconds`: how long the attempt that last completed an epoch's operations took.
- `nym_epoch_driver_pending_epoch_events` and `nym_epoch_driver_pending_interval_events`: the events pending as the last reconciliation began.

## Cutover from nym-api's rewarding

The driver and a rewarding nym-api must never run together: two signers on one account clash on sequence numbers, and an old nym-api finding its own account advancing the epoch resumes the transition beside the driver.

1. Copy the SQLite files of every nym-api and of the network monitor orchestrator before their new binaries first start. The new versions drop tables in forward-only migrations, so a rollback needs the copies.
2. Upgrade the orchestrator before, or together with, nym-api: an orchestrator still checking nym-api's known-monitors route does not start against a nym-api without it.
3. On the rewarding host, run `nym-epoch-driver init` with the rewarding nym-api's mnemonic and nyxd URL and a metrics token.
4. Remove `--enable-monitor`, `--enable-rewarding` and `--monitor-credentials-mode` from the rewarding nym-api's service unit, since `run` no longer accepts them, then upgrade that nym-api. The new binary does not reward, so this stops nym-api's rewarding. Its config's removed sections are ignored.
5. Start the driver.
6. Upgrade every other nym-api, removing the same three flags from their service units if they pass them.

Rollback: stop the driver, then restore the previous nym-api binary, its config and its database copy on the rewarding host. The orchestrator rolls back on its own, with its own copy.

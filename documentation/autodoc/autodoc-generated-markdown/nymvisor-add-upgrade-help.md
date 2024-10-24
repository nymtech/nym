```sh
Queues up another upgrade for the associated daemon

Usage: nymvisor add-upgrade [OPTIONS] --upgrade-name <UPGRADE_NAME> <DAEMON_BINARY>

Arguments:
  <DAEMON_BINARY>  Path to the daemon's upgrade executable

Options:
      --upgrade-name <UPGRADE_NAME>    Name of this upgrade
      --force                          Overwrite existing upgrade binary / upgrade-info.json file
      --add-binary                     Indicate that this command should only add binary to an *existing* scheduled upgrade
      --now                            Force the upgrade to happen immediately
      --publish-date <PUBLISH_DATE>    Specifies the publish date metadata field of this upgrade. If unset, the current time will be used
      --upgrade-time <UPGRADE_TIME>    Specifies the time at which the provided upgrade will be performed (RFC3339 formatted). If left unset, the upgrade will be performed in 15min
      --upgrade-delay <UPGRADE_DELAY>  Specifies delay until the provided upgrade is going to get performed. If let unset, the upgrade will be performed in 15min
  -o, --output <OUTPUT>                [default: text] [possible values: text, json]
  -h, --help                           Print help
```

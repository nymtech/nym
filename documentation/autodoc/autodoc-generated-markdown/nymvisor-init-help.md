```sh
Initialise a nymvisor instance with persistent Config.toml file

Usage: nymvisor init [OPTIONS] <DAEMON_BINARY>

Arguments:
  <DAEMON_BINARY>  Path to the daemon's executable

Options:
      --id <ID>
          ID specifies the human readable ID of this particular nymvisor instance. Can be overridden with $NYMVISOR_ID environmental variable
      --upstream-base-upgrade-url <UPSTREAM_BASE_UPGRADE_URL>
          Sets the base url of the upstream source for obtaining upgrade information for the deaemon. It will be used fo constructing the full url, i.e. $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL/$DAEMON_NAME/upgrade-info.json Can be overridden with $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL environmental variable
      --upstream-polling-rate <UPSTREAM_POLLING_RATE>
          Specifies the rate of polling the upstream url for upgrade information. default: 1h Can be overridden with $NYMVISOR_UPSTREAM_POLLING_RATE
      --disable-nymvisor-logs
          If enabled, this will disable `nymvisor` logs (but not the underlying process) Can be overridden with $NYMVISOR_DISABLE_LOGS environmental variable
      --upgrade-data-directory <UPGRADE_DATA_DIRECTORY>
          Set custom directory for upgrade data - binaries and upgrade plans. If not set, the global nymvisors' data directory will be used instead. Can be overridden with $NYMVISOR_UPGRADE_DATA_DIRECTORY environmental variable
      --daemon-home <DAEMON_HOME>
          The location where the `nymvisor/` directory is kept that contains the auxiliary files associated with the underlying daemon, such as any backups or current version information. (e.g. $HOME/.nym/nym-api/my-nym-api, $HOME/.nym/mixnodes/my-mixnode, etc.). Can be overridden with $DAEMON_HOME environmental variable
      --daemon-absolute-upstream-upgrade-url <DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL>
          Override url to the upstream source for upgrade plans for this daeamon. The Url has to point to an endpoint containing a valid [`UpgradeInfo`] json. Note: if set this takes precedence over `upstream_base_upgrade_url` Can be overridden with $DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL environmental variable
      --allow-download-upgrade-binaries <ALLOW_DOWNLOAD_UPGRADE_BINARIES>
          If set to true, this will enable auto-downloading of new binaries using the url provided in the `upgrade-info.json` Can be overridden with $DAEMON_ALLOW_BINARIES_DOWNLOAD environmental variable [possible values: true, false]
      --enforce-download-checksum <ENFORCE_DOWNLOAD_CHECKSUM>
          If enabled nymvisor will require that a checksum is provided in the upgrade plan for the binary to be downloaded. If disabled, nymvisor will not require a checksum to be provided, but still check the checksum if one is provided. Can be overridden with $DAEMON_ENFORCE_DOWNLOAD_CHECKSUM environmental variable [possible values: true, false]
      --restart-daemon-after-upgrade <RESTART_DAEMON_AFTER_UPGRADE>
          If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags (but with the new binary) after a successful upgrade. Otherwise (if disabled), nymvisor will stop running after an upgrade and will require the system administrator to manually restart it. Note restart is only after the upgrade and does not auto-restart the subprocess after an error occurs. Can be overridden with $DAEMON_RESTART_AFTER_UPGRADE environmental variable [possible values: true, false]
      --restart-daemon-on-failure
          If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags after it has crashed Can be overridden with $DAEMON_RESTART_ON_FAILURE environmental variable
      --on-failure-daemon-restart-delay <ON_FAILURE_DAEMON_RESTART_DELAY>
          If `restart_on_failure` is enabled, the following value defines the amount of time `nymvisor` shall wait before restarting the subprocess. Can be overridden with $DAEMON_FAILURE_RESTART_DELAY environmental variable
      --max-daemon-startup-failures <MAX_DAEMON_STARTUP_FAILURES>
          Defines the maximum number of startup failures the subprocess can experience in a quick succession before no further restarts will be attempted and `nymvisor` will exit. Can be overridden with $DAEMON_MAX_STARTUP_FAILURES environmental variable
      --startup-period-duration <STARTUP_PERIOD_DURATION>
          Defines the length of time during which the subprocess is still considered to be in the startup phase when its failures are going to be considered in `max_startup_failures`. Can be overridden with $DAEMON_STARTUP_PERIOD_DURATION environmental variable
      --daemon-shutdown-grace-period <DAEMON_SHUTDOWN_GRACE_PERIOD>
          Specifies the amount of time `nymvisor` is willing to wait for the subprocess to undergo graceful shutdown after receiving an interrupt (for either an upgrade or shutdown of the `nymvisor` itself) Once the time passes, a kill signal is going to be sent instead. Can be overridden with $DAEMON_SHUTDOWN_GRACE_PERIOD environmental variable
      --daemon-backup-data-directory <DAEMON_BACKUP_DATA_DIRECTORY>
          Set custom backup directory for daemon data. If not set, the daemon's home directory will be used instead. Can be overridden with $DAEMON_BACKUP_DATA_DIRECTORY environmental variable
      --unsafe-skip-backup
          If enabled, `nymvisor` will perform upgrades directly without performing any backups. default: false Can be overridden with $DAEMON_UNSAFE_SKIP_BACKUP environmental variable
  -o, --output <OUTPUT>
          [default: text] [possible values: text, json]
  -h, --help
          Print help
```

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base nymvisor config options #####

[nymvisor]
# ID specifies the human readable ID of this particular nymvisor instance.
# Can be overridden with $NYMVISOR_ID environmental variable.
id = '{{ nymvisor.id }}'

##### further optional configuration nymvisor options #####

# Sets the base url of the upstream source for obtaining upgrade information for the deaemon.
# default: "https://nymtech.net/.wellknown/"
# It will be used fo constructing the full url, i.e. $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL/$DAEMON_NAME/upgrade-info.json
# Can be overridden with $NYMVISOR_UPSTREAM_BASE_UPGRADE_URL environmental variable.
upstream_base_upgrade_url = '{{ nymvisor.upstream_base_upgrade_url }}'

# Specifies the rate of polling the upstream url for upgrade information.
# Can be overridden with $NYMVISOR_UPSTREAM_POLLING_RATE
upstream_polling_rate = '{{ nymvisor.upstream_polling_rate }}'

# If set to true, this will disable `nymvisor` logs (but not the underlying process)
# default: false
# Can be overridden with $NYMVISOR_DISABLE_LOGS environmental variable.
disable_logs = {{ nymvisor.disable_logs }}

# Set custom directory for upgrade data - binaries and upgrade plans.
# If not set, the global nymvisors' data directory will be used instead.
# Can be overridden with $NYMVISOR_UPGRADE_DATA_DIRECTORY environmental variable.
upgrade_data_directory = '{{ nymvisor.upgrade_data_directory }}'

##### main base daemon config options #####

[daemon]

# The name of the managed binary itself (e.g. nym-api, nym-mixnode, nym-gateway, etc.)
# Can be overridden with $DAEMON_NAME environmental variable.
name = '{{ daemon.name }}'

# The location where the `nymvisor/` directory is kept that contains the auxiliary files associated
# with the underlying daemon, such as any backups or current version information.
# (e.g. $HOME/.nym/nym-api/my-nym-api, $HOME/.nym/mixnodes/my-mixnode, etc.).
# Can be overridden with $DAEMON_HOME environmental variable.
home = '{{ daemon.home }}'

##### further optional configuration daemon options #####

# Override url to the upstream source for upgrade plans for this daeamon.
# The Url has to point to an endpoint containing a valid [`UpgradeInfo`] json.
# Note: if set this takes precedence over .nymvisor.debug.upstream_base_upgrade_url
# default: None
# Can be overridden with $DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL environmental variable.
absolute_upstream_upgrade_url = '{{ daemon.absolute_upstream_upgrade_url }}'

# If set to true, this will enable auto-downloading of new binaries using the url provided in the `upgrade-info.json`
# default: true
# Can be overridden with $DAEMON_ALLOW_BINARIES_DOWNLOAD environmental variable.
allow_binaries_download = {{ daemon.allow_binaries_download }}

# If enabled nymvisor will require that a checksum is provided in the upgrade plan for the binary to be downloaded.
# If disabled, nymvisor will not require a checksum to be provided, but still check the checksum if one is provided.
# default: true
# Can be overridden with $DAEMON_ENFORCE_DOWNLOAD_CHECKSUM environmental variable.
enforce_download_checksum = {{ daemon.enforce_download_checksum }}

# If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags (but with the new binary) after a successful upgrade.
# Otherwise (if disabled), nymvisor will stop running after an upgrade and will require the system administrator to manually restart it.
# Note restart is only after the upgrade and does not auto-restart the subprocess after an error occurs.
# default: true
# Can be overridden with $DAEMON_RESTART_AFTER_UPGRADE environmental variable.
restart_after_upgrade = {{ daemon.restart_after_upgrade }}

# If enabled, nymvisor will restart the subprocess with the same command-line arguments and flags after it has crashed
# default: false
# Can be overridden with $DAEMON_RESTART_ON_FAILURE environmental variable.
restart_on_failure = {{ daemon.restart_on_failure }}

# If `restart_on_failure` is enabled, the following value defines the amount of time `nymvisor` shall wait before
# restarting the subprocess.
# default: 10s
# Can be overridden with $DAEMON_FAILURE_RESTART_DELAY environmental variable.
# The default value is so relatively high as to prevent constant restart loops in case of some underlying issue.
failure_restart_delay = '{{ daemon.failure_restart_delay }}'

# Defines the maximum number of startup failures the subprocess can experience in a quick succession before
# no further restarts will be attempted and `nymvisor` will exit/
# default: 10
# Can be overridden with $DAEMON_MAX_STARTUP_FAILURES environmental variable.
max_startup_failures = {{ daemon.max_startup_failures }}

# Defines the length of time during which the subprocess is still considered to be in the startup phase
# when its failures are going to be considered in `max_startup_failures`.
# default: 120s
# Can be overridden with $DAEMON_STARTUP_PERIOD_DURATION environmental variable.
startup_period_duration = '{{ daemon.startup_period_duration }}'

# Specifies the amount of time `nymvisor` is willing to wait for the subprocess to undergo graceful shutdown after receiving an interrupt
# (for either an upgrade or shutdown of the `nymvisor` itself)
# Once the time passes, a kill signal is going to be sent instead.
# default: 10s
# Can be overridden with $DAEMON_SHUTDOWN_GRACE_PERIOD environmental variable.
shutdown_grace_period = '{{ daemon.shutdown_grace_period }}'

# Set custom backup directory for daemon data. If not set, the daemon's home directory will be used instead.
# Can be overridden with $DAEMON_BACKUP_DATA_DIRECTORY environmental variable.
backup_data_directory = '{{ daemon.backup_data_directory }}'

# If enabled, `nymvisor` will perform upgrades directly without performing any backups.
# default: false
# Can be overridden with $DAEMON_UNSAFE_SKIP_BACKUP environmental variable.
unsafe_skip_backup = {{ daemon.unsafe_skip_backup }}

"#;

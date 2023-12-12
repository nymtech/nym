# Automatic Node Upgrade: Nymvisor Setup and Usage

> The Nymvisor binary was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first. You can build just Nymvisor with `cargo build --release --bin nymvisor`. 

> Any syntax in `<>` brackets is a user's unique variable. Exchange with a corresponding name without the `<>` brackets.

## What is Nymvisor?
Nymvisor is a process manager for Nym binaries that monitors the Nym release information for any newly released binaries. If it detects any changes, Nymvisor can automatically download the binary, stop the current binary, switch from the old binary to the new one, and finally restart the underlying process with the new binary. 

In essence, it tries to mirror the behaviour of [Cosmovisor](https://github.com/cosmos/cosmos-sdk/tree/main/tools/cosmovisor), a tool used by Cosmos blockchain operators for managing/automating chain upgrades. Nymvisor, however, introduces some Nym-specific changes since, for example, upgrade information is obtained from our GitHub [releases page](https://github.com/nymtech/nym/releases) instead of (in the case of Cosmos blockchains) governance proposals. 

You can use Nymvisor to automate the upgrades of the following binaries:
* `nym-api`
* `nym-mixnode`
* `nym-gateway`
* `nym-network-requester`
* `nym-client`
* `nym-socks5-client`

```admonish warning
Nymvisor is an early and experimental software. Users should use it at their own risk.
```

## Current version
```
<!-- cmdrun ../../../../target/release/nymvisor --version | grep "Build Version" | cut -b 21-26  -->
```

## Preliminary steps  
You need to have at least one Mixnet node / client / Nym API instance already set up on the **same VPS** that you wish to run Nymvisor on. 

> Using Nymvisor presumes your VPS is running an operating system that is compatible with the pre-compiled binaries avaliable on the [Github releases page](https://github.com/nymtech/nym/releases). If you're not, then until we're packaging for a greater variety of operating systems, you're stuck with [manually upgrading your node](manual-upgrade.md).

## Setup and Usage 
### Viewing command help
You can check that your binaries are properly compiled with:

```
./nymvisor --help
```

Which should return a list of all available commands.

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nymvisor --help -->
```
~~~

You can also check the various arguments required for individual commands with:

```
./nymvisor <COMMAND> --help
```

### Initialising your Nymvisor Instance 
> This example will use the Mix Node binary as an example - however replacing `nym-mixnode` with any other supported binary will work the same.  

Initialise your Nymvisor instance with the following command. You must initialise Nymvisor with the binary you wish to add upgrades for:  

```
./nymvisor init --daemon-home ~/.nym/<NODE_TYPE>/<NODE_ID> <PATH_TO_NODE_BINARY> 
```

Where the value of `--daemon-home` might be `~/.nym/mixnodes/my-node` and `<PATH_TO_NODE_BINARY>` might be `/home/my_user/nym/target/release/nym-mixnode`, or wherever your node binary is located. 

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nymvisor init --daemon-home ~/.nym/mixnodes/my-node ../../../../target/release/nym-mixnode | tail -20 -->
```
~~~

By default this will create config files at `~/.nym/nymvisors/instances/<NODE_TYPE>-default/config/config.toml` as shown in the console output above. For config options look at the different `--flags` available, or the [environment variables](nymvisor-upgrade.md#environment-variables) section below. 

### Running your Nymvisor Instance 
Nymvisor acts as a wrapper around the specified node process - it has to do this in order to be able to pause and restart this process. As such, you need to run your node _via_ Nymvisor! 

The interface to the `nymvisor run <ARGS>` command is quite simple. Any argument passed after the `run` command will be passed directly to the underlying daemon, for example: `nymvisor run run --id my-mixnode` will run the `$DAEMON_NAME run --id my-mixnode` command (where `DAEMON_NAME` is the name of the binary itself (e.g. `nym-api`, `nym-mixnode`, etc.)).

`run` Nymvisor and start your node via the following command. Make sure to stop any existing node before running this command. 

```
./nymvisor run run --id <NODE_ID>  
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nymvisor run run --id my-node -->
```
~~~

Nymvisor will now manage your node process (for an in-depth overview of this command check the [in-depth command information](./nymvisor-upgrade.md#commands-in-depth) below). It will periodically poll [this endpoint](https://nymtech.net/.wellknown/nym-mixnode/upgrade-info.json) (replace `nym-mixnode` with whatever node you may actually be running via Nymvisor) and check for a new `version` of the binary it is watching. If this exists, it will then, using the information there:
* pause your node process
* grab the new binary (`version`)  
* verify it against the provided `checksum` 
* perform a data backup of the existing node
* replace the old binary with the new one 
* restart the process 

And that's it! Check the [maintenance page](./maintenance.md#for-nymvisor) for information on Nymvisor process maintenance and automation, and you can find more in-depth information about the various aspects of Nymvisor such as what happens [under the hood](#commands-in-depth) for various commands. 

### Creating an Adhoc Upgrade 
`nymvisor add-upgrade <PATH_TO_EXECUTABLE> --upgrade-name=<NAME> --arg1=value1 --arg2=value2 ...` can be used to amend an existing `upgrade-plan.json` by creating new entries or to add an executable to an existing scheduled upgrade so that it would not have to be downloaded.

>Generally users **will not have to use this command**. Situations in which this command might be used are: 
> - an adhoc upgrade if e.g. a patched version of a binary was required 
> - if a user doesn't trust the verification process of Nymvisor's pipeline and wishes to build/verify the binary themselves before using Nymvisor to perform the upgrade
> - if a user isn't using a currently supported operating system and needs to manually specify a binary they have built themselves 

Similarly to `init`, `add-upgrade` requires a positional argument specifying a valid path to the upgrade binary. Furthermore, the `--upgrade-name` keyword argument must be set in order to declare the upgrade name. The remaining arguments are optional. They include:
- `--force` - if specified, will allow Nymvisor to overwrite existing upgrade binary / `upgrade-info.json` files if they already exist
- `--add-binary` - indicate that this command should only add binary to an existing scheduled upgrade
- `--now` - if specified will force the upgrade to be performed immediately (technically not 'immediately' within few seconds). It can't be specified alongside either `--upgrade-time` or `--upgrade-delay` arguments
- `--publish-date` - if a new `upgrade-info.json` file is going to be created, this argument will specify the `publish_date` metadata field. Otherwise, the current time will be used. The [RFC3339 datetime](https://www.rfc-editor.org/rfc/rfc3339) format is expected
- `--upgrade-time` - specifies the time at which the provided upgrade will be performed (RFC3339 formatted). If left unset, the upgrade will be performed in 15 minutes. It can't be specified alongside either `--now` or `--upgrade-delay` arguments.
- `--upgrade-delay` - specifies delay until the provided upgrade is going to get performed. If let unset, the upgrade will be performed in 15 minutes. It can't be specified alongside either `--upgrade_time` or `--now` arguments.

## Config 
The output format of `nymvisor config` can be further configured with `--output` argument. By default a human-readable text representation is used:
```
id:                                nym-mixnode-default
daemon name:                       nym-mixnode
daemon home:                       /home/nym/.nym/mixnodes/my-mixnode
upstream base upgrade url:         https://nymtech.net/.wellknown/
disable nymvisor logs:             false
CUSTOM upgrade data directory      ""
upstream absolute upgrade url:     ""
allow binaries download:           true
enforce download checksum:         true
restart after upgrade:             true
restart on failure:                false
on failure restart delay:          10s
max startup failures:              10
startup period duration:           2m
shutdown grace period:             10s
CUSTOM backup data directory       ""
UNSAFE skip backups                false
```

Adding `--output=json` would format the same data into JSON which can be more easily parsed programmatically to e.g. pipe the output into `jq` for further processing.
```
nymvisor config --output=json
```
outputs: 
```
{"nymvisor":{"id":"nym-mixnode-default","upstream_base_upgrade_url":"https://nymtech.net/.wellknown/","upstream_polling_rate":"1h","disable_logs":false,"upgrade_data_directory":null},"daemon":{"name":"nym-mixnode","home":"/home/nym/.nym/mixnodes/my-mixnode","absolute_upstream_upgrade_url":null,"allow_binaries_download":true,"enforce_download_checksum":true,"restart_after_upgrade":true,"restart_on_failure":false,"failure_restart_delay":"10s","max_startup_failures":10,"startup_period_duration":"2m","shutdown_grace_period":"10s","backup_data_directory":null,"unsafe_skip_backup":false}}
```

## CLI Overview  
Command options are:
- `help`, `--help`, or `-h` - Output Nymvisor help information and display the available commands.
- `config` - Display the current Nymvisor configuration, that means displaying the current configuration file that might have been overridden with environment variables value that Nymvisor is using.
- `init` - Generate a `config.toml` file for this instance of Nymvisor that will use the provided arguments alongside any environmental variables that are set.
- `add-upgrade` - Add an upgrade manually to Nymvisor. This command allows you to easily add the binary corresponding to an upgrade or amend the existing `upgrade-plan.json` whilst creating new `upgrade-info.json` file.
- `build-info` - Output the build information.
- `daemon-build-info` - Output the build information of the current binary used by the associated daemon.
- `run` - Run the configured binary using the rest of the provided arguments.
- `-V` or `--version` - Output the Nymvisor version

Similarly to other Nym binaries, Nymvisor supports a global `--config-env-file` or `-c` flag that allows specifying path to a `.env` file defining any relevant environmental variables that are going to be applied to any of the Nymvisor commands as described in the [Environment section](./nymvisor-upgrade.md#environment-variables).

For commands that depend on Nymvisor config file (i.e. all but `init`), the configuration file is loaded as follows:
- if available, reading `$NYMVISOR_CONFIG_PATH`
- otherwise, if `$NYMVISOR_ID` is set, a default path will be used, i.e. `$HOME/.nym/nymvisors/instances/$NYMVISOR_ID/config/config.toml`
- finally, if there's only a single `nymvisor` instance instantiated (as defined by directories in `$HOME/.nym/nymvisors/instances`), that one will be loaded
- if all of the above fails, an error is returned

Nymvisor attempts to load the file from the derived path. If it fails, it attempts to use one of the older schemas to and upgrade it as it goes, the loaded configuration is then overridden with any value that might have been set in the environment.

## Environment Variables 

> Please note environmental variables take precedence over any arguments passed, i.e. if one were to specify `--daemon_home="/foo"` and set `DAEMON_HOME="bar"`, the value of `"bar"` would end up being used.

For any of its commands as described in [CLI Overview section](./nymvisor-upgrade.md#cli-overview), Nymvisor reads its configuration from the following environment variables:

- `NYMVISOR_ID` is the human-readable identifier of the particular Nymvisor instance.
- `NYMVISOR_CONFIG_PATH` is used to manually override path to the configuration file of the Nymvisor instance.
- `NYMVISOR_UPSTREAM_BASE_UPGRADE_URL` (defaults to https://nymtech.net/.wellknown/) is the base url of the upstream source for obtaining upgrade information for the daemon. It will be used fo constructing the full url, i.e. `$NYMVISOR_UPSTREAM_BASE_UPGRADE_URL/$DAEMON_NAME/upgrade-info.json`.
- `NYMVISOR_UPSTREAM_POLLING_RATE` (defaults to 1h) is polling rate the upstream url for upgrade information.
- `NYMVISOR_DISABLE_LOGS` (defaults to `false`). If set to `true`, this will disable Nymvisor logs (but not the underlying process) completely.
- `NYMVISOR_UPGRADE_DATA_DIRECTORY` is the custom directory for upgrade data - binaries and upgrade plans. If not set, the global Nymvisors' data directory will be used instead.
- `DAEMON_NAME` is the name of the binary itself (e.g. `nym-api`, `nym-mixnode`, etc.).
- `DAEMON_HOME` is the location where the `nymvisor/` directory is kept that contains the auxiliary files associated with the underlying daemon instance, such as any backups or current version information, e.g. `$HOME/.nym/nym-api/my-nym-api`, `$HOME/.nym/mixnodes/my-mixnode`, etc.
- `DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL` is the absolute (i.e. the full url) upstream source for upgrade plans for this daemon. The url has to point to an endpoint containing a valid `UpgradeInfo` json file. If set it takes precedence over `NYMVISOR_UPSTREAM_BASE_UPGRADE_URL`.
- `DAEMON_ALLOW_BINARIES_DOWNLOAD` (defaults to `true`), if set to `true`, it will enable auto-downloading of new binaries (as declared by urls in corresponding `upgrade-info.json` files). For security reasons one might wish to disable it and instead manually provide binaries by either placing them in the appropriate directory or by invoking `add-upgrade` command.
- `DAEMON_ENFORCE_DOWNLOAD_CHECKSUM` (defaults to `true`), if set to `true` Nymvisor will require that a checksum is provided in the upgrade plan for the upgrade binary to be downloaded. If disabled, Nymvisor will not require a checksum to be provided, but still check the checksum if one is provided.
- `DAEMON_RESTART_AFTER_UPGRADE` (defaults to `true`), if set to `true` Nymvisor will restart the subprocess with the same command-line arguments and flags (but with the new binary) after a successful upgrade. Otherwise (`false`), Nymvisor stops running after an upgrade and requires the system administrator to manually restart it. **Note restart is only after the upgrade and does not auto-restart the subprocess after an error occurs.** That is controlled via `DAEMON_RESTART_ON_FAILURE`.
- `DAEMON_RESTART_ON_FAILURE` (defaults to `true`), if set to `true`, Nymvisor will restart the subprocess with the same command-line arguments and flags if it has terminated with a non-zero exit code.
- `DAEMON_FAILURE_RESTART_DELAY` (defaults to 10s), if `DAEMON_RESTART_ON_FAILURE` is set to `true`, this will specify a delay between the process shutdown (with a non-zero exit code) and it being restarted.
- `DAEMON_MAX_STARTUP_FAILURES` (defaults to 10) if `DAEMON_RESTART_ON_FAILURE` is set to `true`, this defines the maximum number of startup failures the subprocess can experience in a quick succession before no further restarts will be attempted and Nymvisor will terminate.
- `DAEMON_STARTUP_PERIOD_DURATION` (defaults to 120s) if `DAEMON_RESTART_ON_FAILURE` is set to `true`, this defines the length of time during which the subprocess is still considered to be in the startup phase when its failures are going to be counted towards the limit defined in `DAEMON_MAX_STARTUP_FAILURES`.
- `DAEMON_SHUTDOWN_GRACE_PERIOD` (defaults to 10s), specifies the amount of time Nymvisor is willing to wait for the subprocess to undergo graceful shutdown after receiving an interrupt before it sends a kill signal.
- `DAEMON_BACKUP_DATA_DIRECTORY` specifies custom backup directory for daemon data. If not set, `DAEMON_HOME/nymvisor/backups` is used instead.
- `DAEMON_UNSAFE_SKIP_BACKUP` (defaults to `false`), if set to `true`, all upgrades will be performed directly without performing any backups. Otherwise (`false`), Nymvisor will back up the contents of `DAEMON_HOME` before trying the upgrade.

## Dir structure
The folder structure of Nymvisor is heavily inspired by Cosmovisor, but with some notable changes to accommodate our binaries having possibly multiple instances due to their different `--id` flags. The data is spread through three main directories:
- in a global `nymvisors` data directory shared by all Nymvisor instances (default: `$HOME/.nym/nymvisors/data`) that contains daemon upgrade plans, binaries and upgrades histories. It includes a subdirectory for each version of the application (i.e. `genesis` or `upgrades<name>`). Within each subdirectory is the application binary (i.e. `bin/$DAEMON_NAME`), the associated `upgrade-info.json` and any additional auxiliary files associated with each binary. `current` is a symbolic link to the currently active directory (i.e. `genesis` or `upgrades/<NAME>`)
- in a home directory of a particular `nymvisor` instance (e.g. `$HOME/.nym/nymvisors/instances/<nymvisor-instance-id>/`). It includes subdirectories for its configuration file (i.e. `config/config.toml`), that preconfigures the instance, and for any additional persistent data that might be added in the future (i.e. `data`)
- in a `nymvisor` data directory inside the home directory of the managed daemon instance (default: `$HOME/.nym/$DAEMON_NAME/nymvisor`) that contains subdirectories for data backups (i.e. `backups/<name>`) and current version information (`current-version-info.json`)

A sample full structure looks as follows:

```
~/.nym
├── nymvisors
│   ├── instances
│   │   ├── <id1>
│   │   │   ├── config
│   │   │   │   └── config.toml
│   │   │   └── data
│   │   │       └── ...
│   │   └── <id2>
│   │       └── ...
│   └── data
│       ├── nym-api
│       │   ├── current -> genesis or upgrades/<name>
│       │   ├── genesis
│       │   │   ├── bin
│       │   │   │   └── nym-api
│       │   │   └── upgrade-info.json
│       │   ├── upgrades
│       │   │   └── <upgrade-name>
│       │   │       ├── bin
│       │   │       │   └── nym-api
│       │   │       └── upgrade-info.json
│       │   ├── upgrade-history.json
│       │   └── upgrade-plan.json
│       ├── nym-mixnode
│       │   └── ...
│       └── $DAEMON_NAME
│           └── ...
└── nym-api
├── <id1>
│   ├── config
│   │   └── <nym-api-config-data>
│   ├── data
│   │   └── <nym-api-data>
│   └── nymvisor
│       ├── backups
│       │   └── <upgrade-name>
│       │       └── ....
│       └── current-version-info.json
└── <id2>
└── ...
```

## Commands In-Depth
This section outlines what happens under the hood with the following commands:

### Init 
`init` does the following:
- executes the `$DAEMON_NAME build-info` command on the daemon executable to check its validity and obtain its name
- creates the required directory structure:
  - `$DAEMON_HOME/nymvisor` folder if it doesn't yet exist 
  - `$DAEMON_BACKUP_DATA_DIRECTORY` folder if it doesn't yet exist 
  - `$NYMVISOR_UPGRADE_DATA_DIRECTORY` folder if it doesn't yet exist 
  - `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/genesis/bin` folder if it doesn't yet exist 
  - `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/upgrades` folder if it doesn't yet exist
- copies the provided executable to `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/genesis/bin/$DAEMON_NAME`
- generates initial `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/genesis/upgrade-info.json` file based on the provided binary
- generates initial `$DAEMON_HOME/nymvisor/current-version-info.json` file based on the provided binary
- creates a `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/current` symlink pointing to `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/genesis`
- saves the Nymvisor instance's config file to `$NYMVISOR_CONFIG_PATH` and creates the full directory structure for the file
- outputs (to `stdout`) the full configuration used

> `nymvisor init` is specifically for initializing Nymvisor, and should **not** be confused with a daemon's `init` command - such as `nym-mixnode init` (e.g. `cosmovisor run init`).

### Run 
`nymvisor run` is a lightweight wrapper around the underlying daemon. It uses only a single thread and spawns three simple tasks:
- an upstream poller that checks the upstream source (as defined by `DAEMON_ABSOLUTE_UPSTREAM_UPGRADE_URL` or derived from `NYMVISOR_UPSTREAM_BASE_UPGRADE_URL`) for any recently released upgrades. It then creates appropriate `upgrade-info.json` file and updates the `upgrade-plan.json`
- a file watcher for the `upgrade-plan.json` file that can notify the main run loop of upgrades that were added by either the above upstream poller task or via the `add-upgrade` command,
- the daemon run loop that:
  - runs the `DAEMON_NAME` with the provided arguments until:
    - it completes the execution (with any exit code), 
    - Nymvisor receives a `SIGINT`, `SIGTERM` or `SIGQUIT`, 
    - a new upgrade is scheduled to be performed (by other task watching for changes in `upgrade-plan.json` and waiting until the `upgrade_time`, 
  - if `DAEMON_UNSAFE_SKIP_BACKUP` is not set to `true`, it backups the content of `DAEMON_HOME` directory, 
  - performs the binary upgrade by:
    - creating a temporary, exclusive and non-blocking, `upgrade.lock` file for the `DAEMON_NAME`. `flock` with `LOCK_EX | LOCK_NB` is used for that purpose. The file is created in case users didn't read any warnings and attempted to run multiple instances of `nymvisor` managing the same `DAEMON_NAME`, 
    - downloading the upgrade binary for the runners architecture using one of the urls defined in `upgrade-info.json`. Note, however, that this is only done if the binary associated with the `<UPGRADE-NAME>` does not already exist and `DAEMON_ALLOW_DOWNLOAD_BINARIES` is set to `true`, 
      - if the binary has been downloaded and `DAEMON_ENFORCE_DOWNLOAD_CHECKSUM` is set to true, the file checksum is verified using the specified algorithm,
    - verifying the upgrade binary - checking if it's a valid executable with expected `build-info`. Note that this will also set `a+x` bits on the file if those permissions have not already been set, 
      - removing the queued upgrade from `upgrade-plan.json`,
      - inserting new upgrade into the `upgrade-history.json`,
      - updating the `current-version-info.json`,
      - updating the `$NYMVISOR_UPGRADE_DATA_DIRECTORY/$DAEMON_NAME/current` symlink to the upgrade directory,
      - removing the `upgrade.lock` file.
  - the above loop is repeated if either:
    - the daemon has crashed and `DAEMON_MAX_STARTUP_FAILURES` has not been reached yet,
    - the daemon has successfully been upgraded, `DAEMON_RESTART_AFTER_UPGRADE` has been set to `true` and the manual flag on the performed upgrade has been set to `false`.

### Add-Upgrade
`nymvisor add-upgrade` does the following:
- executes the `$DAEMON_NAME build-info` command on the daemon executable to check its validity and obtain its name 
- attempts to load existing `upgrade-info.json` for the provided `<upgrade-name>`. If it already exists and neither `--force` nor `--add-binary` was specified, Nymvisor will terminate
- checks if `upgrades/<upgrade-name>/$DAEMON_NAME` binary already exists. If it does and `--force` flag was not specified, Nymvisor will terminate the provided upgrade binary is copied to its appropriate location
- if applicable, new `upgrade-info.json` is created and written to its appropriate location
- `upgrade-plan.json` is updated with the new upgrade details. If there's an active Nymvisor instance running, this change will be detected to initialise upgrade process

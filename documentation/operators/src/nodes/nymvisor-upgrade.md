# Automatic Node Upgrade: Nymvisor Setup and Usage

> The Nymvisor binary was built in the [building nym](../binaries/building-nym.md) section. If you haven't yet built Nym and want to run the code, go there first. You can build just Nymvisor with `cargo build --release --bin nymvisor`. 

## What is Nymvisor?
Nymvisor is a process manager for Nym binaries that monitors the Nym release information for any newly released binaries. If it detects any changes, Nymvisor can automatically download the binary, stop the current binary, switch from the old binary to the new one, and finally restart the underlying process with the new binary. 

In essence, it tries to mirror the behaviour of [Cosmovisor](https://github.com/cosmos/cosmos-sdk/tree/main/tools/cosmovisor), a tool used by Cosmos blockchain operators for managing/automating chain upgrades. Nymvisor, however, introduces some Nym-specific changes since, for example, upgrade information is obtained from our GitHub [releases page](https://github.com/nymtech/nym/releases) instead of (in the case of Cosmos blockchains) governance proposals. 

You can use Nymvisor to automate the upgrades of the following binaries:
* `nym-mixnode`
* `nym-gateway`
* `nym-network-requester`
* `nym-client`
* `nym-socks5-client`
* `nym-api` 

```admonish warning
Warning Nymvisor is an early and experimental software with no stability guarantees. Users should use it at their own risk.
```

## Current version
```
<!-- cmdrun ../../../../target/release/nymvisor --version | grep "Build Version" | cut -b 21-26  -->
```

## Preliminary steps  
You need to have at least one Mixnet node / client / Nym API instance already set up on the **same VPS** that you wish to run Nymvisor on. 

```admonish warning
Using Nymvisor presumes your VPS is running an operating system that is compatible with the pre-compiled binaries avaliable on the Github releases page. If you're not, then until we're packaging for a greater variety of operating systems, you're stuck with [manually upgrading your node](manual-upgrade.md).
```

## Setup 
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

By default this will create config files at `~/.nym/nymvisors/instances/<NODE_TYPE>-default/config/config.toml` as shown in the console output above. For config options look at the different `--flags` available, or the [environment variables](#environment-variables-) section below. 

### Running your Nymvisor Instance 
Nymvisor acts as a wrapper around the specified node process - it has to do this in order to be able to pause and restart this process. As such, you need to run your node _via_ Nymvisor! The interface to the `nymvisor run <args>` command is quite simple. Any argument passed after the `run` command will be passed directly to the underlying daemon, for example: `nymvisor run run --id my-mixnode` will run the `$DAEMON_NAME run --id my-mixnode` command (where `DAEMON_NAME` is the name of the binary itself (e.g. `nym-api`, `nym-mixnode`, etc.).

`run` Nymvisor and start your node via the following command. Make sure to stop any existing node before running this command. 

```
./nymvisor run run --id <NODE_ID>  
```

~~~admonish example collapsible=true title="Console output"
```
<!-- cmdrun ../../../../target/release/nymvisor run run --id my-node -->
```
~~~

Nymvisor will now manage your node process. It will also periodically poll [this endpoint](https://nymtech.net/.wellknown/nym-mixnode/upgrade-info.json) (replace `nym-mixnode` with whatever node you may actually be running via Nymvisor) and check for a new version. If this exists, it will then:
* pause your node process
* grab the new binary 
* verify it against the provided `checksum` 
* replace the old binary with the new one 
* restart the process 

And that's it! Check the [maintenance page](./maintenance.md#for-nymvisor) for information on Nymvisor process maintenance and automation.  

### Creating an Adhoc Upgrade 
TODO

why would you want an adhoc upgrade instead?

nymvisor add-upgrade <path to executable> --upgrade-name=<name> --arg1=value1 --arg2=value2 ... command can be used to amend existing upgrade-plan.json by creating new entries or to add an executable to an existing scheduled upgrade so that it would not have to be downloaded.

https://nymsphere.vercel.app/architecture/tooling/nymvisor#add-upgrade

### More complex: you have multiple nodes on a single box
TODO 

## CLI Overview  
TODO 

https://nymsphere.vercel.app/architecture/tooling/nymvisor#command-line-interface

## Environment Variables 
For any of its commands as described in [CLI Overview section](#cli-overview-), Nymvisor reads its configuration from the following environment variables:

- `NYMVISOR_ID` is the human-readable identifier of the particular nymvisor instance.
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
- `DAEMON_FAILURE_RESTART_DELAY` (defaults to 10s), if `DAEMON_RESTART_ON_FAILURE` is set to true, this will specify a delay between the process shutdown (with a non-zero exit code) and it being restarted.
- `DAEMON_MAX_STARTUP_FAILURES` (defaults to 10) if `DAEMON_RESTART_ON_FAILURE` is set to `true`, this defines the maximum number of startup failures the subprocess can experience in a quick succession before no further restarts will be attempted and Nymvisor will terminate.
- `DAEMON_STARTUP_PERIOD_DURATION` (defaults to 120s) if `DAEMON_RESTART_ON_FAILURE` is set to `true`, this defines the length of time during which the subprocess is still considered to be in the startup phase when its failures are going to be counted towards the limit defined in `DAEMON_MAX_STARTUP_FAILURES`.
- `DAEMON_SHUTDOWN_GRACE_PERIOD` (defaults to 10s), specifies the amount of time Nymvisor is willing to wait for the subprocess to undergo graceful shutdown after receiving an interrupt before it sends a kill signal.
- `DAEMON_BACKUP_DATA_DIRECTORY` specifies custom backup directory for daemon data. If not set, `DAEMON_HOME/nymvisor/backups` is used instead.
- `DAEMON_UNSAFE_SKIP_BACKUP` (defaults to `false`), if set to `true`, all upgrades will be performed directly without performing any backups. Otherwise (`false`), Nymvisor will back up the contents of `DAEMON_HOME` before trying the upgrade.

> Please note environmental variables take precedence over any arguments passed, i.e. if one were to specify `--daemon_home="/foo"` and set `DAEMON_HOME="bar"`, the value of `"bar"` would end up being used.

## Dir structure 
TODO 

https://nymsphere.vercel.app/architecture/tooling/nymvisor#environment
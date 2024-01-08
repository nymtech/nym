# NymVPN UI app for desktop clients

This is the application UI layer for the next NymVPN clients.

## Install

#### Prerequisites

- Rust
- Nodejs, latest LTS version recommended
- yarn 1.x

Some system libraries are required depending on the host platform.
Follow the instructions for your specific OS [here](https://tauri.app/v1/guides/getting-started/prerequisites)

To install:

```
yarn
```

## Required config

First you can provide a network configuration using en env file,
pick the relevant one [here](https://github.com/nymtech/nym/tree/develop/envs).
The mainnet config will be used by default if not provided.

Then create the main app config file `config.toml` under `nym-vpn`
directory, full path is platform specific:

- Linux: Resolves to `$XDG_CONFIG_HOME` or `$HOME/.config`
- macOS: Resolves to `$HOME/Library/Application Support`
- Windows: Resolves to `{FOLDERID_RoamingAppData}`

For example on Linux the path would be `~/.config/nym-vpn/config.toml`

```toml
# example config on Linux

# path to the env config file if you provide one
env_config_file = "/home/<USER>/.config/nym-vpn/qa.env"
```

## Dev

```
yarn dev:app
```

or

```
cd src-tauri
cargo tauri dev
```

**NOTE** Starting a VPN connection requires root privileges as it will set up a link interface.
If you want to connect during development, you need to run the app as root,
likely using `sudo` (or equivalent)

```shell
sudo -E RUST_LOG=debug cargo tauri dev
```

#### Logging

Rust logging (standard output) is controlled by the `RUST_LOG`
env variable

Example:

```
cd src-tauri
RUST_LOG=trace cargo tauri dev
```

## Dev in the browser

For convenience and better development experience, we can run the
app directly in the browser

```
yarn dev:browser
```

Then press `o` to open the app in the browser.

#### Tauri commands mock

Browser mode requires all tauri [commands](https://tauri.app/v1/guides/features/command) (IPC calls) to be mocked.
When creating new tauri command, be sure to add the corresponding
mock definition into `nym-vpn/ui/src/dev/tauri-cmd-mocks/` and
update `nym-vpn/ui/src/dev/setup.ts` accordingly.

## Type bindings

[ts-rs](https://github.com/Aleph-Alpha/ts-rs) can be used to generate
TS type definitions from Rust types

To generate bindings, first
[annotate](https://github.com/Aleph-Alpha/ts-rs/blob/main/example/src/lib.rs)
Rust types, then run

```
cd src-tauri
cargo test
```

Generated TS types will be located in `src-tauri/bindings/`

## Build

```
yarn build:app
```

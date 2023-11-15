# NymVPN UI app for desktop clients

This is the application UI layer for the next NymVPN clients.

## Install

#### Prerequisites

- Rust
- Nodejs, latest LTS version recommended
- yarn 1.x

Some system libraries are required depending on the host platform.
Follow the instructions for your specific OS [here](https://tauri.app/v1/guides/getting-started/prerequisites)

To install run

```
yarn
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

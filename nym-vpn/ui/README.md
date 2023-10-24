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

## Build

To build as a **shared library**

```
yarn build && cd src-tauri && cargo build --release --lib --features custom-protocol

#alias
yarn build:app
```

You can build for a different platform using [Cross](https://github.com/cross-rs/cross).
For example, to build for Windows on Linux:

```
cross build --target x86_64-pc-windows-gnu --release --lib --features custom-protocol
```

# NymVPN UI app for desktop clients

This is the application UI layer for the next NymVPN clients.

## Install

```
yarn
```

## Dev

```
yarn dev:app
```

## Dev in the browser

For convenience and better development experience, we can run the
app in dev mode in the browser

```
yarn dev:browser
```

#### Tauri commands mock

In browser mode requires all tauri [commands](https://tauri.app/v1/guides/features/command) (IPC calls) in use to be mocked.
When creating new tauri command, be sure to add the corresponding
mock definition into `nym-vpn/ui/src/dev/tauri-cmd-mocks/` and
update `nym-vpn/ui/src/dev/setup.ts` accordingly.

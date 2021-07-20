
## Getting started

1. Start Sapper([docs](https://sapper.svelte.dev/docs/)) -> `yarn run dev`
2. Start Tauri([docs](https://tauri.studio/en/)) in another terminal -> `yarn tauri dev`
3. Start validator-api locally, or override `validator-urls` in `index.svelte`

## Getting around

+ Frontend -> `tauri-client/`
  + logic -> `tauri-client/src/routes/index.svelte`
  + assets -> `tauri-client/static`
+ Backend -> `tauri-client/src/src-tauri`

## Build standalone app

+ yarn tauri build [--debug]
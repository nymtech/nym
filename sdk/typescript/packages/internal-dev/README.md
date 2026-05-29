# smolmix SDK playground (internal-dev)

Internal-only browser playground that exercises the smolmix-based TS SDK
family end-to-end against a live mixnet:

- `@nymproject/mix-tunnel` &mdash; tunnel lifecycle (setup, state, disconnect)
- `@nymproject/mix-fetch` &mdash; HTTP/HTTPS through the mixnet
- `@nymproject/mix-dns` &mdash; hostname resolution via the IPR
- `@nymproject/mix-websocket` &mdash; WS/WSS through the mixnet

Use at your own risk; this is dev scaffolding, not a polished demo.

## Getting started

From the repo root:

```
make sdk-wasm-build      # build smolmix-wasm pkg/
pnpm i                   # install workspace deps (resolves workspace:* refs)
```

Then build the four TS SDK packages so the playground can resolve them:

```
pnpm build:ci:sdk
```

Then start the dev server (webpack):

```
cd sdk/typescript/packages/internal-dev
pnpm start
```

Open <http://localhost:3000/>. The page has four sections (Tunnel, Fetch,
DNS, WebSocket); start with **Setup tunnel** and wait for the state to flip
green. The other three sections are then live.

Alternative: parcel-based playground in `parcel/` &mdash; same source, different
bundler.

## Iterating on the SDKs

If you edit a TS SDK package (e.g. `mix-fetch`) the playground won't see the
change until the package is rebuilt. From the repo root:

```
pnpm --filter @nymproject/mix-fetch build
```

webpack-dev-server picks up the new `dist/` via the workspace symlink and
hot-reloads.

If you edit `mix-tunnel`'s rollup config or anything touching the inlined
worker bundle, run the package's own `pnpm build` so the rollup chain
re-runs:

```
pnpm --filter @nymproject/mix-tunnel build
```

If you edit `wasm/smolmix` itself, run `make -C wasm/smolmix build-debug`
to regenerate `pkg/` &mdash; then `pnpm --filter @nymproject/mix-tunnel build`
to pick up the new wasm bytes.

## Stuck?

```
rm -rf node_modules && pnpm i && pnpm start
```

Often resets pnpm-link state if the workspace package symlinks have drifted.

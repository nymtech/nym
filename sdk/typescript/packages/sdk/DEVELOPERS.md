# Information for Nym developers

> **This package is legacy.** `@nymproject/sdk` is the v1 TypeScript SDK. It is not built
> by `pnpm sdk:build` and not published by `sdk/typescript/scripts/publish.sh`. It is absent
> from the committed `pnpm-workspace.yaml`, though `pnpm dev:on` pulls it in via the
> `sdk/typescript/packages/**` glob, which is what makes the build below work. It remains
> on disk and on npm for existing consumers only.
>
> Current work happens in the four smolmix-family packages:
> `@nymproject/mix-tunnel`, `@nymproject/mix-fetch`, `@nymproject/mix-dns` and
> `@nymproject/mix-websocket`. For how those are built, versioned and released, see
> [`ts-sdk-publishing.md`](../../../../ts-sdk-publishing.md) at the repository root.

## Prerequisites

- **Node.js 24**
- **pnpm**
- **Rust toolchain** with `wasm-pack` and `wasm-opt` ([setup instructions](https://rustwasm.github.io/docs/book/game-of-life/setup.html))
- **jq**, used by `wasm/smolmix/Makefile` to mark the generated wasm package private

## Building from source

This package depends on `@nymproject/nym-client-wasm`, which is built from Rust first.
From the **root of the monorepo**:

```bash
pnpm build:wasm                      # make sdk-wasm-build: wasm/client and wasm/smolmix
pnpm dev:on                          # inject the SDK workspaces into pnpm-workspace.yaml
pnpm install --no-frozen-lockfile    # install dependencies
```

The order matters. `dev:on` adds `wasm/smolmix/pkg` to `pnpm-workspace.yaml`, and that
directory only exists once the wasm has been built. Installing first fails with
`ERR_PNPM_WORKSPACE_PKG_NOT_FOUND`. The `--no-frozen-lockfile` flag is needed because
`dev:on` introduces workspace importers the committed lockfile does not know about.

Then from `sdk/typescript/packages/sdk`:

```bash
pnpm build:dev       # full dev build -> dist/
pnpm build:dev:esm   # ESM-only (faster iteration)
pnpm start:dev       # watch mode, rebuilds ESM on changes
```

Run `pnpm dev:off` from the root when you are done, so `pnpm-workspace.yaml` returns to
its committed state.

## Publishing

This package is not published by any current workflow. The `publish-sdk-npm` workflow and
`sdk/typescript/scripts/publish.sh` cover the four smolmix-family packages only; the
runbook for those is [`ts-sdk-publishing.md`](../../../../ts-sdk-publishing.md).

The v1 mixFetch build (`wasm/mix-fetch`, the Go `go-mix-conn` component and its root-CA
bundle script) has been removed from the repository. Any instructions elsewhere that
reference `./wasm/mix-fetch/...` paths, a Go toolchain prerequisite, or
`./sdk/typescript/scripts/release.sh` no longer apply to this package.

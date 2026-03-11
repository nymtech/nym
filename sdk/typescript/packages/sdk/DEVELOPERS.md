# Information for Nym developers

## Prerequisites

- **Node.js 20** (LTS)
- **Yarn** (`npm install -g yarn`)
- **Rust toolchain** with `wasm-pack` and `wasm-opt` — [setup instructions](https://rustwasm.github.io/docs/book/game-of-life/setup.html)
- **Go 1.24+** (required for `mix-fetch` WASM builds - same as defined version in CI)

## Building from source

The SDK depends on WASM packages that must be built from Rust first.

From the **root of the monorepo**:

```bash
yarn dev:on          # add dev workspaces to root package.json
yarn                 # install dependencies
yarn build:wasm      # build Rust -> WASM packages
```

Then from `sdk/typescript/packages/sdk`:

```bash
yarn build:dev       # full dev build -> dist/
yarn build:dev:esm   # ESM-only (faster iteration)
yarn start:dev       # watch mode, rebuilds ESM on changes
```

## Publishing

### Via CI 

The `publish-sdk-npm` GitHub Actions workflow (`.github/workflows/publish-sdk-npm.yml`) handles the cert update, build, and publish. You only need to do the version bump and RC suffix locally:

1. Bump versions, commit, and push:
   ```bash
   yarn sdk:versions:bump
   git add -A && git commit -m "chore: bump sdk versions"
   git push
   ```
2. Trigger the `publish-sdk-npm` workflow from GitHub Actions.
3. After the workflow succeeds, add the RC suffix, commit, and push:
   ```bash
   yarn sdk:versions:add-rc
   git add -A && git commit -m "chore: add rc suffix"
   git push
   ```

### Manually (local build + publish)

1. Update the root CA certificate bundle:
   ```bash
   ./wasm/mix-fetch/go-mix-conn/scripts/update-root-certs.sh
   ```
2. Bump version numbers:
   ```bash
   yarn sdk:versions:bump
   ```
3. Build and publish:
   ```bash
   ./sdk/typescript/scripts/release.sh
   ```
4. Add RC suffix and commit:
   ```bash
   yarn sdk:versions:add-rc
   ```

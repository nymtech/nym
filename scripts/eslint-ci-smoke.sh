#!/usr/bin/env bash
# Fast CI: flat config resolves (incl. parserOptions.project) and workspace lint is warning-clean.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
ESLINT="$ROOT/node_modules/.bin/eslint"

print_cfg() {
  local subdir="$1" relpath="$2"
  (cd "$ROOT/$subdir" && "$ESLINT" --print-config "$relpath" >/dev/null)
}

print_cfg ts-packages/types src/index.ts
print_cfg ts-packages/webpack index.js
print_cfg sdk/typescript/packages/react-components src/components/link/Link.tsx
print_cfg sdk/typescript/packages/mui-theme src/index.ts
print_cfg nym-wallet src/theme/index.tsx

yarn workspace @nymproject/eslint-config-react-typescript run lint --max-warnings 0
yarn workspace @nymproject/types run lint --max-warnings 0
yarn workspace @nymproject/webpack run lint --max-warnings 0
yarn workspace @nymproject/react run lint --max-warnings 0
yarn workspace @nymproject/mui-theme run lint --max-warnings 0
yarn workspace @nymproject/nym-wallet-app run lint --max-warnings 0

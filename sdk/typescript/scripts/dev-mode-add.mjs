import fs from 'fs';

const WORKSPACE_FILE = 'pnpm-workspace.yaml';

// Order matters only for human readability; the script appends missing
// entries to the yaml's `packages:` block. The `wasm/smolmix/pkg` entry
// requires `make -C wasm/smolmix` to have produced pkg/package.json first;
// otherwise `pnpm install` bails with ERR_PNPM_WORKSPACE_PKG_NOT_FOUND from
// mix-tunnel's `workspace:*` lookup.
const devWorkspace = [
  'dist/**',
  'sdk/typescript/packages/**',
  'sdk/typescript/examples/**',
  'sdk/typescript/codegen/**',
  'wasm/smolmix/pkg',
];

const content = fs.readFileSync(WORKSPACE_FILE, 'utf-8');

// Match the packages: block — one or more indented list items
const packagesRegex = /(^packages:\n)((?:  - .+\n)+)/m;
const match = content.match(packagesRegex);
if (!match) throw new Error('Could not find packages: section in pnpm-workspace.yaml');

const current = match[2]
  .split('\n')
  .filter(l => l.startsWith('  - '))
  .map(l => l.replace(/^  - ['"]?/, '').replace(/['"]?\s*$/, ''));

const toAdd = devWorkspace.filter(e => !current.includes(e));
if (toAdd.length === 0) process.exit(0);

const updated = [...current, ...toAdd].map(p => `  - '${p}'`).join('\n') + '\n';
fs.writeFileSync(WORKSPACE_FILE, content.replace(packagesRegex, `$1${updated}`));

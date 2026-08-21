import fs from 'fs';

const WORKSPACE_FILE = 'pnpm-workspace.yaml';
const LOCKFILE = 'pnpm-lock.yaml';
const LOCKFILE_STASH = 'pnpm-lock.yaml.predev';

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

const updated = current
  .filter(p => !devWorkspace.includes(p))
  .map(p => `  - '${p}'`).join('\n') + '\n';

fs.writeFileSync(WORKSPACE_FILE, content.replace(packagesRegex, `$1${updated}`));

// Put back the lockfile dev-mode-add.mjs stashed, so dev:off leaves the tree as
// it found it. Absent stash means dev:on never ran here; leave the lockfile be.
if (fs.existsSync(LOCKFILE_STASH)) {
  fs.copyFileSync(LOCKFILE_STASH, LOCKFILE);
  fs.rmSync(LOCKFILE_STASH);
}

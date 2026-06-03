// Generates Nextra `_meta.json` files for the TypeDoc markdown output.
//
// typedoc-plugin-markdown does a clean write into each package's `out` dir and
// does NOT emit `_meta.json`, so without this step the API-reference sidebar
// falls back to alphabetical order with filename-derived (camelCase-mangled)
// titles. This runs straight after `typedoc` in generate-typedoc.sh, which
// makes `_meta.json` a generated artifact like the markdown beside it: it can't
// drift from the source and a regen can't wipe it.
//
// A TypeDoc output root is any directory containing `globals.md`. For each one
// we write an ordered root `_meta.json` (API Index first, then the category
// folders that exist) plus a `_meta.json` per category folder listing its
// symbol pages. Directories without a `globals.md` (the hand-written page
// trees) are left untouched.

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const DEVELOPERS = path.resolve(scriptDir, '../../..', 'documentation/docs/pages/developers');

// Category folder -> sidebar label, in the order we want them to appear. The
// `globals.md` index page is pinned first; everything else follows this order.
const CATEGORY_LABELS = {
  classes: 'Classes',
  functions: 'Functions',
  interfaces: 'Interfaces',
  enumerations: 'Enumerations',
  'type-aliases': 'Type Aliases',
  variables: 'Variables',
};

function writeMeta(dir, obj) {
  const file = path.join(dir, '_meta.json');
  fs.writeFileSync(file, JSON.stringify(obj, null, 2) + '\n');
}

// Build the `_meta.json` for one TypeDoc output root and its category folders.
function generateForRoot(root) {
  const entries = fs.readdirSync(root, { withFileTypes: true });
  const dirs = new Set(entries.filter((e) => e.isDirectory()).map((e) => e.name));

  const rootMeta = { globals: 'API Index' };
  for (const [folder, label] of Object.entries(CATEGORY_LABELS)) {
    if (!dirs.has(folder)) continue;
    rootMeta[folder] = label;

    // Each category folder holds flat `<Symbol>.md` pages; label each with its
    // own name so the sidebar shows `SetupMixTunnelOpts`, not "Set Up Mix...".
    const folderDir = path.join(root, folder);
    const pages = fs
      .readdirSync(folderDir)
      .filter((f) => f.endsWith('.md'))
      .map((f) => f.slice(0, -'.md'.length))
      .sort((a, b) => a.localeCompare(b));

    const folderMeta = {};
    for (const name of pages) folderMeta[name] = name;
    writeMeta(folderDir, folderMeta);
  }
  writeMeta(root, rootMeta);
}

// Walk `developers/` and treat any directory containing `globals.md` as a root.
function findRoots(dir, found = []) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  if (entries.some((e) => e.isFile() && e.name === 'globals.md')) found.push(dir);
  for (const e of entries) {
    if (e.isDirectory()) findRoots(path.join(dir, e.name), found);
  }
  return found;
}

const roots = findRoots(DEVELOPERS);
if (roots.length === 0) {
  console.error(`No TypeDoc output roots (globals.md) found under ${DEVELOPERS}`);
  process.exit(1);
}
for (const root of roots) {
  generateForRoot(root);
  console.log(`_meta.json written for ${path.relative(DEVELOPERS, root)}`);
}

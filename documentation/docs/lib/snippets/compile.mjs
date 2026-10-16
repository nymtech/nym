// Per-snippet compile check (D10, task 1.10): compile each prose snippet alone
// with Nextra's `compileMdx`. The round trip cannot see a cut through a JSX
// element, because the broken parts join back to a correct page; a compile of
// each part alone can.
//
//   node lib/snippets/compile.mjs [--root <repo root>]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { compileMdx } from 'nextra/compile';
import { openRepo } from './repo.mjs';

/** Compile each `.mdx` under `dir`. @returns {Promise<{ files: number, errors: string[] }>} */
export async function compileSnippets(dir, root) {
  const files = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const full = path.join(d, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.mdx')) files.push(full);
    }
  };
  if (fs.existsSync(dir)) walk(dir);
  const errors = [];
  for (const file of files.sort()) {
    try {
      await compileMdx(fs.readFileSync(file, 'utf-8'), { filePath: file });
    } catch (error) {
      const at = error.place?.line ?? error.line;
      errors.push(`${path.relative(root, file)}${at ? `:${at}` : ''}: ${error.reason ?? error.message}`);
    }
  }
  return { files: files.length, errors };
}

async function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const repo = openRepo(at('root') ? path.resolve(at('root')) : undefined);
  const { files, errors } = await compileSnippets(repo.snippets, repo.root);
  for (const e of errors) console.log(`error  ${e}`);
  console.log(`${files} prose snippets compiled, ${errors.length} failed`);
  process.exitCode = errors.length > 0 ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}

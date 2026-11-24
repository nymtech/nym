// Per-snippet compile check (D10, task 1.10): compile each prose snippet alone
// with Nextra's `compileMdx`. The round trip cannot see a cut through a JSX
// element, because the broken parts join back to a correct page; a compile of
// each part alone can.
//
//   node lib/snippets/compile.mts [--root <repo root>]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { compileMdx } from 'nextra/compile';
import { openRepo } from './repo.mts';

/** The fields of a compile error that the report reads: a VFileMessage gives `place` and `reason`. */
interface CompileError {
  place?: { line?: number };
  line?: number;
  reason?: string;
  message?: string;
}

/** Compile each `.mdx` under `dir`. */
export async function compileSnippets(dir: string, root: string): Promise<{ files: number; errors: string[] }> {
  const files: string[] = [];
  const walk = (d: string): void => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const full = path.join(d, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.name.endsWith('.mdx')) files.push(full);
    }
  };
  if (fs.existsSync(dir)) walk(dir);
  const errors: string[] = [];
  for (const file of files.sort()) {
    try {
      await compileMdx(fs.readFileSync(file, 'utf-8'), { filePath: file });
    } catch (thrown) {
      const error: CompileError = thrown instanceof Object ? thrown : { message: String(thrown) };
      const at = error.place?.line ?? error.line;
      errors.push(`${path.relative(root, file)}${at ? `:${at}` : ''}: ${error.reason ?? error.message}`);
    }
  }
  return { files: files.length, errors };
}

async function main(argv: string[]): Promise<void> {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const root = at('root');
  const repo = openRepo(root ? path.resolve(root) : undefined);
  const { files, errors } = await compileSnippets(repo.snippets, repo.root);
  for (const e of errors) console.log(`error  ${e}`);
  console.log(`${files} prose snippets compiled, ${errors.length} failed`);
  process.exitCode = errors.length > 0 ? 1 : 0;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}

// Source file collection for the code index: walk a producer's roots, skip build
// output and dependency trees, and cap file size.

import fs from 'node:fs';
import path from 'node:path';
import { langOf } from './code-chunker.ts';

export const DEFAULT_EXCLUDE =
  /(^|\/)(node_modules|target|dist|build|out|\.next|pkg|coverage|__pycache__)(\/|$)|\.d\.ts$/;

// No hand-written source approaches this (the largest is ~76KB), but gitignored
// build bundles that live under src/ (e.g. a 30MB rollup `src/worker/worker.js`)
// slip past the directory excludes, carry near-zero retrieval value, and would
// explode the index (one 30MB file is ~13k chunks). Skip anything above the cap.
export const MAX_FILE_BYTES = 256 * 1024;

export interface SourceFile {
  /** repo-relative, `/`-separated */
  path: string;
  content: string;
}

/**
 * The chunkable source files under `roots` (repo-relative) in `repoRoot`, in walk
 * order, and the repo-relative paths skipped for size. A root that does not exist
 * on this checkout is skipped.
 */
export function collectSourceFiles(
  repoRoot: string,
  roots: string[],
  { exclude = DEFAULT_EXCLUDE, maxBytes = MAX_FILE_BYTES }: { exclude?: RegExp; maxBytes?: number } = {},
): { files: SourceFile[]; oversized: string[] } {
  const found: string[] = [];
  const oversized: string[] = [];

  const walk = (dir: string) => {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return; // root may not exist on every checkout
    }
    for (const e of entries) {
      const full = path.join(dir, e.name);
      const rel = path.relative(repoRoot, full);
      if (exclude.test(rel)) continue;
      if (e.isDirectory()) walk(full);
      else if (langOf(e.name)) {
        if (fs.statSync(full).size > maxBytes) oversized.push(rel);
        else found.push(full);
      }
    }
  };
  for (const root of roots) walk(path.join(repoRoot, root));

  const files = found.map((full) => ({
    path: path.relative(repoRoot, full).split(path.sep).join('/'),
    content: fs.readFileSync(full, 'utf-8'),
  }));
  return { files, oversized };
}

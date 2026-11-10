// Source file collection for the code index: walk a producer's roots, skip build
// output and dependency trees, cap file size, and count the source files the
// chunker cannot read, so a language that indexes to zero is a recorded
// decision, not an accident (design decision 10).

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

// Extensions that look like source. A file with one of these that langOf()
// cannot chunk is counted as dropped.
export const SOURCE_LIKE = new Set([
  'kt', 'kts', 'swift', 'go', 'py', 'rb', 'java', 'scala', 'cs',
  'c', 'h', 'cpp', 'cc', 'cxx', 'hpp', 'hxx', 'm', 'mm', 'cjs', 'proto',
]);

// Source-like extensions dropped on purpose: in the producer repos these are
// generated bindings, a cgo shim, an FFI example, and a handful of Objective-C
// and proto files. Any other dropped source-like extension fails the build
// (assertNoSilentDrops). To index a language, give code-chunker a rule for it;
// to skip one on purpose, add its extension here.
export const ACCEPTED_DROP = new Set(['c', 'h', 'cc', 'cpp', 'cxx', 'hpp', 'hxx', 'm', 'mm', 'proto']);

export interface SourceFile {
  /** repo-relative, `/`-separated */
  path: string;
  content: string;
}

/**
 * The chunkable source files under `roots` (repo-relative) in `repoRoot`, in walk
 * order, the repo-relative paths skipped for size, and the dropped source-like
 * files by extension. A root that does not exist on this checkout is skipped.
 */
export function collectSourceFiles(
  repoRoot: string,
  roots: string[],
  { exclude = DEFAULT_EXCLUDE, maxBytes = MAX_FILE_BYTES }: { exclude?: RegExp; maxBytes?: number } = {},
): { files: SourceFile[]; oversized: string[]; dropped: Record<string, string[]> } {
  const found: string[] = [];
  const oversized: string[] = [];
  const dropped: Record<string, string[]> = {};

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
      } else {
        const ext = path.extname(e.name).slice(1).toLowerCase();
        if (SOURCE_LIKE.has(ext)) (dropped[ext] ??= []).push(rel);
      }
    }
  };
  for (const root of roots) walk(path.join(repoRoot, root));

  const files = found.map((full) => ({
    path: path.relative(repoRoot, full).split(path.sep).join('/'),
    content: fs.readFileSync(full, 'utf-8'),
  }));
  return { files, oversized, dropped };
}

/** Throw, naming each file, if any dropped extension is not on the accepted set. */
export function assertNoSilentDrops(dropped: Record<string, string[]>, accepted: Set<string> = ACCEPTED_DROP): void {
  const silent = Object.entries(dropped).filter(([ext]) => !accepted.has(ext));
  if (silent.length === 0) return;
  const detail = silent.map(([ext, files]) => `  .${ext} (${files.length}): ${files.join(', ')}`).join('\n');
  throw new Error(
    `source files the code chunker cannot read would index to zero:\n${detail}\n` +
      'Give code-chunker a rule for the language, or add the extension to ACCEPTED_DROP in walk.ts.',
  );
}

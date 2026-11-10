// Load a TypeScript data module from a plain-node build script.
//
// The threat-model pages render from typed data (lib/privacy-model/*.ts) rather
// than from prose, so the retrieval build has to read that data to see what a
// page actually says. The generators are plain ESM and cannot import .ts, and
// keeping a second copy of the data in .mjs would reintroduce exactly the drift
// the typed spine exists to prevent.
//
// `typescript` is already a dependency of this app, so transpiling costs no new
// install. Types are erased and nothing is type-checked: this is a loader, and
// the type checker still runs over the same files during `next build`.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import ts from 'typescript';

/**
 * Name of the throwaway twin for a .ts file. Dotted so it reads as disposable.
 * `load` is a random token for one loadTsModule call, so two loads at the same
 * time (test files in parallel, in processes or threads) do not write or delete
 * each other's twins. The unique name also makes each import a new module.
 */
function twinName(tsPath, load) {
  return `.${path.basename(tsPath, '.ts')}.${load}.retrieval.mjs`;
}


/**
 * Transpile `tsPath` and every .ts sibling it imports, writing each beside its
 * original. Returns the path of the root twin. Callers must delete everything in
 * `written` afterwards.
 *
 * Twins live beside the originals rather than in a temp directory because these
 * modules import their siblings by relative path, and a relative import needs a
 * directory to resolve against.
 */
function transpileTree(tsPath, written, load) {
  const resolved = path.resolve(tsPath);
  const out = path.join(path.dirname(resolved), twinName(resolved, load));
  if (written.has(out)) return out;
  written.set(out, resolved);

  const { outputText } = ts.transpileModule(fs.readFileSync(resolved, 'utf-8'), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
    fileName: resolved,
  });

  const dir = path.dirname(resolved);
  const deps = [];

  // Emitted relative imports point at extensionless .ts siblings, which node
  // cannot resolve. Repoint each at its own twin and remember to build it.
  const rewritten = outputText.replace(
    /(\bfrom\s+["'])(\.\.?\/[^"']+)(["'])/g,
    (whole, pre, spec, post) => {
      if (/\.(mjs|js|json)$/.test(spec)) return whole;
      const target = path.resolve(dir, spec + '.ts');
      if (!fs.existsSync(target)) return whole;
      deps.push(target);
      const rel = path.relative(dir, path.join(path.dirname(target), twinName(target, load)));
      // A twin's name starts with a dot, so test for a path segment, not a dot.
      return pre + (/^\.\.?\//.test(rel) ? rel : './' + rel) + post;
    },
  );

  fs.writeFileSync(out, rewritten);
  for (const dep of deps) transpileTree(dep, written, load);
  return out;
}

/**
 * Transpile a .ts data module and import it, returning its exports.
 *
 * Every twin written for the import is removed afterwards, including on failure,
 * so an interrupted build cannot leave a stale copy that later shadows the real
 * module.
 */
export async function loadTsModule(tsPath) {
  const written = new Map();
  const load = crypto.randomUUID();
  try {
    const root = transpileTree(tsPath, written, load);
    return await import(pathToFileURL(root).href);
  } finally {
    for (const f of written.keys()) fs.rmSync(f, { force: true });
  }
}

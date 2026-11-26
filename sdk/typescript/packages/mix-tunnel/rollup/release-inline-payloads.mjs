// The worker and the smolmix-wasm bytes are inlined as base64 string literals,
// and the code that decodes each literal keeps it reachable for the life of
// the worker: rollup-plugin-web-worker-loader's factory closes over the
// worker's base64 (17 MB), and @rollup/plugin-wasm's getter holds the wasm's
// (13 MB) in its bytecode. Each is decoded once, so these plugins drop the
// reference after first use and the strings can be garbage-collected. Each
// rewrite fails the build if the generated code no longer matches.

const rewriteOnce = (ctx, code, from, to, what) => {
  const at = code.indexOf(from);
  if (at < 0 || code.indexOf(from, at + 1) >= 0) {
    ctx.error(`${what}: expected exactly one \`${from}\` in the generated code`);
  }
  return code.slice(0, at) + to + code.slice(at + from.length);
};

/** Drop the worker factory's base64 once the worker's blob URL exists. */
export const releaseInlineWorker = () => ({
  name: 'release-inline-worker',
  renderChunk(code) {
    const from = 'url = url || createURL(base64, sourcemapArg, enableUnicodeArg);';
    return { code: rewriteOnce(this, code, from, `${from} base64 = null;`, 'release-inline-worker'), map: null };
  },
});

/** Move the inlined wasm into a closure that the getter drops after its one call. */
export const releaseInlineWasm = () => ({
  name: 'release-inline-wasm',
  transform(code, id) {
    if (!id.endsWith('.wasm')) return null;
    // The whole module @rollup/plugin-wasm generates for an inlined file.
    const match =
      /^(import \{ _loadWasmModule \} from "[^"\n]+";\n)export default function\(imports\)\{return _loadWasmModule\(0, null, ('[A-Za-z0-9+/=]*'), imports\)\}$/.exec(
        code,
      );
    if (!match) this.error(`release-inline-wasm: unexpected @rollup/plugin-wasm output for ${id}`);
    const [, helperImport, literal] = match;
    const getter =
      'export default function(imports){const bytes = take; take = null; return _loadWasmModule(0, null, bytes(), imports)}';
    return { code: `${helperImport}let take = function(){return ${literal}};\n${getter}`, map: null };
  },
});

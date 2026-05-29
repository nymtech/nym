import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import { wasm } from '@rollup/plugin-wasm';
import replace from '@rollup/plugin-replace';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

/**
 * Worker bundle. `@rollup/plugin-wasm` inlines the smolmix-wasm bytes; the
 * worker is then base64-inlined into the main ESM bundle by rollup/esm.mjs.
 */
export const getConfig = () => ({
  input: 'src/worker/index.ts',
  output: {
    dir: 'dist',
    format: 'es',
  },
  plugins: [
    resolve({ extensions }),
    // smolmix-wasm's wasm-pack glue auto-generates a `new URL(...)` reference
    // for the .wasm at the path the bundler is expected to resolve. The wasm
    // plugin injects bytes directly via `init(bytes)`, so we null out the URL
    // assignment to keep the runtime from trying to fetch a file we don't ship.
    // NB: the variable name (`module_or_path`) matches current wasm-bindgen
    // output; older versions called it `input` — check pkg/smolmix_wasm.js if
    // the replace stops matching after a wasm-pack upgrade.
    replace({
      values: {
        "module_or_path = new URL('smolmix_wasm_bg.wasm', import.meta.url);": 'module_or_path = undefined;',
      },
      delimiters: ['', ''],
      preventAssignment: true,
    }),
    // Set maxFileSize above the smolmix-wasm size so the plugin inlines the
    // bytes as base64 rather than emitting a sibling .wasm asset. The cost is
    // a fat bundle (~28 MB for the worker, base64 overhead); the benefit is
    // zero-config consumer deployment — no need to teach every downstream
    // bundler how to copy the .wasm next to the JS.
    wasm({ maxFileSize: 50_000_000, targetEnv: 'browser' }),
    typescript({
      compilerOptions: {
        declaration: false,
        target: 'es2020',
      },
    }),
  ],
});

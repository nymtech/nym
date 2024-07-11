import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import { wasm } from '@rollup/plugin-wasm';
import replace from '@rollup/plugin-replace';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

/**
 * Configure worker output
 *
 * @param opts
 *      `format`: `es` or `cjs`,
 *      `inlineWasm`: true or false,
 *      `tsTarget`: `es5` or `es6`
 */
export const getConfig = (input, wasmFilename, opts) => ({
  input,
  output: {
    dir: 'dist',
    format: opts?.format || 'es',
  },
  plugins: [
    resolve({ extensions }),
    // this is some nasty monkey patching that removes the WASM URL (because it is handled by the `wasm` plugin)
    replace({
      values: { [`input = new URL('${wasmFilename}', import.meta.url);`]: 'input = undefined;' },
      delimiters: ['', ''],
      preventAssignment: true,
    }),
    opts?.inlineWasm === true
      ? wasm({ maxFileSize: 10_000_000, targetEnv: 'browser' }) // force the wasm plugin to embed the wasm bundle - this means no downstream bundlers have to worry about handling it
      : wasm({
          targetEnv: 'browser',
          fileName: '[name].wasm',
        }),
    typescript({
      compilerOptions: {
        declaration: false,
        target: opts?.tsTarget || 'es6',
      },
    }),
  ],
});

import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import { wasm } from '@rollup/plugin-wasm';
import replace from '@rollup/plugin-replace';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

export default {
  input: 'src/worker/index.ts',
  output: {
    dir: 'dist',
    format: process.env.MIX_FETCH_BUNDLE_INLINE_WORKER === true ? 'cjs' : 'es',
    // format: 'cjs',
  },
  plugins: [
    resolve({ extensions }),
    // this is some nasty monkey patching that removes the WASM URL (because it is handled by the `wasm` plugin)
    replace({
      values: { "input = new URL('mix_fetch_wasm_bg.wasm', import.meta.url);": 'input = undefined;' },
      delimiters: ['', ''],
      preventAssignment: true,
    }),
    wasm({
      targetEnv: 'browser',
      fileName: '[name].wasm',
      // force the wasm plugin to embed the wasm bundle - this means no downstream bundlers have to worry about handling it
      maxFileSize: process.env.MIX_FETCH_BUNDLE_INLINE_WASM === 'true' ? 10000000 : undefined,
    }),
    typescript({
      compilerOptions: {
        declaration: false,
        target: process.env.MIX_FETCH_BUNDLE_INLINE_WORKER === true ? 'es5' : 'es6',
        // target: 'es5',
      },
    }),
  ],
};

import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import { wasm } from '@rollup/plugin-wasm';
import replace from '@rollup/plugin-replace';

const extensions = [
  '.js', '.jsx', '.ts', '.tsx',
];

export default {
  input: 'src/index.ts',
  output: {
    dir: 'dist/cjs',
    format: 'cjs',
  },
  plugins: [
    resolve({ extensions }),
    // this is some nasty monkey patching that removes the WASM URL (because it is handled by the `wasm` plugin)
    replace({
      values: { 'input = new URL(\'nym_client_wasm_bg.wasm\', import.meta.url);': 'input = undefined;' },
      delimiters: ['', ''],
      preventAssignment: true,
    }),
    // force the wasm plugin to embed the wasm bundle - this means no downstream bundlers have to worry about handling it
    wasm({ maxFileSize: 10000000, targetEnv: 'browser' }),
    typescript({ compilerOptions: { outDir: 'dist/cjs', declaration: false } }),
  ],
};

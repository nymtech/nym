/* eslint-disable import/no-extraneous-dependencies */
import replace from '@rollup/plugin-replace';
import resolve from '@rollup/plugin-node-resolve';
import typescript from '@rollup/plugin-typescript';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';
import { wasm } from '@rollup/plugin-wasm';

export default {
  input: 'src/index.ts',
  output: {
    dir: 'dist/cjs',
    format: 'cjs',
  },
  plugins: [
    webWorkerLoader({ targetPlatform: 'node', inline: false }),
    replace({
      values: {
        "createURLWorkerFactory('web-worker-0.js')":
          "createURLWorkerFactory(require('path').resolve(__dirname, 'web-worker-0.js'))",
      },
      delimiters: ['', ''],
      preventAssignment: true,
    }),
    resolve({ browser: false, extensions: ['.js', '.ts'] }),
    wasm({ targetEnv: 'node', maxFileSize: 0 }),
    typescript({
      compilerOptions: { outDir: 'dist/cjs', target: 'es5' },
      exclude: ['src/worker.ts'],
    }),
  ],
};

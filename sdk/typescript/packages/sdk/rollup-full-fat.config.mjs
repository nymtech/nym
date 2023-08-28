// This is the rollup config for the full-fat SDK package.
// The config is similar to the esm config, but exports web workers as separate files.
// This can be necessary for implentations that do not support inline web workers.

import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

export default {
  input: 'src/index.ts',
  output: {
    dir: 'dist/full-fat',
    format: 'es',
  },
  plugins: [
    webWorkerLoader({
      targetPlatform: 'browser',
      inline: false,
    }),
    resolve({ extensions }),
    typescript({
      exclude: ['mixnet/wasm/worker.ts', 'mixnet/node-tester/worker.ts'],
      compilerOptions: { outDir: 'dist/full-fat' },
    }),
  ],
};

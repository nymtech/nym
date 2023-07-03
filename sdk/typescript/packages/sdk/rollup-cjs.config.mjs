import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import { wasm } from '@rollup/plugin-wasm';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

export default {
  input: 'src/index.ts',
  output: {
    dir: 'dist/cjs',
    format: 'cjs',
  },
  plugins: [
    webWorkerLoader({ targetPlatform: 'browser', inline: true }),
    resolve({ extensions }),
    wasm({ maxFileSize: 10000000, targetEnv: 'browser' }),
    typescript({
      compilerOptions: { outDir: 'dist/cjs', target: 'es5' },
      exclude: ['mixnet/wasm/worker.ts', 'mixnet/node-tester/worker.ts'],
    }),
  ],
};

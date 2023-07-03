import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

export default {
  input: 'src/index.ts',
  output: {
    dir: 'dist/esm',
    format: 'es',
  },
  plugins: [
    webWorkerLoader({ targetPlatform: 'browser', inline: true }),
    resolve({ extensions }),
    typescript({
      exclude: ['mixnet/wasm/worker.ts', 'mixnet/node-tester/worker.ts'],
      compilerOptions: { outDir: 'dist/esm' },
    }),
  ],
};

import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';
import replace from '@rollup/plugin-replace';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

/**
 * ESM bundle. The worker is inlined as a base64 blob; the worker bundle itself
 * carries the smolmix-wasm bytes (see rollup/worker.mjs).
 */
export const getConfig = (opts = {}) => ({
  input: 'src/index.ts',
  output: {
    dir: opts.outputDir || 'dist/esm',
    format: 'es',
  },
  plugins: [
    webWorkerLoader({ targetPlatform: 'browser', inline: true }),
    replace({
      values: {
        "createURLWorkerFactory('web-worker-0.js')":
          "createURLWorkerFactory(new URL('web-worker-0.js', import.meta.url))",
      },
      delimiters: ['', ''],
      preventAssignment: true,
    }),
    resolve({ extensions }),
    typescript({
      exclude: ['worker/*'],
      compilerOptions: { outDir: opts.outputDir || 'dist/esm' },
    }),
  ],
});

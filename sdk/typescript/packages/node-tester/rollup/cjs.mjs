import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import { wasm } from '@rollup/plugin-wasm';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

/**
 * Gets the config for bundling the package as a CommonJS module.
 *
 * @param opts Options:
 *    `{ inline: boolean }` - set inline to true to inline the web worker in the main bundle
 *    `{ outputDir: string }` - override the destination
 */
export const getConfig = (opts) => ({
  input: 'src/index.ts',
  output: {
    dir: opts.outputDir || 'dist/cjs',
    format: 'cjs',
  },
  plugins: [
    webWorkerLoader({ targetPlatform: 'browser', inline: opts.inline }), // the inline param is used here
    resolve({ extensions }),
    wasm({ maxFileSize: 10000000, targetEnv: 'browser' }),
    typescript({
      compilerOptions: { outDir: opts.outputDir || 'dist/cjs', target: 'es5' },
      exclude: ['worker.ts'],
    }),
  ],
});

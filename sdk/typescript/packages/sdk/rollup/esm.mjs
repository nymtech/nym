import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';

const extensions = ['.js', '.jsx', '.ts', '.tsx'];

/**
 * Gets the config for bundling the package as an ES Module.
 *
 * @param opts Options:
 *    `{ inline: boolean }` - set inline to true to inline the web worker in the main bundle
 *    `{ outputDir: string }` - override the destination *
 */
export const getConfig = (opts) => ({
  input: 'src/index.ts',
  output: {
    dir: opts.outputDir || 'dist/esm',
    format: 'es',
  },
  plugins: [
    webWorkerLoader({ targetPlatform: 'browser', inline: opts.inline }), // the inline param is used here
    resolve({ extensions }),
    typescript({
      exclude: ['mixnet/wasm/worker.ts', 'zk-nym/worker.ts'],
      compilerOptions: { outDir: opts.outputDir || 'dist/esm' },
    }),
  ],
});

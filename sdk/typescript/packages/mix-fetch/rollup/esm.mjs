import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import webWorkerLoader from 'rollup-plugin-web-worker-loader';
import replace from '@rollup/plugin-replace';

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
    replace({
      // when loading the web worker as a full ES module, tell pass `new Worker({ type: 'module'})` to tell
      // the browser to load and allow imports inside the worker code. Also load as a URL so relative paths work.
      // values: opts.inline
      //   ? undefined
      //   : {
      //       "createURLWorkerFactory('web-worker-0.js')":
      //         "createURLWorkerFactory(new URL('web-worker-0.js', import.meta.url))",
      //     },
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

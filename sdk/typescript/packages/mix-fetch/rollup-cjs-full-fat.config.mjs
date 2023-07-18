import { getConfig } from './rollup/cjs.mjs';

export default {
  ...getConfig({
    inline: true,
    outputDir: 'dist/cjs-full-fat',
  }),
};

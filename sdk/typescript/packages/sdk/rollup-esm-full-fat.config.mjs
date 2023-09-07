import { getConfig } from './rollup/esm.mjs';

export default {
  ...getConfig({
    inline: true,
    outputDir: 'dist/esm-full-fat',
  }),
};

import { getConfig } from './rollup/worker.mjs';

export default {
  ...getConfig({
    inlineWasm: true,
    format: 'cjs',
  }),
};

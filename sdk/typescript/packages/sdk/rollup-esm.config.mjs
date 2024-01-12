import { getConfig } from './rollup/esm.mjs';

export default {
  ...getConfig({
    inline: process.env.SDK_DEV_MODE === 'true',
  }),
};

import { getConfig } from './rollup/esm.mjs';

export default {
  ...getConfig({
    // by default, the web worker will not be inlined, in local development mode it will be
    inline: process.env.MIX_FETCH_DEV_MODE === 'true',
  }),
};

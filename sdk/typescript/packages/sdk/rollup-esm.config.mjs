import { getConfig } from './rollup/esm.mjs';

export default {
  ...getConfig({
    inline: false,
  }),
};

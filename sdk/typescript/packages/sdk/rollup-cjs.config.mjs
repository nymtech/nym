import { getConfig } from './rollup/cjs.mjs';

export default {
  ...getConfig({ inline: false }),
};

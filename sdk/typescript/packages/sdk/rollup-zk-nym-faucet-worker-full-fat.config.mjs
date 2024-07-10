import { getConfig } from './rollup/worker.mjs';

export default {
  ...getConfig('src/zk-nym-faucet/worker.ts', 'zk_nym_faucet_lib_bg.wasm'),
  inlineWasm: true,
  format: 'cjs',
};

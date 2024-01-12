import { getConfig } from './rollup/worker.mjs';

export default {
  ...getConfig('src/mixnet/wasm/worker.ts', 'nym_client_wasm_bg.wasm', {
    inlineWasm: true,
    format: 'cjs',
  }),
};

import { getConfig } from './rollup/worker.mjs';

export default {
  ...getConfig('src/zk-nym/worker.ts', 'nym_credential_client_wasm_bg.wasm'),
  inlineWasm: true,
  format: 'cjs',
};

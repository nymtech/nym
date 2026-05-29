/// <reference types="@nymproject/smolmix-wasm" />
/* eslint-disable import/no-extraneous-dependencies */

// Rollup's @rollup/plugin-wasm rewrites this import to a function returning
// the smolmix-wasm bytes (base64-inlined into this worker bundle at build time).
// @ts-ignore - resolved by @rollup/plugin-wasm
import getSmolmixWasmBytes from '@nymproject/smolmix-wasm/smolmix_wasm_bg.wasm';
import init, { main } from '@nymproject/smolmix-wasm';

export async function loadWasm() {
  const bytes = await getSmolmixWasmBytes();
  await init(bytes);
  main();
}

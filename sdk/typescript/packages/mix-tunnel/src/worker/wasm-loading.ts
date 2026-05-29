/* eslint-disable @typescript-eslint/naming-convention, no-restricted-globals, import/no-extraneous-dependencies */
/// <reference types="@nymproject/smolmix-wasm" />

// Rollup will replace the wasmBytes import with a base64-inlined or
// fetch-by-URL accessor (decided by rollup-plugin-wasm at build time).
//
// @ts-ignore - resolved by @rollup/plugin-wasm
import getSmolmixWasmBytes from '@nymproject/smolmix-wasm/smolmix_wasm_bg.wasm';

import init, { main as wasmMain } from '@nymproject/smolmix-wasm';

export async function loadWasm() {
  const bytes = await getSmolmixWasmBytes();
  await init(bytes);
  wasmMain();
}

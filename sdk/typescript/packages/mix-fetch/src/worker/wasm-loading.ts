/* eslint-disable @typescript-eslint/naming-convention,no-restricted-globals */
/// <reference types="@nymproject/mix-fetch-wasm" />

// Copyright 2020-2023 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//
// Rollup will replace wasmBytes with a function that loads the WASM bundle from a base64 string embedded in the output.
//
// Doing it this way, saves having to support a large variety of bundlers and their quirks.
//
// @ts-ignore
// eslint-disable-next-line import/no-extraneous-dependencies
import getMixFetchWasmBytes from '@nymproject/mix-fetch-wasm/mix_fetch_wasm_bg.wasm';

// @ts-ignore
// eslint-disable-next-line import/no-extraneous-dependencies
import getGoConnectionWasmBytes from '@nymproject/mix-fetch-wasm/go_conn.wasm';

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
import init, {
  set_panic_hook,
  send_client_data,
  start_new_mixnet_connection,
  mix_fetch_initialised,
  finish_mixnet_connection,
} from '@nymproject/mix-fetch-wasm';

// see `typings/wasm_exec.d.ts` for the defintion of the `class Go` in global scope
import '@nymproject/mix-fetch-wasm/wasm_exec';

async function loadGoWasm() {
  // rollup will provide a function to get the Go connection WASM bytes here
  const bytes = await getGoConnectionWasmBytes();

  // @ts-ignore
  const go = new Go(); // Defined in wasm_exec.js

  // the WebAssembly runtime will parse the bytes and then start the Go runtime
  const wasmObj = await WebAssembly.instantiate(bytes, go.importObject);
  go.run(wasmObj);
}

function setupRsGoBridge() {
  const rsGoBridge = {
    send_client_data,
    start_new_mixnet_connection,
    mix_fetch_initialised,
    finish_mixnet_connection,
  };

  // (note: reason for intermediate `__rs_go_bridge__` object is to decrease global scope bloat
  // and to discourage users from trying to call those methods directly)
  // eslint-disable-next-line no-underscore-dangle
  (self as any).__rs_go_bridge__ = rsGoBridge;
}

export async function loadWasm() {
  // rollup with provide a function to get the mixFetch WASM bytes
  const bytes = await getMixFetchWasmBytes();

  // load rust WASM package
  await init(bytes);
  console.log('Loaded RUST WASM');

  // load go WASM package
  await loadGoWasm();
  console.log('Loaded GO WASM');

  // sets up better stack traces in case of in-rust panics
  set_panic_hook();

  setupRsGoBridge();

  // goWasmSetLogging('trace');
}

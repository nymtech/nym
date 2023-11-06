/* eslint-disable no-console */
/// <reference types="@nymproject/mix-fetch-wasm-node" />

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

import '@nymproject/mix-fetch-wasm-node/mix_fetch_wasm_bg.wasm';

// @ts-ignore
import getGoConnectionWasmBytes from '@nymproject/mix-fetch-wasm-node/go_conn.wasm';

import {
  send_client_data,
  start_new_mixnet_connection,
  mix_fetch_initialised,
  finish_mixnet_connection,
  set_panic_hook,
} from '@nymproject/mix-fetch-wasm-node';

export async function loadGoWasm() {
  // rollup will provide a function to get the Go connection WASM bytes here
  const bytes = await getGoConnectionWasmBytes();

  const go = new Go(); // Defined in wasm_exec.js

  // the WebAssembly runtime will parse the bytes and then start the Go runtime
  const wasmObj = await WebAssembly.instantiate(bytes, go.importObject);
  // eslint-disable-next-line no-console
  console.log('Loaded GO WASM');

  go.run(wasmObj);
}

function setupRsGoBridge() {
  const rsGoBridge = {
    send_client_data,
    start_new_mixnet_connection,
    mix_fetch_initialised,
    finish_mixnet_connection,
  };

  // and to discourage users from trying to call those methods directly)
  // @ts-expect-error globalThis has index signature of any
  // eslint-disable-next-line no-underscore-dangle
  globalThis.__rs_go_bridge__ = rsGoBridge;
}

export async function loadWasm() {
  // load go WASM package
  await loadGoWasm();

  console.log('Loaded GO WASM');

  // sets up better stack traces in case of in-rust panics
  set_panic_hook();

  setupRsGoBridge();
}

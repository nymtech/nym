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

import * as Comlink from 'comlink';

//
// Rollup will replace wasmBytes with a function that loads the WASM bundle from a base64 string embedded in the output.
//
// Doing it this way, saves having to support a large variety of bundlers and their quirks.
//
// @ts-ignore
// eslint-disable-next-line import/no-extraneous-dependencies
import wasmBytes from '@nymproject/nym-client-wasm/nym_client_wasm_bg.wasm';

/* eslint-disable no-restricted-globals */
import init, {
  NymNodeTester,,
  current_network_topology,
  NodeTestResult,
} from '@nymproject/nym-client-wasm';
import { Network, NodeTestEvent } from './types';
import { MAINNET_VALIDATOR_URL, QA_VALIDATOR_URL } from './constants';

console.log('[Nym WASM client] Starting Nym WASM web worker...');

class ClientWrapper {
  client?: NymNodeTester;

  constructor(validatorUrl: string) {
    this.buildTester(validatorUrl)
      .then(() => {
        console.log('Built tester');
      })
      .catch((err: any) => {
        console.error(err);
      });
  }

  buildTester = async (validatorUrl: string) => {
    const topology = await current_network_topology(validatorUrl);
    const nodeTester = await new NymNodeTester(topology, validatorUrl);

    this.client = nodeTester;
  };

  start = (mixnodeId: string) => {
    if (!this.client) {
      console.error('Client has not been initialised');
      return undefined;
    }

    const result: unknown = this.client.test_node(mixnodeId);
    return result as NodeTestResult;
  };
}

init(wasmBytes()).then((importResult: any) => {
  importResult.set_panic_hook();
  const wrapper = new ClientWrapper(MAINNET_VALIDATOR_URL);
  // implement the public logic of this web worker (message exchange between the worker and caller is done by https://www.npmjs.com/package/comlink)
  const webWorker = {
    startTest(mixnodeId: string) {
      return wrapper.start(mixnodeId);
    },
  };

  // start comlink listening for messages and handle them above
  Comlink.expose(webWorker);
});

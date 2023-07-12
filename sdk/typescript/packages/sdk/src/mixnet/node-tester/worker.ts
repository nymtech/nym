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
import wasmBytes from '@nymproject/nym-node-tester-wasm/nym_node_tester_wasm_bg.wasm';

/* eslint-disable no-restricted-globals */
import init, { NymNodeTester, NodeTestResult } from '@nymproject/nym-node-tester-wasm/nym_node_tester_wasm';
import type { INodeTesterWorkerAsync, NodeTesterLoadedEvent } from './types';
import { NodeTesterEventKinds } from './types';

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
// eslint-disable-next-line no-restricted-globals
const postMessageWithType = <E>(event: E) => self.postMessage(event);

console.log('[Nym WASM client] Starting Nym WASM web worker...');

const buildTester = async (validatorUrl: string, nodeTesterId?: string): Promise<NymNodeTester> =>
  new NymNodeTester({ nymApi: validatorUrl, id: nodeTesterId });

async function main() {
  const importResult = await init(wasmBytes());
  importResult.set_panic_hook();

  let nodeTester: NymNodeTester | null = null;

  const webWorker: INodeTesterWorkerAsync = {
    async init(validatorUrl: string, nodeTesterId?: string) {
      nodeTester = await buildTester(validatorUrl, nodeTesterId);
    },
    async reconnectToGateway() {
      if (!nodeTester) {
        throw Error('Please run init first');
      }
      await nodeTester.reconnect_to_gateway();
    },
    async disconnectFromGateway() {
      if (!nodeTester) {
        throw Error('Please run init first');
      }
      await nodeTester.disconnect_from_gateway();
    },
    async startTest(mixnodeIdentityKey: string) {
      if (!nodeTester) {
        throw Error('Please run init first');
      }
      console.log(`Testing mixnode with identity key = ${mixnodeIdentityKey}`);
      // TODO: fix typing in Rust code
      const result = (await nodeTester.test_node(mixnodeIdentityKey)) as NodeTestResult | undefined;

      // return early if there was an error
      if (!result) {
        return result;
      }

      // log the result in the worker so that the packet stats are visible somewhere and extract the score
      result.log_details();

      // eslint-disable-next-line @typescript-eslint/naming-convention
      const { duplicate_acks, duplicate_packets, received_acks, received_packets, sent_packets } = result;

      // construct the response to avoid any weird proxy effects
      return {
        score: result.score(),
        sentPackets: sent_packets,
        receivedPackets: received_packets,
        receivedAcks: received_acks,
        duplicatePackets: duplicate_packets,
        duplicateAcks: duplicate_acks,
      };
    },
  };

  // start comlink listening for messages and handle them above
  Comlink.expose(webWorker);

  // notify any listeners that the web worker has loaded and is ready for testing
  postMessageWithType<NodeTesterLoadedEvent>({ kind: NodeTesterEventKinds.Loaded, args: { loaded: true } });
}

main();

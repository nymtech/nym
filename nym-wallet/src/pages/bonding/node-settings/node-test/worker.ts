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

/* eslint-disable no-restricted-globals */
import { NymNodeTester, set_panic_hook, current_network_topology, NodeTestResult } from '@nymproject/nym-client-wasm';
import { Network } from 'src/types';
import { MAINNET_VALIDATOR_URL, QA_VALIDATOR_URL } from 'src/constants';
import { NodeTestEvent } from './types';

console.log('Initializing worker');

const postMessage = (data: NodeTestEvent) => self.postMessage(data);

postMessage({
  kind: 'WorkerLoaded',
});

const printAndDisplayTestResult = (result: NodeTestResult) => {
  result.log_details();

  postMessage({
    kind: 'DisplayTesterResults',
    args: {
      result: {
        score: result.score(),
        sentPackets: result.sent_packets,
        receivedPackets: result.received_packets,
        receivedAcks: result.received_acks,
        duplicatePackets: result.duplicate_packets,
        duplicateAcks: result.duplicate_acks,
      },
    },
  });
};

const buildTester = async (network: Network) => {
  const validator = network === 'QA' ? QA_VALIDATOR_URL : MAINNET_VALIDATOR_URL;
  const topology = await current_network_topology(validator);
  const nodeTester = await new NymNodeTester(topology, network);

  return nodeTester;
};

async function testNode() {
  self.onmessage = async (event: MessageEvent<NodeTestEvent>) => {
    const eventKind = event.data.kind;

    switch (eventKind) {
      case 'TestPacket': {
        const { mixnodeIdentity, network } = event.data.args;
        const nodeTester = await buildTester(network);

        try {
          console.log(`Testing mixnode identity: ${mixnodeIdentity}, on network: ${network}.`);
          const result = await nodeTester.test_node(mixnodeIdentity);

          printAndDisplayTestResult(result);

          await nodeTester.disconnect_from_gateway();
          console.log('Disconnected from gateway');
        } catch (e) {
          const errorMessage = e instanceof Error ? e.message : 'Node test error';
          console.log(errorMessage);

          nodeTester.disconnect_from_gateway();

          postMessage({
            kind: 'Error',
            args: { message: errorMessage },
          });
        }
        break;
      }
      default:
        return undefined;
    }

    return undefined;
  };
}

async function main() {
  // sets up better stack traces in case of in-rust panics
  set_panic_hook();

  // run test on simplified and dedicated tester:
  await testNode();
}

// Let's get started!
main();

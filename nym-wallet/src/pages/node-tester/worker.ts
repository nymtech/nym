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

console.log('Initializing worker');

import {
  NymNodeTester,
  WasmGateway,
  WasmMixNode,
  WasmNymTopology,
  set_panic_hook,
  current_network_topology,
} from 'nym-client-wasm';

self.postMessage({
  kind: 'Worker loaded',
});

function dummyTopology() {
  const l1Mixnode = new WasmMixNode(
    1,
    'n1fzv4jc7fanl9s0qj02ge2ezk3kts545kjtek47',
    '178.79.143.65',
    1789,
    '4Yr4qmEHd9sgsuQ83191FR2hD88RfsbMmB4tzhhZWriz',
    '8ndjk5oZ6HxUZNScLJJ7hk39XtUqGexdKgW7hSX6kpWG',
    1,
    '1.10.0',
  );

  const l2Mixnode = new WasmMixNode(
    2,
    'n1z93z44vf8ssvdhujjvxcj4rd5e3lz0l60wdk70',
    '109.74.197.180',
    1789,
    '7sVjiMrPYZrDWRujku9QLxgE8noT7NTgBAqizCsu7AoK',
    'GepXwRnKZDd8x2nBWAajGGBVvF3mrpVMQBkgfrGuqRCN',
    2,
    '1.10.0',
  );

  const l3Mixnode = new WasmMixNode(
    3,
    'n1ptg680vnmef2cd8l0s9uyc4f0hgf3x8sed6w77',
    '176.58.101.80',
    1789,
    'FoM5Mx9Pxk1g3zEqkS3APgtBeTtTo3M8k7Yu4bV6kK1R',
    'DeYjrDC2AcQRVFshiKnbUo6bRvPyZ33QGYR2DLeFJ9qD',
    3,
    '1.10.0',
  );

  const gateway = new WasmGateway(
    'n16evnn8glr0sham3matj8rg2s24m6x56ayk87ts',
    '85.159.212.96',
    1789,
    9000,
    '336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9',
    'BtYjoWihiuFihGKQypmpSspbhmWDPxzqeTVSd8ciCpWL',
    '1.10.1',
  );

  const mixnodes = new Map();
  mixnodes.set(1, [l1Mixnode]);
  mixnodes.set(2, [l2Mixnode]);
  mixnodes.set(3, [l3Mixnode]);

  const gateways = [gateway];

  return new WasmNymTopology(mixnodes, gateways);
}

function printAndDisplayTestResult(result: any) {
  result.log_details();

  self.postMessage({
    kind: 'DisplayTesterResults',
    args: {
      score: result.score(),
      sentPackets: result.sent_packets,
      receivedPackets: result.received_packets,
      receivedAcks: result.received_acks,
      duplicatePackets: result.duplicate_packets,
      duplicateAcks: result.duplicate_acks,
    },
  });
}

async function testWithTester() {
  try {
    // A) construct with hardcoded topology
    const preferredGateway = '3B7PsbXFuqq6rerYFLw5HPbQb4UmBqAhfWURRovMmWoj';
    // const topology = dummyTopology();
    // const nodeTester = await new NymNodeTester(topology, preferredGateway);

    // B) first get topology directly from nym-api
    const validator = 'https://validator.nymtech.net/api/';
    const topology = await current_network_topology(validator);
    const nodeTester = await new NymNodeTester(topology, preferredGateway);
    //
    // C) use nym-api in the constructor (note: it does no filtering for 'good' nodes on other layers)
    // const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    // const nodeTester = await NymNodeTester.new_with_api(validator, preferredGateway)

    // D, E, F) you also don't have to specify the gateway. if you don't, a random one (from your topology) will be used
    // const topology = dummyTopology()
    // const nodeTester = await new NymNodeTester(topology);

    self.onmessage = async (event) => {
      if (event.data && event.data.kind) {
        console.log(event);

        switch (event.data.kind) {
          case 'TestPacket': {
            const { mixnodeIdentity } = event.data.args;
            console.log('starting node test...');

            let result = await nodeTester.test_node(mixnodeIdentity);
            printAndDisplayTestResult(result);
          }
        }
      }
    };
  } catch (e) {
    const errorMessage = 'Node test error';
    console.log(errorMessage, e);
    self.postMessage({
      kind: 'Error',
      args: { message: e instanceof Error ? e.message : errorMessage },
    });
  }
}

async function main() {
  // load WASM package

  console.log('Loaded WASM');

  // sets up better stack traces in case of in-rust panics
  set_panic_hook();

  // run test on simplified and dedicated tester:
  await testWithTester();
}

// Let's get started!
main();

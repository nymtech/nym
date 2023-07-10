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

const RUST_WASM_URL = "nym_node_tester_wasm_bg.wasm"

importScripts('nym_node_tester_wasm.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    WasmGateway,
    WasmMixNode,
    WasmNymTopology,
    default_debug,
    no_cover_debug,
    NymNodeTester,
    set_panic_hook,
} = wasm_bindgen;

let client = null;

function dummyTopology() {
    const l1Mixnode = new WasmMixNode(
        1,
        'n1lftjhnl35cjsfd533zhgrwrspx6qmumd8vjgp9',
        '80.85.86.75',
        1789,
        '91mNjhJSBkJ9Lb6f1iuYMDQPLiX3kAv6paSUCWjGRwQz',
        'DmfN1mL1T95nPXvLK44AQKCpW1pStHNQCi6Fgpz5dxDV',
        1,
        '1.1.20',
    );
    const l2Mixnode = new WasmMixNode(
        2,
        'n18ztkyh20gwzrel0e5m4sahd358fq9p4skwa7d3',
        '139.162.199.75',
        1789,
        'BkLhuKQNyPS19sHZ3HHKCTKwK7hCU6XiFLndyZZHiB7s',
        '7KGC97tJRhJZKhDqFcsp4Vu715VVxizuD7BktnzuSmZC',
        2,
        '1.1.20',
    );
    const l3Mixnode = new WasmMixNode(
        3,
        'n1njq8h4nndp7ngays5el2rdp22hq67lwqcaq3ph',
        '139.162.244.139',
        1789,
        'EPja9Kv8JtPHsFbzPdBQierMu5GmQy5roE5njyD6dmND',
        'HWpsZChDrtEH8XNscW3qJMRzdCfUD8N8DmMcKqFv7tcf',
        3,
        '1.1.20',
    );

    const gateway = new WasmGateway(
        'n13n48znq3v2fu4nwx95vfcyf68zfsad7py2jz4m',
        '85.159.211.99',
        1789,
        9000,
        '6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM',
        '2V9uwPG2YPogX1BR5WXQGFrYzrAnUpnD3aSFyeZepdTp',
        '1.1.19',
    );

    const mixnodes = new Map();
    mixnodes.set(1, [l1Mixnode]);
    mixnodes.set(2, [l2Mixnode]);
    mixnodes.set(3, [l3Mixnode]);


    const gateways = [gateway];

    return new WasmNymTopology(mixnodes, gateways)
}

function printAndDisplayTestResult(result) {
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
    const preferredGateway = "6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";

    // A) construct with hardcoded topology
    const topology = dummyTopology()
    const nodeTester = await new NymNodeTester(topology, undefined, preferredGateway);

    // B) first get topology directly from nym-api
    // const validator = 'https://qa-nym-api.qa.nymte.ch/api';
    // const topology = await current_network_topology(validator)
    // const nodeTester = await new NymNodeTester(topology, undefined, preferredGateway);
    //
    // C) use nym-api in the constructor (note: it does no filtering for 'good' nodes on other layers)
    // const validator = 'https://qa-nym-api.qa.nymte.ch/api';
    // const nodeTester = await NymNodeTester.new_with_api(validator, undefined, preferredGateway)

    // D, E, F) you also don't have to specify the gateway. if you don't, a random one (from your topology) will be used
    // const topology = dummyTopology()
    // const nodeTester = await new NymNodeTester(topology);

    self.onmessage = async event => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'MagicPayload': {
                    const {mixnodeIdentity} = event.data.args;
                    console.log("starting node test...");

                    let result = await nodeTester.test_node(mixnodeIdentity);
                    printAndDisplayTestResult(result)
                }
            }
        }
    };
}

async function main() {
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN START");

    // load rust WASM package
    await wasm_bindgen(RUST_WASM_URL);
    console.log('Loaded RUST WASM');

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();
    //
    // run test on simplified and dedicated tester:
    await testWithTester();
    //
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN END")
}

// Let's get started!
main();
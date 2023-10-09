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

const RUST_WASM_URL = "nym_credentials_wasm_bg.wasm"

importScripts('nym_credentials_wasm_bg.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    default_debug,
    no_cover_debug,
    NymNodeTester,
    set_panic_hook,
    currentNetworkTopology,
} = wasm_bindgen;

let client = null;

function dummyTopology() {
    const l1Mixnode = {
        mixId: 1,
        owner: 'n1lftjhnl35cjsfd533zhgrwrspx6qmumd8vjgp9',
        host: '80.85.86.75',
        mixPort: 1789,
        identityKey: '91mNjhJSBkJ9Lb6f1iuYMDQPLiX3kAv6paSUCWjGRwQz',
        sphinxKey: 'DmfN1mL1T95nPXvLK44AQKCpW1pStHNQCi6Fgpz5dxDV',
        layer: 1,
        version: '1.1.20',
    };
    const l2Mixnode = {
        mixId: 2,
        owner: 'n18ztkyh20gwzrel0e5m4sahd358fq9p4skwa7d3',
        host: '139.162.199.75',
        mixPort: 1789,
        identityKey: 'BkLhuKQNyPS19sHZ3HHKCTKwK7hCU6XiFLndyZZHiB7s',
        sphinxKey: '7KGC97tJRhJZKhDqFcsp4Vu715VVxizuD7BktnzuSmZC',
        layer: 2,
        version: '1.1.20',
    };
    const l3Mixnode = {
        mixId: 3,
        owner: 'n1njq8h4nndp7ngays5el2rdp22hq67lwqcaq3ph',
        host: '139.162.244.139',
        identityKey: 'EPja9Kv8JtPHsFbzPdBQierMu5GmQy5roE5njyD6dmND',
        sphinxKey: 'HWpsZChDrtEH8XNscW3qJMRzdCfUD8N8DmMcKqFv7tcf',
        layer: 3,
    };

    const gateway = {
        owner: 'n1d9lclqnfddgg57xe5p0fw4ng54m9f95hal5tlq',
        host: '85.159.211.99',
        mixPort: 1789,
        clientsPort: 9000,
        identityKey: '6pXQcG1Jt9hxBzMgTbQL5Y58z6mu4KXVRbA1idmibwsw',
        sphinxKey: 'GSdqV7GFSwHWQrVV13pNLMeafTLDVFKBKVPxuhdGrpR3',
        version: '1.1.19',
    };

    const mixnodes = new Map();
    mixnodes.set(1, [l1Mixnode]);
    mixnodes.set(2, [l2Mixnode]);
    mixnodes.set(3, [l3Mixnode]);

    const gateways = [gateway];

    return {
        mixnodes, gateways
    }
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

async function testWithNymCredentials() {
    const preferredGateway = "6pXQcG1Jt9hxBzMgTbQL5Y58z6mu4KXVRbA1idmibwsw";
    const nymApi = 'https://qa-nym-api.qa.nymte.ch/api';

    // A) construct with hardcoded topology
    const topology = dummyTopology()

    // optional arguments: id, gateway
    // mandatory (one of) arguments: topology, nymApi
    const nodeTester = await new NymNodeTester({ id: "foomp", topology: topology });

    // B) first get topology directly from nym-api
    // const topology = await currentNetworkTopology(nymApi)
    // const nodeTester = await new NymNodeTester({topology});

    // C) use nym-api in the constructor (note: it does no filtering for 'good' nodes on other layers)
    // const validator = 'https://qa-nym-api.qa.nymte.ch/api';
    // const nodeTester = await new NymNodeTester({nymApi});

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
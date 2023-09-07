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

const RUST_WASM_URL = "nym_client_wasm_bg.wasm"

importScripts('nym_client_wasm.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    default_debug,
    no_cover_debug,
    NymClientBuilder,
    NymClient,
    set_panic_hook,
    ClientConfig,
    GatewayEndpointConfig,
    current_network_topology,
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

async function testWithNymClient() {
    const preferredGateway = "6pXQcG1Jt9hxBzMgTbQL5Y58z6mu4KXVRbA1idmibwsw";
    const topology = dummyTopology()

    let received = 0

    const onMessageHandler = (message) => {
        received += 1;
        self.postMessage({
            kind: 'ReceiveMessage',
            args: {
                message,
                senderTag: undefined,
                isMagicPayload: true,
            },
        });

        // it's really up to the user to create proper callback here...
        console.log(`received ${received} packets so far`)
    };

    console.log('Instantiating WASM client...');

    let clientBuilder = NymClientBuilder.new_tester(topology, onMessageHandler, preferredGateway)
    console.log('Web worker creating WASM client...');
    let local_client = await clientBuilder.start_client();
    console.log('WASM client running!');

    const selfAddress = local_client.self_address();

    // set the global (I guess we don't have to anymore?)
    client = local_client;

    console.log(`Client address is ${selfAddress}`);
    self.postMessage({
        kind: 'Ready',
        args: {
            selfAddress,
        },
    });

    // Set callback to handle messages passed to the worker.
    self.onmessage = async event => {
        console.log(event)
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'SendMessage': {
                    const {message, recipient} = event.data.args;
                    let uint8Array = new TextEncoder().encode(message);
                    await client.send_regular_message(uint8Array, recipient);
                    break;
                }
                case 'MagicPayload': {
                    const {mixnodeIdentity} = event.data.args;
                    const req = await client.try_construct_test_packet_request(mixnodeIdentity);
                    await client.change_hardcoded_topology(req.injectable_topology());
                    await client.try_send_test_packets(req);
                    break;
                }
            }
        }
    };
}

async function wasm_bindgenSetup(onMessageHandler) {
    const preferredGateway = "6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
    const validator = 'https://qa-nym-api.qa.nymte.ch/api';

    // STEP 1. construct config
    // those are just some examples, there are obviously more permutations;
    // note, the extra optional argument is of the following type:
    // /*
    // export interface ClientConfigOpts {
    //     id?: string;
    //     nymApi?: string;
    //     nyxd?: string;
    //     debug?: DebugWasm;
    // }
    //  */
    //
    // const debug = no_cover_debug()
    //
    // #1
    // const config = new ClientConfig({ id: 'my-awesome-client', nymApi: validator, debug: debug} );
    // #2
    // const config = new ClientConfig({ nymApi: validator, debug: debug} );
    // #3
    // const config = new ClientConfig({ id: 'my-awesome-client' } );
    //
    // #4
    const differentDebug = default_debug()
    const updatedTraffic = differentDebug.traffic;
    updatedTraffic.use_extended_packet_size = true
    updatedTraffic.average_packet_delay_ms = 666;
    differentDebug.traffic = updatedTraffic;

    const config = new ClientConfig( { debug: differentDebug } );
    //
    // // STEP 2. setup the client
    // // note, the extra optional argument is of the following type:
    // /*
    //     export interface MixFetchOptsSimple {
    //         preferredGateway?: string;
    //         storagePassphrase?: string;
    //     }
    //  */
    // #1
    // return await NymClient.newWithConfig(config, onMessageHandler)
    //
    // #2
    return await NymClient.newWithConfig(config, onMessageHandler, { storagePassphrase: "foomp" })
    //
    // #3
    // return await NymClient.newWithConfig(config, onMessageHandler, { storagePassphrase: "foomp", preferredGateway })
}

async function nativeSetup(onMessageHandler) {
    const preferredGateway = "6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
    const validator = 'https://qa-nym-api.qa.nymte.ch/api';

    // those are just some examples, there are obviously more permutations;
    // note, the extra optional argument is of the following type:
    /*
        export interface ClientOpts extends ClientOptsSimple {
            clientId?: string;
            nymApiUrl?: string;
            nyxdUrl?: string;
            clientOverride?: DebugWasmOverride;
        }

    	where `DebugWasmOverride` is a rather nested struct that you can look up yourself : )
     */

    // #1
    // return new NymClient(onMessageHandler)
    // #2
    // return new NymClient(onMessageHandler, { nymApiUrl: validator })
    // #3
    const noCoverTrafficOverride = {
        traffic: { disableMainPoissonPacketDistribution: true },
        coverTraffic: { disableLoopCoverTrafficStream: true },
    }

    return new NymClient(onMessageHandler, { storagePassphrase: "foomp", nymApiUrl: validator, clientId: "my-client", clientOverride: noCoverTrafficOverride } )
}

async function normalNymClientUsage() {
    self.postMessage({ kind: 'DisableMagicTestButton' });

    const onMessageHandler = (message) => {
        console.log(message);
        self.postMessage({
            kind: 'ReceiveMessage',
            args: {
                message,
            },
        });
    };

    console.log('Instantiating WASM client...');
    // let localClient = await wasm_bindgenSetup(onMessageHandler)
    let localClient = await nativeSetup(onMessageHandler)
    console.log('WASM client running!');

    const selfAddress = localClient.self_address();

    // set the global (I guess we don't have to anymore?)
    client = localClient;

    console.log(`Client address is ${selfAddress}`);
    self.postMessage({
        kind: 'Ready',
        args: {
            selfAddress,
        },
    });

    // Set callback to handle messages passed to the worker.
    self.onmessage = async event => {
        console.log(event)
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'SendMessage': {
                    const {message, recipient} = event.data.args;
                    let uint8Array = new TextEncoder().encode(message);
                    await client.send_regular_message(uint8Array, recipient);
                    break;
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
    // hook-up the whole client for testing (not recommended)
    // await testWithNymClient()
    //
    // 'Normal' client setup (to send 'normal' messages)
    await normalNymClientUsage()
    //
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN END")
}

// Let's get started!
main();
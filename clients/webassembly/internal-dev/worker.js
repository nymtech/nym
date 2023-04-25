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

importScripts('nym_client_wasm.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    NymNodeTester,
    WasmGateway,
    WasmMixNode,
    WasmNymTopology,
    default_debug,
    NymClientBuilder,
    NymClient,
    set_panic_hook,
    Config,
    GatewayEndpointConfig,
    current_network_topology,
} = wasm_bindgen;

let client = null;
let tester = null;

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

function dummyGatewayConfig() {
    return new GatewayEndpointConfig(
        '336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9',
        'n1rqqw8km7a0rvf8lr6k8dsdqvvkyn2mglj7xxfm',
        'ws://85.159.212.96:9000',
    )
}

async function testWithTester() {
    const gatewayConfig = dummyGatewayConfig();

    // A) construct with hardcoded topology
    const topology = dummyTopology()
    const nodeTester = await new NymNodeTester(gatewayConfig, topology);

    // B) first get topology directly from nym-api
    // const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    // const topology = await current_network_topology(validator)
    // const nodeTester = await new NymNodeTester(gatewayConfig, topology);
    //
    // C) use nym-api in the constructor (note: it does no filtering for 'good' nodes on other layers)
    // const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    // const nodeTester = await NymNodeTester.new_with_api(gatewayConfig, validator)

    self.onmessage = async event => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'TestPacket': {
                    const {mixnodeIdentity} = event.data.args;
                    console.log("starting node test...");

                    let result = await nodeTester.test_node(mixnodeIdentity);
                    printAndDisplayTestResult(result)
                }
            }
        }
    };
}

async function testWithNymClient() {
    const gatewayConfig = dummyGatewayConfig();
    const topology = dummyTopology()

    let received = 0

    const onMessageHandler = (message) => {
        received += 1;
        self.postMessage({
            kind: 'ReceiveMessage',
            args: {
                message,
                senderTag: undefined,
                isTestPacket: true,
            },
        });

        // it's really up to the user to create proper callback here...
        console.log(`received ${received} packets so far`)
    };

    console.log('Instantiating WASM client...');

    let clientBuilder = NymClientBuilder.new_tester(gatewayConfig, topology, onMessageHandler)
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
                case 'TestPacket': {
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

async function normalNymClientUsage() {
    self.postMessage({kind: 'DisableMagicTestButton'});

    // only really useful if you want to adjust some settings like traffic rate
    // (if not needed you can just pass a null)
    const debug = default_debug();

    debug.disable_main_poisson_packet_distribution = true;
    debug.disable_loop_cover_traffic_stream = true;
    debug.use_extended_packet_size = false;
    // debug.average_packet_delay_ms = BigInt(10);
    // debug.average_ack_delay_ms = BigInt(10);
    // debug.ack_wait_addition_ms = BigInt(3000);
    // debug.ack_wait_multiplier = 10;

    debug.topology_refresh_rate_ms = BigInt(60000)

    const gatewayConfig = dummyGatewayConfig();
    const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';

    const config = new Config('my-awesome-wasm-client', validator, gatewayConfig, debug);

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

    let localClient = await new NymClient(config, onMessageHandler)
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
    // load WASM package
    await wasm_bindgen('nym_client_wasm_bg.wasm');
    console.log('Loaded WASM');

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    // run test on simplified and dedicated tester:
    await testWithTester()

    // hook-up the whole client for testing
    // await testWithNymClient()

    // 'Normal' client setup (to send 'normal' messages)
    // await normalNymClientUsage()
}

// Let's get started!
main();
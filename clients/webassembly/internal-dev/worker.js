// Copyright 2020-2022 Nym Technologies SA
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
const { WasmGateway, WasmMixNode, WasmNymTopology, default_debug, NymClientBuilder, set_panic_hook, Config, GatewayEndpointConfig } = wasm_bindgen;

let client = null;

async function main() {
    // load WASM package
    await wasm_bindgen('nym_client_wasm_bg.wasm');

    console.log('Loaded WASM');

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    // validator server we will use to get topology from
    const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';

    // const gatewayId = '336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9';
    // const gatewayOwner = 'n1rqqw8km7a0rvf8lr6k8dsdqvvkyn2mglj7xxfm';
    // const gatewayListener = 'ws://85.159.212.96:9000';
    // const gatewayEndpoint = new GatewayEndpointConfig(gatewayId, gatewayOwner, gatewayListener);

    const gatewayConfig = new GatewayEndpointConfig(
      '336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9',
      'n1rqqw8km7a0rvf8lr6k8dsdqvvkyn2mglj7xxfm',
      'ws://85.159.212.96:9000',
    );


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
      '336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9',
      '1.1.1',
    );

    const mixnodes = new Map();
    mixnodes.set(1, [l1Mixnode]);
    mixnodes.set(2, [l2Mixnode]);
    mixnodes.set(3, [l3Mixnode]);


    const gateways = [gateway];

    const topology = new WasmNymTopology(mixnodes, gateways);


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

    let clientBuilder = NymClientBuilder.new_tester(gatewayConfig, topology, onMessageHandler)
    console.log('Web worker creating WASM client...');
    let local_client = await clientBuilder.start_client();
    console.log('WASM client running!');

    const selfAddress = local_client.self_address();

    // set the global (I guess we don't have to anymore?)
    client = local_client;


    await client.send_test_packet("FoM5Mx9Pxk1g3zEqkS3APgtBeTtTo3M8k7Yu4bV6kK1R")

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
                    const { message, recipient } = event.data.args;
                    let uint8Array = new TextEncoder().encode(message);
                    console.log("client: ", client)
                    console.log(message)
                    await client.send_regular_message(uint8Array, recipient);
                }
            }
        }
    };

    //
    //
    //
    //
    // // only really useful if you want to adjust some settings like traffic rate
    // // (if not needed you can just pass a null)
    // const debug = default_debug();
    //
    // debug.disable_main_poisson_packet_distribution = true;
    // debug.disable_loop_cover_traffic_stream = true;
    // debug.use_extended_packet_size = false;
    // // debug.average_packet_delay_ms = BigInt(10);
    // // debug.average_ack_delay_ms = BigInt(10);
    // // debug.ack_wait_addition_ms = BigInt(3000);
    // // debug.ack_wait_multiplier = 10;
    //
    // debug.topology_refresh_rate_ms = BigInt(60000)
    //
    // const config = new Config('my-awesome-wasm-client', validator, gatewayConfig, debug);
    //
    // const onMessageHandler = (message) => {
    //     console.log(message);
    //     self.postMessage({
    //         kind: 'ReceiveMessage',
    //         args: {
    //             message,
    //         },
    //     });
    // };
    //
    // console.log('Instantiating WASM client...');
    //
    // let clientBuilder = new NymClientBuilder(config, onMessageHandler)
    // console.log('Web worker creating WASM client...');
    // let local_client = await clientBuilder.start_client();
    // console.log('WASM client running!');
    //
    // const selfAddress = local_client.self_address();
    //
    // // set the global (I guess we don't have to anymore?)
    // client = local_client;
    //
    // console.log(`Client address is ${selfAddress}`);
    // self.postMessage({
    //     kind: 'Ready',
    //     args: {
    //         selfAddress,
    //     },
    // });
    //
    // // Set callback to handle messages passed to the worker.
    // self.onmessage = async event => {
    //     console.log(event)
    //     if (event.data && event.data.kind) {
    //         switch (event.data.kind) {
    //             case 'SendMessage': {
    //                 const { message, recipient } = event.data.args;
    //                 let uint8Array = new TextEncoder().encode(message);
    //                 console.log("client: ", client)
    //                 console.log(message)
    //                 await client.send_regular_message(uint8Array, recipient);
    //             }
    //         }
    //     }
    // };
}

// Let's get started!
main();
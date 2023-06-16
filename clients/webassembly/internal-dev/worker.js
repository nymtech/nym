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
const GO_WASM_URL = "main.wasm"

importScripts('nym_client_wasm.js');
importScripts('wasm_exec.js');

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
    FetchToMixnetRequest,
    ClientStorage,
    MixFetchConfig,
    MixFetchClient,
    current_network_topology,
    make_key,
    make_key2,
    call_go_foomp
} = wasm_bindgen;

let client = null;
let tester = null;
const go = new Go(); // Defined in wasm_exec.js
var goWasm;

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

async function testWithTester() {
    const preferredGateway = "336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9";

    // A) construct with hardcoded topology
    const topology = dummyTopology()
    const nodeTester = await new NymNodeTester(topology, preferredGateway);

    // B) first get topology directly from nym-api
    // const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    // const topology = await current_network_topology(validator)
    // const nodeTester = await new NymNodeTester(topology, preferredGateway);
    //
    // C) use nym-api in the constructor (note: it does no filtering for 'good' nodes on other layers)
    // const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    // const nodeTester = await NymNodeTester.new_with_api(validator, preferredGateway)

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

async function testWithNymClient() {
    const preferredGateway = "336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9";
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

    const preferredGateway = "336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9";
    const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';

    const config = new Config('my-awesome-wasm-client', validator, debug);

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

async function messWithStorage() {
    self.onmessage = async event => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'MagicPayload': {
                    const { mixnodeIdentity } = event.data.args;
                    console.log("button clicked...", mixnodeIdentity);

                    let id1 = "one";
                    let id2 = "two";

                    console.log("making store1 NO-ENC");
                    let _storage1 = await ClientStorage.new_unencrypted(id1);

                    console.log("making store2 ENC")
                    let _storage2 = await new ClientStorage(id2, "my-secret-password");
                    //
                    //
                    //
                    //     console.log("attempting to use store1 WITH PASSWORD")
                    //     let _storage1_alt = await new ClientStorage(id1, "password");
                    //
                    //
                    //
                    //     console.log("attempting to use store2 WITHOUT PASSWORD")
                    //     let _storage2_alt = await ClientStorage.new_unencrypted(id2);
                    //
                    //
                    //
                    //     console.log("attempting to use store2 with WRONG PASSWORD")
                    //     let _storage2_bad = await new ClientStorage(id2, "bad-password")


                    //
                    // console.log("read1: ", await storage1.read());
                    // console.log("read2: ", await storage2.read());
                    //
                    // console.log("store1: ", await storage1.store("FOOMP"));
                    //
                    // console.log("read1: ", await storage1.read());
                    // console.log("read2: ", await storage2.read());
                }
            }
        }
    };
}


async function testMixFetch() {
    // self.postMessage({kind: 'DisableMagicTestButton'});

    // only really useful if you want to adjust some settings like traffic rate
    // (if not needed you can just pass a null)
    const debug = default_debug();
    debug.disable_main_poisson_packet_distribution = true;
    debug.disable_loop_cover_traffic_stream = true;
    debug.use_extended_packet_size = false;

    const preferredGateway = "336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9";
    const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    const mix_fetch_network_requester_address= "FbFmrWX1xkd3MUv1LinQ4emXrtP8krvGEngXPECDpN3c.BZJ9zVb19q8JDWRYSvcwQMSivBWt8FJPdK7dY2A3Aqx1@6Lnxj9vD2YMtSmfe8zp5RBtj1uZLYQAFRxY9q7ANwrZz";

    const config = new MixFetchConfig('my-awesome-mix-fetch-client', mix_fetch_network_requester_address, validator, undefined, debug);

    const onMessageHandler = (message) => {
        console.log(message);
        self.postMessage({
            kind: 'ReceiveMessage',
            args: {
                message,
            },
        });
    };

    console.log('Instantiating Mix Fetch client...');

    let mix_fetch = await new MixFetchClient(config, preferredGateway)
    console.log('Mix Fetch client running!');

    const selfAddress = mix_fetch.self_address();

    // set the global (I guess we don't have to anymore?)
    client = mix_fetch;

    console.log(`Client address is ${selfAddress}`);
    self.postMessage({
        kind: 'Ready',
        args: {
            selfAddress,
        },
    });

    // const fetchToMixnetRequest = new FetchToMixnetRequest();
    // console.log(fetchToMixnetRequest.fetch_with_str('https://nymtech.net/index.html'));
    // console.log(fetchToMixnetRequest.fetch_with_request({
    //     url: 'https://nymtech.net/.wellknown/wallet/validators.json',
    //     method: 'GET'
    // }));
    // console.log(fetchToMixnetRequest.fetch_with_request({
    //     url: 'http://localhost:3000',
    //     method: 'POST',
    //     body: Uint8Array.from([0, 1, 2, 3, 4])
    // }));
    // console.log(fetchToMixnetRequest.fetch_with_str_and_init('http://localhost:3000', {
    //     method: 'POST',
    //     body: Uint8Array.from([1, 1, 1, 1, 1]),
    //     headers: {'Content-Type': 'application/json'}
    // }));
    // console.log(fetchToMixnetRequest.fetch_with_request_and_init({
    //     url: 'https://nymtech.net/.wellknown/wallet/validators.json',
    //     method: 'GET'
    // }, {body: Uint8Array.from([1, 1, 1, 1, 1]), headers: {'Content-Type': 'application/json'}}));

    // Set callback to handle messages passed to the worker.
    self.onmessage = async event => {
        console.log(event)
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'MagicPayload': {
                    // ignore the field naming : ) I'm just abusing that a bit...
                    const {mixnodeIdentity} = event.data.args;
                    const url = mixnodeIdentity;

                    console.log('using mixFetch...');
                    let res = await client.fetch_with_str(url);
                    let text = await res.text()
                    console.log('mixFetch done');
                    console.log("HEADERS:     ", ...res.headers)
                    console.log("STATUS:      ", res.status)
                    console.log("STATUS TEXT: ", res.statusText)
                    console.log("OK:          ", res.ok)
                    console.log("TYPE:        ", res.type)
                    console.log("URL:         ", res.url)
                    console.log("TEXT:\n",text)

                    self.postMessage({
                        kind: 'DisplayString',
                        args: {
                            rawString: text,
                        },
                    });
                }
            }
        }
    };

    // console.log('using mixFetch...');
    // await client.fetch_with_str('https://nymtech.net/.wellknown/wallet/validators.json');
}

async function testMixFetchSSL() {
    const debug = default_debug();
    debug.disable_main_poisson_packet_distribution = true;
    debug.disable_loop_cover_traffic_stream = true;

    const preferredGateway = "336yuXAeGEgedRfqTJZsG2YV7P13QH1bHv1SjCZYarc9";
    const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
    const mix_fetch_network_requester_address= "FbFmrWX1xkd3MUv1LinQ4emXrtP8krvGEngXPECDpN3c.BZJ9zVb19q8JDWRYSvcwQMSivBWt8FJPdK7dY2A3Aqx1@6Lnxj9vD2YMtSmfe8zp5RBtj1uZLYQAFRxY9q7ANwrZz";

    const config = new MixFetchConfig('my-awesome-mix-fetch-client-with-go', mix_fetch_network_requester_address, validator, undefined, debug);

    const onMessageHandler = (message) => {
        console.log(message);
        self.postMessage({
            kind: 'ReceiveMessage',
            args: {
                message,
            },
        });
    };

    console.log('Instantiating Mix Fetch client...');
    let mix_fetch = await new MixFetchClient(config, preferredGateway)
    console.log('Mix Fetch client running!');

    const selfAddress = mix_fetch.self_address();

    // set the global (I guess we don't have to anymore?)
    client = mix_fetch;

    console.log(`Client address is ${selfAddress}`);
    self.postMessage({
        kind: 'Ready',
        args: {
            selfAddress,
        },
    });

    // const fetchToMixnetRequest = new FetchToMixnetRequest();
    // console.log(fetchToMixnetRequest.fetch_with_str('https://nymtech.net/index.html'));
    // console.log(fetchToMixnetRequest.fetch_with_request({
    //     url: 'https://nymtech.net/.wellknown/wallet/validators.json',
    //     method: 'GET'
    // }));
    // console.log(fetchToMixnetRequest.fetch_with_request({
    //     url: 'http://localhost:3000',
    //     method: 'POST',
    //     body: Uint8Array.from([0, 1, 2, 3, 4])
    // }));
    // console.log(fetchToMixnetRequest.fetch_with_str_and_init('http://localhost:3000', {
    //     method: 'POST',
    //     body: Uint8Array.from([1, 1, 1, 1, 1]),
    //     headers: {'Content-Type': 'application/json'}
    // }));
    // console.log(fetchToMixnetRequest.fetch_with_request_and_init({
    //     url: 'https://nymtech.net/.wellknown/wallet/validators.json',
    //     method: 'GET'
    // }, {body: Uint8Array.from([1, 1, 1, 1, 1]), headers: {'Content-Type': 'application/json'}}));

    // Set callback to handle messages passed to the worker.
    self.onmessage = async event => {
        console.log(event)
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'MagicPayload': {
                    // ignore the field naming : ) I'm just abusing that a bit...
                    const {mixnodeIdentity} = event.data.args;
                    const url = mixnodeIdentity;

                    console.log('using mixFetch...');
                    let res = await client.fetch_with_str(url);
                    let text = await res.text()
                    console.log('mixFetch done');
                    console.log("HEADERS:     ", ...res.headers)
                    console.log("STATUS:      ", res.status)
                    console.log("STATUS TEXT: ", res.statusText)
                    console.log("OK:          ", res.ok)
                    console.log("TYPE:        ", res.type)
                    console.log("URL:         ", res.url)
                    console.log("TEXT:\n",text)

                    self.postMessage({
                        kind: 'DisplayString',
                        args: {
                            rawString: text,
                        },
                    });
                }
            }
        }
    };

    // console.log('using mixFetch...');
    // await client.fetch_with_str('https://nymtech.net/.wellknown/wallet/validators.json');
}

async function basicSSL() {

    self.onmessage = async event => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'StartHandshake': {
                    console.log("start")
                    let clientHello = goWasmStartSSLHandshake();
                    self.postMessage({
                        kind: 'SSLClient',
                        args: { data: clientHello },
                    });

                    break
                }

                case 'ClientPayload': {
                    let clientData = goWasmTryReadClientData();
                    self.postMessage({
                        kind: 'SSLClient',
                        args: { data: clientData },
                    });

                    break
                }
                case 'ServerPayload': {
                    const data = event.data.args.data
                    console.log("INJECTING", data)


                    goWasmInjectServerData(data)

                    break

                }
            }
        }
    };
}

async function loadGoWasm() {
    const resp = await fetch(GO_WASM_URL);
    const bytes = await resp.arrayBuffer();
    const wasmObj = await WebAssembly.instantiate(bytes, go.importObject)
    goWasm = wasmObj.instance
    go.run(goWasm)

    // if ('instantiateStreaming' in WebAssembly) {
    //     WebAssembly.instantiateStreaming(fetch(GO_WASM_URL), go.importObject).then(function (obj) {
    //         goWasm = obj.instance;
    //         go.run(goWasm);
    //     })
    // } else {
    //     fetch(GO_WASM_URL).then(resp =>
    //         resp.arrayBuffer()
    //     ).then(bytes =>
    //         WebAssembly.instantiate(bytes, go.importObject).then(function (obj) {
    //             goWasm = obj.instance;
    //             go.run(goWasm);
    //         })
    //     )
    // }
}

async function main() {
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN START");

    // load rust WASM package
    await wasm_bindgen(RUST_WASM_URL);
    console.log('Loaded RUST WASM');

    // load go WASM package
    await loadGoWasm();
    console.log("Loaded GO WASM");


    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    await basicSSL()


    // let foomp = goFoomp();
    // console.log("logging results from go in JS: ", foomp)
    //
    // console.log("attempting to call go from rust via js!");
    // call_go_foomp()

    // test mixFetch
    // await testMixFetch();

    // run test on simplified and dedicated tester:
    // await testWithTester()

    // hook-up the whole client for testing
    // await testWithNymClient()

    // 'Normal' client setup (to send 'normal' messages)
    // await normalNymClientUsage()

    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN END")
}

// Let's get started!
main();
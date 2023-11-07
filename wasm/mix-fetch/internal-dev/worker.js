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

const RUST_WASM_URL = "mix_fetch_wasm_bg.wasm"
const GO_WASM_URL = "go_conn.wasm"

importScripts('mix_fetch_wasm.js');
importScripts('wasm_exec.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    default_debug,
    no_cover_debug,
    NymClient,
    set_panic_hook,
    Config,
    GatewayEndpointConfig,
    ClientStorage,
    MixFetchConfig,
    send_client_data,
    start_new_mixnet_connection,
    setupMixFetch,
    disconnectMixFetch,
    setupMixFetchWithConfig,
    mix_fetch_initialised,
    finish_mixnet_connection} = wasm_bindgen;

let client = null;
let tester = null;
const go = new Go(); // Defined in wasm_exec.js
var goWasm;

async function logFetchResult(res) {
    console.log(res)
    let text = await res.text()
    console.log("HEADERS:     ", ...res.headers)
    console.log("STATUS:      ", res.status)
    console.log("STATUS TEXT: ", res.statusText)
    console.log("OK:          ", res.ok)
    console.log("TYPE:        ", res.type)
    console.log("URL:         ", res.url)
    console.log("BODYUSED:    ", res.bodyUsed)
    console.log("REDIRECTED:  ", res.redirected)
    console.log("TEXT:        ", text)

    self.postMessage({
        kind: 'DisplayString',
        args: {
            rawString: text,
        },
    });
}

async function wasm_bindgenSetup() {
    const preferredGateway = "6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
    const validator = 'https://qa-nym-api.qa.nymte.ch/api';

    // local
    const mixFetchNetworkRequesterAddress= "2o47bhnXWna6VEyt4mXMGQQAbXfpKmX7BkjkxUz8uQVi.6uQGnCqSczpXwh86NdbsCoDDXuqZQM9Uwko8GE7uC9g8@6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
    // const mixFetchNetworkRequesterAddress= "GqiGWmKRCbGQFSqH88BzLKijvZgipnqhmbNFsmkZw84t.4L8sXFuAUyUYyHZYgMdM3AtiusKnYUft6Pd8e41rrCHA@6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";

    // STEP 1. construct config
    // those are just some examples, there are obviously more permutations;
    // note, the extra optional argument is of the following type:
    // /*
    //     export interface MixFetchConfigOpts {
    //         id?: string;
    //         nymApi?: string;
    //         nyxd?: string;
    //         debug?: DebugWasm;
    //     }
    //  */
    //
    // const debug = no_cover_debug()
    //
    // #1
    // const config = new MixFetchConfig(mixFetchNetworkRequesterAddress, { id: 'my-awesome-mix-fetch-client', nymApi: validator, debug: debug} );
    // #2
    // const config = new MixFetchConfig(mixFetchNetworkRequesterAddress, { nymApi: validator, debug: debug} );
    // #3
    // const config = new MixFetchConfig(mixFetchNetworkRequesterAddress, { id: 'my-awesome-mix-fetch-client' } );
    //
    // #4
    const differentDebug = default_debug()
    const updatedTraffic = differentDebug.traffic;
    updatedTraffic.use_extended_packet_size = true
    updatedTraffic.average_packet_delay_ms = 666;
    differentDebug.traffic = updatedTraffic;

    const config = new MixFetchConfig(mixFetchNetworkRequesterAddress, { debug: differentDebug } );
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
    await setupMixFetchWithConfig(config)
    //
    // #2
    // await setupMixFetchWithConfig(config, { storagePassphrase: "foomp" })
    //
    // #3
    // await setupMixFetchWithConfig(config, { storagePassphrase: "foomp", preferredGateway })
}

async function nativeSetup() {
    // const preferredGateway = "6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
    // const validator = 'https://qa-nym-api.qa.nymte.ch/api';

    // local
    // const preferredNetworkRequester= "2o47bhnXWna6VEyt4mXMGQQAbXfpKmX7BkjkxUz8uQVi.6uQGnCqSczpXwh86NdbsCoDDXuqZQM9Uwko8GE7uC9g8@6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
    // const preferredNetworkRequester= "GqiGWmKRCbGQFSqH88BzLKijvZgipnqhmbNFsmkZw84t.4L8sXFuAUyUYyHZYgMdM3AtiusKnYUft6Pd8e41rrCHA@6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";

    // those are just some examples, there are obviously more permutations;
    // note, the extra optional argument is of the following type:
    /*
        export interface MixFetchOpts extends MixFetchOptsSimple {
            clientId?: string;
            nymApiUrl?: string;
            nyxdUrl?: string;
            clientOverride?: DebugWasmOverride;
            mixFetchOverride?: MixFetchDebugOverride;
        }

        where
        export interface MixFetchDebugOverride {
	       requestTimeoutMs?: number;
    	}

    	and `DebugWasmOverride` is a rather nested struct that you can look up yourself : )
     */

    // #1
    // await setupMixFetch(preferredNetworkRequester)
    // #2
    // await setupMixFetch(preferredNetworkRequester, { nymApiUrl: validator })
    // // #3
    const noCoverTrafficOverride = {
        traffic: { disableMainPoissonPacketDistribution: true },
        coverTraffic: { disableLoopCoverTrafficStream: true },
    }
    const mixFetchOverride = {
        requestTimeoutMs: 10000
    }

    await setupMixFetch({
        // preferredNetworkRequester,
        preferredGateway: "E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM",
        storagePassphrase: "foomp",
        forceTls: true,
        // nymApiUrl: validator,
        clientId: "my-client",
        clientOverride: noCoverTrafficOverride,
        mixFetchOverride,
    })
}

async function testMixFetch() {
    console.log('Instantiating Mix Fetch...');
    // await wasm_bindgenSetup()

    await nativeSetup()


    console.log('Mix Fetch client running!');

    // Set callback to handle messages passed to the worker.
    self.onmessage = async event => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'FetchPayload': {
                    const {target} = event.data.args;
                    const url = target;

                    // const args = { mode: "ors", redirect: "manual", signal }
                    const args = { mode: "unsafe-ignore-cors" }

                    try {
                        console.log('using mixFetch...');
                        const mixFetchRes = await mixFetch(url, args)
                        console.log(">>> MIX FETCH")
                        await logFetchResult(mixFetchRes)

                        console.log('done')

                    } catch(e) {
                        console.error("mix fetch request failure: ", e)
                    }

                    // console.log("will disconnect");
                    // await disconnectMixFetch()
                    //
                    // try {
                    //     console.log('using mixFetch...');
                    //     const mixFetchRes = await mixFetch(url, args)
                    //     console.log(">>> MIX FETCH")
                    //     await logFetchResult(mixFetchRes)
                    //
                    //     console.log('done')
                    //
                    // } catch(e) {
                    //     console.error("mix fetch request failure: ", e)
                    // }


                    // try {
                    //     console.log('using normal Fetch...');
                    //     const fetchRes = await fetch(url, args)
                    //     console.log(">>> NORMAL FETCH")
                    //     await logFetchResult(fetchRes)
                    // } catch(e) {
                    //     console.error("fetch request failure: ", e)
                    // }
                }
            }
        }
    };
}


// TODO: look into https://www.aaron-powell.com/posts/2019-02-08-golang-wasm-5-compiling-with-webpack/
async function loadGoWasm() {
    const resp = await fetch(GO_WASM_URL);

    if ('instantiateStreaming' in WebAssembly) {
        const wasmObj = await WebAssembly.instantiateStreaming(resp, go.importObject)
        goWasm = wasmObj.instance
        go.run(goWasm)
    } else {
        const bytes  = await resp.arrayBuffer()
        const wasmObj = await WebAssembly.instantiate(bytes, go.importObject)
        goWasm = wasmObj.instance
        go.run(goWasm)
    }
}

function setupRsGoBridge() {
    // (note: reason for intermediate `__rs_go_bridge__` object is to decrease global scope bloat
    // and to discourage users from trying to call those methods directly)
    self.__rs_go_bridge__ = {}
    self.__rs_go_bridge__.send_client_data = send_client_data
    self.__rs_go_bridge__.start_new_mixnet_connection = start_new_mixnet_connection
    self.__rs_go_bridge__.mix_fetch_initialised = mix_fetch_initialised
    self.__rs_go_bridge__.finish_mixnet_connection = finish_mixnet_connection
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

    setupRsGoBridge();

    goWasmSetLogging("trace")

    // test mixFetch
    await testMixFetch();
    //
    // // run test on simplified and dedicated tester:
    // // await testWithTester()
    //
    // // hook-up the whole client for testing
    // // await testWithNymClient()
    //
    // // 'Normal' client setup (to send 'normal' messages)
    // // await normalNymClientUsage()
    //
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN END")
}

// Let's get started!
main();
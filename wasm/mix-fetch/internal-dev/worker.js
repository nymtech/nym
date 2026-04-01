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

const RUST_WASM_URL = "mix_fetch_wasm_bg.wasm";
const GO_WASM_URL = "go_conn.wasm";

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
    finish_mixnet_connection,
} = wasm_bindgen;

let client = null;
let tester = null;
const go = new Go(); // Defined in wasm_exec.js
var goWasm;
let mixFetchReady = false;

function sendLog(message, level = 'info') {
    self.postMessage({
        kind: 'Log',
        args: { message, level },
    });
}

function sendReady() {
    self.postMessage({ kind: 'MixFetchReady' });
}

function sendError(error) {
    self.postMessage({
        kind: 'MixFetchError',
        args: { error: String(error) },
    });
}

async function logFetchResult(res) {
    console.log(res);
    let text = await res.text();
    console.log("HEADERS:     ", ...res.headers);
    console.log("STATUS:      ", res.status);
    console.log("STATUS TEXT: ", res.statusText);
    console.log("OK:          ", res.ok);
    console.log("TYPE:        ", res.type);
    console.log("URL:         ", res.url);
    console.log("BODYUSED:    ", res.bodyUsed);
    console.log("REDIRECTED:  ", res.redirected);
    console.log("TEXT:        ", text);

    self.postMessage({
        kind: 'DisplayString',
        args: {
            rawString: text,
        },
    });
}

/*
 * ── Alternative MixFetch setup ──────────────────────────────
 *
 * Shows how to pass custom MixFetchConfig options: specific network requester
 * address, validator URL, or debug overrides.  Uncomment and adapt if you
 * need non-default setup for local testing.
 *
 *  async function wasm_bindgenSetup() {
 *      const preferredGateway = "6qQYb4ArXANU6HJDxzH4PFCUqYb39Dae2Gem2KpxescM";
 *      const validator = 'https://qa-nym-api.qa.nymte.ch/api';
 *
 *      const mixFetchNetworkRequesterAddress = "2o47bhnXWna6VEyt4mXMGQQAbXfpKmX7BkjkxUz8uQVi...";
 *
 *      // MixFetchConfigOpts: { id?, nymApi?, nyxd?, debug? }
 *      const differentDebug = default_debug()
 *      differentDebug.traffic.use_extended_packet_size = true
 *      differentDebug.traffic.average_packet_delay_ms = 666
 *
 *      const config = new MixFetchConfig(mixFetchNetworkRequesterAddress, {debug: differentDebug});
 *
 *      // MixFetchOptsSimple: { preferredGateway?, storagePassphrase? }
 *      await setupMixFetchWithConfig(config)
 *  }
 */

async function nativeSetup(preferredGateway, setupOpts = {}) {
    sendLog('Setting up MixFetch...');
    if (preferredGateway) {
        sendLog(`Using preferred gateway: ${preferredGateway}`);
    } else {
        sendLog('Using random gateway selection');
    }

    const {
        forceTls = true,
        clientId = 'my-client',
        disablePoisson = true,
        disableCover = true,
        requestTimeoutMs = 60000,
    } = setupOpts;

    const noCoverTrafficOverride = {
        traffic: { disableMainPoissonPacketDistribution: disablePoisson },
        coverTraffic: { disableLoopCoverTrafficStream: disableCover },
    };
    const mixFetchOverride = {
        requestTimeoutMs,
    };

    const opts = {
        forceTls,
        clientId,
        clientOverride: noCoverTrafficOverride,
        mixFetchOverride,
    };

    if (preferredGateway) {
        opts.preferredGateway = preferredGateway;
    }

    sendLog(
        `Setup config: forceTls=${forceTls}, clientId=${clientId}, disablePoisson=${disablePoisson}, disableCover=${disableCover}, timeout=${requestTimeoutMs}ms`
    );
    sendLog('Calling setupMixFetch...');
    await setupMixFetch(opts);
    sendLog('setupMixFetch completed');
}

async function startMixFetch(preferredGateway, setupOpts) {
    sendLog('Instantiating MixFetch...');

    try {
        await nativeSetup(preferredGateway, setupOpts);
        mixFetchReady = true;
        sendLog('MixFetch client running!');
        sendReady();
    } catch (e) {
        sendLog('Failed to start MixFetch: ' + e, 'error');
        sendError(e);
    }
}

async function handleFetchPayload(target) {
    if (!mixFetchReady) {
        sendLog('MixFetch not ready yet', 'error');
        return;
    }

    const url = target;
    const args = { mode: "unsafe-ignore-cors" };

    try {
        sendLog(`Fetching: ${url}`);
        const mixFetchRes = await mixFetch(url, args);
        sendLog('Fetch completed');
        await logFetchResult(mixFetchRes);
    } catch (e) {
        sendLog('Fetch request failure: ' + e, 'error');
        console.error("mix fetch request failure: ", e);
    }
}

async function handleStressTestFetch(id, url, label) {
    if (!mixFetchReady) {
        sendLog('MixFetch not ready yet', 'error');
        return;
    }

    const tag = `[stress #${id} ${label}]`;
    const start = performance.now();
    const args = { mode: "unsafe-ignore-cors" };

    try {
        sendLog(`${tag} Fetching: ${url}`);
        const res = await mixFetch(url, args);
        const text = await res.text();
        const elapsed = ((performance.now() - start) / 1000).toFixed(2);
        sendLog(`${tag} ${res.status} OK in ${elapsed}s (${text.length} bytes)`);
        self.postMessage({
            kind: 'StressTestFetchResult',
            args: {
                id,
                ok: true,
                status: res.status,
                elapsed,
                textLength: text.length,
                body: text,
            },
        });
    } catch (e) {
        const elapsed = ((performance.now() - start) / 1000).toFixed(2);
        sendLog(`${tag} FAILED in ${elapsed}s: ${e}`, 'error');
        self.postMessage({
            kind: 'StressTestFetchResult',
            args: { id, ok: false, elapsed, error: String(e) },
        });
    }
}

function setupMessageHandler() {
    self.onmessage = async (event) => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'StartMixFetch': {
                    const { preferredGateway, setupOpts } = event.data.args;
                    await startMixFetch(preferredGateway, setupOpts);
                    break;
                }
                case 'FetchPayload': {
                    const { target } = event.data.args;
                    await handleFetchPayload(target);
                    break;
                }
                case 'SetGoTimeout': {
                    const { timeoutMs } = event.data.args;
                    sendLog(`Setting Go-side request timeout to ${timeoutMs}ms`);
                    self.__go_rs_bridge__.goWasmSetMixFetchRequestTimeout(timeoutMs);
                    break;
                }
                case 'StressTestFetch': {
                    const { id, url, label } = event.data.args;
                    // NOT awaited — each request runs independently,
                    // just like separate callers in a real app
                    handleStressTestFetch(id, url, label);
                    break;
                }
            }
        }
    };
}

// TODO: look into https://www.aaron-powell.com/posts/2019-02-08-golang-wasm-5-compiling-with-webpack/
async function loadGoWasm() {
    const resp = await fetch(GO_WASM_URL);

    if ('instantiateStreaming' in WebAssembly) {
        const wasmObj = await WebAssembly.instantiateStreaming(resp, go.importObject);
        goWasm = wasmObj.instance;
        go.run(goWasm);
    } else {
        const bytes = await resp.arrayBuffer();
        const wasmObj = await WebAssembly.instantiate(bytes, go.importObject);
        goWasm = wasmObj.instance;
        go.run(goWasm);
    }
}

function setupRsGoBridge() {
    // (note: reason for intermediate `__rs_go_bridge__` object is to decrease global scope bloat
    // and to discourage users from trying to call those methods directly)
    self.__rs_go_bridge__ = {};
    self.__rs_go_bridge__.send_client_data = send_client_data;
    self.__rs_go_bridge__.start_new_mixnet_connection = start_new_mixnet_connection;
    self.__rs_go_bridge__.mix_fetch_initialised = mix_fetch_initialised;
    self.__rs_go_bridge__.finish_mixnet_connection = finish_mixnet_connection;
}

async function main() {
    sendLog('Worker starting...');

    // load rust WASM package
    sendLog('Loading Rust WASM...');
    await wasm_bindgen(RUST_WASM_URL);
    sendLog('Loaded Rust WASM');

    // load go WASM package
    sendLog('Loading Go WASM...');
    await loadGoWasm();
    sendLog('Loaded Go WASM');

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    setupRsGoBridge();

    goWasmSetLogging("trace");

    // Set up message handler (MixFetch will be started on demand)
    setupMessageHandler();

    sendLog('Worker ready - click Start MixFetch to begin');
}

// Let's get started!
main();

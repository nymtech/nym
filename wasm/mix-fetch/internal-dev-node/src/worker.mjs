import {parentPort} from 'worker_threads';
import {setupMixFetch, set_panic_hook, send_client_data, start_new_mixnet_connection, mix_fetch_initialised, finish_mixnet_connection} from '@nymproject/mix-fetch-wasm';
import fs from 'node:fs';
import("./polyfill.mjs")

// Sorry Mark, but I don't want to deal with bundling hell to get this one work via proper import : (
let goWasmPath = "../../../dist/node/wasm/mix-fetch/go_conn.wasm"
// import goWasm from '@nymproject/mix-fetch-wasm/go_conn.wasm'


function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

function setupRsGoBridge() {
    // (note: reason for intermediate `__rs_go_bridge__` object is to decrease global scope bloat
    // and to discourage users from trying to call those methods directly)
    globalThis.__rs_go_bridge__ = {}
    globalThis.__rs_go_bridge__.send_client_data = send_client_data
    globalThis.__rs_go_bridge__.start_new_mixnet_connection = start_new_mixnet_connection
    globalThis.__rs_go_bridge__.mix_fetch_initialised = mix_fetch_initialised
    globalThis.__rs_go_bridge__.finish_mixnet_connection = finish_mixnet_connection
}

async function loadGoWasm() {
    // I... don't even want to know. Javascript (and node) works in mysterious ways...
    if (!globalThis.Go) {
        await sleep(500)
    }

    console.log(globalThis.Origin)

    const go = new Go()
    const blob = fs.readFileSync(goWasmPath);
    const wasmObj = await WebAssembly.instantiate(blob, go.importObject)
    const goWasm = wasmObj.instance
    go.run(goWasm)
}

async function logFetchResult(res) {
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
}


let initDone = false

parentPort.on('message', async message => {
    await handleMessage(message)
});

async function handleMessage(message) {
    console.log(`handling "${message.kind}"`)
    switch (message.kind) {
        case 'initRequest':
            await initialiseMixFetch()
            break;
        case 'fetchRequest':
            await handleFetchRequest(message.data.resource, message.data.options);
            break;
        default:
            console.log("UNKOWN MESSAGE")
            break;
    }
}

async function handleFetchRequest(resource, options) {
    console.log("will fetch", resource, options)
    let res = await mixFetch(resource, options)
    await logFetchResult(res)
}

async function nativeSetup(onMessageHandler) {
    const noCoverTrafficOverride = {
        traffic: { disableMainPoissonPacketDistribution: true },
        coverTraffic: { disableLoopCoverTrafficStream: true },
    }
    const mixFetchOverride = {
        requestTimeoutMs: 10000
    }

    await setupMixFetch({ storagePassphrase: "foomp", clientId: "my-client", clientOverride: noCoverTrafficOverride, mixFetchOverride })

}

function finishInit() {
    initDone = true
    parentPort.postMessage({kind: 'initResponse', data: {done: initDone}})
}

async function initialiseMixFetch() {
    if (initDone) {
        return finishInit()
    }

    setupRsGoBridge()
    await loadGoWasm()

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    console.log('Instantiating mix fetch...');
    await nativeSetup()
    console.log('mix fetch client running!');

    finishInit()
}

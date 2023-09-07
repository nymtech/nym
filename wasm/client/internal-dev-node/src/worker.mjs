import {parentPort} from 'worker_threads';
import setGlobalVars from 'indexeddbshim';
import WebSocket from 'ws';
import {NymClient, set_panic_hook} from '@nymproject/nym-client-wasm';

// polyfill setup
const globalVar =
    typeof window !== "undefined"
        ? window
        : typeof WorkerGlobalScope !== "undefined"
            ? self
            : typeof global !== "undefined"
                ? global
                : Function("return this;")();

// checkOrigin:false is required to avoid  SecurityError Cannot open
// an IndexedDB database from an opaque origin.
setGlobalVars(globalVar, {checkOrigin: false})
globalVar.WebSocket = WebSocket

let initDone = false
let nymClient = null
let clientAddress = null

parentPort.on('message', async message => {
    await handleMessage(message)
});

function onReceived(message, senderTag) {
    parentPort.postMessage({kind: 'receivedMessage', data: {message, senderTag}});
}

async function handleMessage(message) {
    console.log(`handling "${message.kind}"`)
    switch (message.kind) {
        case 'initRequest':
            await initialiseWasmClient()
            break;
        case 'sendRequest':
            await handleSendRequest(message.data.message, message.data.recipient);
            break;
        default:
            console.log("UNKOWN MESSAGE")
            break;
    }
}

async function handleSendRequest(message, recipient) {
    await nymClient.send_regular_message(message, recipient)
}

async function nativeSetup(onMessageHandler) {
    const validator = 'https://validator.nymtech.net/api/';

    const noCoverTrafficOverride = {
        traffic: {disableMainPoissonPacketDistribution: true},
        coverTraffic: {disableLoopCoverTrafficStream: true},
    }

    return new NymClient(onMessageHandler, {
        storagePassphrase: "foomp",
        nymApiUrl: validator,
        clientId: "my-client",
        clientOverride: noCoverTrafficOverride
    })
}


function finishInit() {
    initDone = true
    parentPort.postMessage({kind: 'initResponse', data: {done: initDone, clientAddress}})
}

async function initialiseWasmClient() {
    if (initDone) {
        return finishInit()
    }

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    console.log('Instantiating WASM client...');
    nymClient = await nativeSetup(onReceived)
    clientAddress = nymClient.self_address()
    console.log('WASM client running!');

    finishInit()
}

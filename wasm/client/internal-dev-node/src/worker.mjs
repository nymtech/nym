import { parentPort } from 'worker_threads';

import { set_panic_hook, NymClient } from '../../../../dist-node/wasm/client/nym_client_wasm.js';

// const RUST_WASM_URL = "nym_client_wasm_bg.wasm"
//
// importScripts('nym_client_wasm.js');



let client = null

parentPort.on('message', async event => {
    await handleMessage(event)

    console.log("done handling");
});

async function handleMessage(event) {
    switch (event.kind) {
        case 'init':
            await initialiseWasmClient()
            break;
        default:
            console.log("UNKOWN MESSAGE")
            break;
    }
}

async function nativeSetup(onMessageHandler) {
    const preferredGateway = "2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh";
    const validator = 'https://validator.nymtech.net/api/';

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

function dummyOnMessage(message) {
    console.log("received a message!", message)
}

async function initialiseWasmClient() {
    console.log(">>>>>>>>>>>>>>>>>>>>> NODE WORKER START");

    // load rust WASM package
    // await wasm_bindgen(RUST_WASM_URL);
    // console.log('Loaded RUST WASM');

    // sets up better stack traces in case of in-rust panics
    set_panic_hook();

    console.log('Instantiating WASM client...');
    let localClient = await nativeSetup(dummyOnMessage)
    console.log('WASM client running!');

    const selfAddress = localClient.self_address();
    console.log("our address is ", selfAddress)

    let message = "hello world"
    console.log(`sending "${message}" to ourselves...`)
    let uint8Array = new TextEncoder().encode(message);
    await localClient.send_regular_message(uint8Array, selfAddress)


    // await sleep(1000);
    console.log("done!");
}

//
// self.onmessage = async event => {
//     console.log("worker event!", event)
// }


function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}
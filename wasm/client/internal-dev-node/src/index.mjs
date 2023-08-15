import { Worker, workerData } from 'worker_threads';
import setGlobalVars from 'indexeddbshim';
import WebSocket from 'ws';
import { set_panic_hook, NymClient } from '../../../../dist-node/wasm/client/nym_client_wasm.js';

// polyfill setup
var globalVar =
    typeof window !== "undefined"
        ? window
        : typeof WorkerGlobalScope !== "undefined"
            ? self
            : typeof global !== "undefined"
                ? global
                : Function("return this;")();

// checkOrigin:false is required to avoid  SecurityError Cannot open
// an IndexedDB database from an opaque origin.
setGlobalVars(globalVar, { checkOrigin: false })
globalVar.WebSocket = WebSocket


// import client from "websocket";
// const {WebSocketClient} = client
// // var WebSocketClient = require('websocket').client;
//
// globalVar.WebSocket = function (uri) {
//     var self = this;
//     this.connection = null;
//     console.log(client)
//     this.socket = new WebSocketClient();
//     this.socket.on('connect', function (connection) {
//         self.connection = connection;
//
//         connection.on('error', function (error) {
//             self.onerror();
//         });
//
//         connection.on('close', function () {
//             self.onclose();
//         });
//
//         connection.on('message', function (message) {
//             if (message.type === 'utf8') {
//                 self.onmessage({data:message.utf8Data});
//             }
//         });
//
//         self.onopen();
//     });
//     this.socket.connect(uri);
// }
//
// globalVar.WebSocket.prototype.send = function (data) {
//     this.connection.sendUTF(data);
// }



// export function runWorker() {// ./src/migration/migration.js
//     const worker = new Worker('your file path of migration code to js file', {
//         workerData: {
//             path: 'Your ts file path from js file'  /// ./db-migration.ts
//         }
//     });worker.on('message', function (data) {
//         console.log('In Message');
//     });
//     worker.on('error', function (error) {
//         worker.terminate();
//     });worker.on('exit', (code) => {
//         if (code !== 0)
//             worker.terminate();
//     });
//     return worker;
// }

let mixClient = null;

class NodeWorkerClient {
    worker = null;

    constructor() {
        this.worker = new Worker('./src/worker.mjs');

        this.worker.on("message", message => {
            console.log(`received worker message: ${message}`);
        });

        this.worker.on("error", error => {
            console.log(error);
        });

        this.worker.on("exit", exitCode => {
            console.log(`It exited with code ${exitCode}`);
        })

    }
}

function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

async function main() {

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


    // client = new NodeWorkerClient()
    // console.log("hello world")
    //
    // client.worker.postMessage({kind: 'init'})
    //
    // await sleep(100000000)
}

await main()

process.on('SIGTERM', signal => {
    console.log(`Process ${process.pid} received a SIGTERM signal`)
    process.exit(0)
})

process.on('SIGINT', signal => {
    console.log(`Process ${process.pid} has been interrupted`)
    process.exit(0)
})
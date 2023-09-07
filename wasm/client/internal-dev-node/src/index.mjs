import {Worker} from 'worker_threads';

function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

class NodeWorkerClient {
    worker = null;
    clientInitialised = false;
    selfAddress = null

    constructor() {
        this.worker = new Worker('./src/worker.mjs');

        this.worker.on("message", message => {
            this.handleMessage(message)
        });

        this.worker.on("error", error => {
            console.log(error);
        });

        this.worker.on("exit", exitCode => {
            console.log(`It exited with code ${exitCode}`);
        })

    }

    initWasmClient = () => {
        this.worker.postMessage({kind: 'initRequest', data: {}})
    }

    sendMessage = (message, recipient) => {
        this.worker.postMessage({kind: 'sendRequest', data: {message, recipient}})
    }

    handleReceived = (message, senderTag) => {
        console.log(`received raw message: ${message}`)
        console.log(`with senderTag: ${senderTag}`)

        let decoded = new TextDecoder().decode(message);
        console.log(`decoded message: "${decoded}"`)
    }

    handleMessage = (message) => {
        console.log(`handling "${message.kind}"`)
        switch (message.kind) {
            case 'initResponse':
                this.clientInitialised = message.data.done;
                this.selfAddress = message.data.clientAddress
                break;
            case 'receivedMessage':
                this.handleReceived(message.data.message, message.data.senderTag)
                break;
            default:
                console.log("UNKOWN MESSAGE")
                break;
        }
    }

    // hehe, that's so disgusting, but I don't want to spend ages in the JS callback hell
    // because that's not the point of this example
    waitForWasmClient = async () => {
        let start = new Date();
        let pollingRate = 100;
        let maxWaitSecs = 5;
        while (!this.clientInitialised) {
            let now = new Date();
            let diff = (now.getTime() - start.getTime()) / 1000;
            if (diff > maxWaitSecs) {
                return Promise.reject(new Error('failed to initialise wasm client'))
            }
            await sleep(pollingRate)
        }
    }
}


async function main() {
    let client = new NodeWorkerClient()
    client.initWasmClient()
    await client.waitForWasmClient()

    let ourAddress = client.selfAddress
    console.log("main thread address: ", ourAddress)

    let message = "hello world"
    let uint8Array = new TextEncoder().encode(message);
    console.log(`sending "${message}" to ourselves...`)
    client.sendMessage(uint8Array, ourAddress)
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
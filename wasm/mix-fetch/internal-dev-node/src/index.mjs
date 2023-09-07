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

    initMixFetch = () => {
        this.worker.postMessage({kind: 'initRequest', data: {}})
    }

    fetch = (resource, options) => {
        this.worker.postMessage({kind: 'fetchRequest', data: {resource, options}})
    }

    handleResponse = (message, senderTag) => {
        // console.log(`received raw message: ${message}`)
        // console.log(`with senderTag: ${senderTag}`)
        //
        // let decoded = new TextDecoder().decode(message);
        // console.log(`decoded message: "${decoded}"`)
    }

    handleMessage = (message) => {
        console.log(`handling "${message.kind}"`)
        switch (message.kind) {
            case 'initResponse':
                this.clientInitialised = message.data.done;
                break;
            case 'receivedResponse':
                this.handleResponse(message.data.response)
                break;
            default:
                console.log("UNKOWN MESSAGE")
                break;
        }
    }

    // hehe, that's so disgusting, but I don't want to spend ages in the JS callback hell
    // because that's not the point of this example
    waitForMixFetch = async () => {
        let start = new Date();
        let pollingRate = 100;
        let maxWaitSecs = 5;
        while (!this.clientInitialised) {
            let now = new Date();
            let diff = (now.getTime() - start.getTime()) / 1000;
            if (diff > maxWaitSecs) {
                return Promise.reject(new Error('failed to initialise mix fetch'))
            }
            await sleep(pollingRate)
        }
    }
}


async function main() {
    let client = new NodeWorkerClient()
    client.initMixFetch()
    await client.waitForMixFetch()

    await client.fetch("https://nymtech.net", { mode: "unsafe-ignore-cors" })
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
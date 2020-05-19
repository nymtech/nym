import * as wasm from "nym-client-wasm";

export class Identity {
    // in the future this should allow for loading from local storage
    constructor() {
        const raw_identity = JSON.parse(wasm.keygen());
        this.address = raw_identity.address;
        this.privateKey = raw_identity.private_key;
        this.publicKey = raw_identity.public_key;
    }
}

export class Client {
    // constructor(gateway_url, ownAddress, registeredCallback) {
    constructor(directoryUrl, identity, authToken) {
        this.authToken = authToken
        this.gateway = null; // {socketAddress, mixAddress, conn}
        this.identity = identity;
        this.topology = null;
        this.topologyEndpoint = directoryUrl + "/api/presence/topology";

        // I think a cleaner alternative would be to just change the functions
        // into arrow functions. However, for some peculiar reason 
        // this can't be achieved by default because webpack server runs ES5
        // and you need at least ES6 for this feature. You could still get this by wasting
        // few hours by setting all loaders, babel plugins, etc and transpiling it...
        this.onGatewayMessage = this.onGatewayMessage.bind(this);
    }

    async start() {
        await this.updateTopology();
        this._getInitialGateway();
        await this.establishGatewayConnection();
        // TODO: a way to somehow await for our authenticate response to be processed
    }

    _isRegistered() {
        return this.authToken !== null
    }

    async updateTopology() {
        let response = await http('get', this.topologyEndpoint);
        let topology = JSON.parse(response); // make sure it's a valid json
        console.log(topology);
        this.topology = topology;
        this.onUpdatedTopology();
        return topology;
    }

    /* Gets the address of a Nym gateway to send the Sphinx packet to.
    At present we choose the first gateway as the network should only be running
    one. Later, we will implement multiple gateways. */
    _getInitialGateway() {
        if (this.gateway !== null) {
            console.error("tried to re-initialise gateway data");
            return;
        }
        if (this.topology === null || this.topology.gatewayNodes.length === 0) {
            console.error("No gateways available on the network")
        }
        this.gateway = {
            socketAddress: this.topology.gatewayNodes[0].clientListener,
            mixAddress: this.topology.gatewayNodes[0].pubKey,
            conn: null,
        }
    }

    establishGatewayConnection() {
        return new Promise((resolve, reject) => {
            const conn = new WebSocket(this.gateway.socketAddress);
            conn.onclose = this.onGatewayConnectionClose;
            conn.onerror = (event) => {
                this.onGatewayConnectionError(event);
                reject(event);
            };
            conn.onmessage = this.onGatewayMessage;
            conn.onopen = (event) => {
                this.onEstablishedGatewayConnection(event);
                if (this._isRegistered()) {
                    this.sendAuthenticateRequest();
                    resolve(); // TODO: we should wait for authenticateResponse...
                } else {
                    this.sendRegisterRequest();
                    resolve(); // TODO: we should wait for registerResponse...
                }
            }

            this.gateway.conn = conn;
        })
    }

    sendRegisterRequest() {
        const registerReq = makeRegisterRequest(this.identity.address);
        this.gateway.conn.send(registerReq);
        this.onRegisterRequestSend();
    }

    sendAuthenticateRequest(token) {
        const authenticateReq = makeAuthenticateRequest(this.identity.address, token);
        this.conn.send(authenticateReq);
        this.onAuthenticateRequestSend();
    }

    /* 
        NOTE: this currently does not implement chunking and messages over ~1KB 
        will cause a panic. This will be fixed in a future version.
    */
    sendMessage(message, recipient) {
        if (this.gateway === null || this.gateway.conn === null) {
            console.error("Client was not initialised");
            return
        }
        if (message instanceof Blob || message instanceof ArrayBuffer) {
            // but it wouldn't be difficult to implement it. 
            console.error("Binary messages are not yet supported");
            return
        }
        // TODO: CURRENTLY WE ONLY ACCEPT "recipient", not recipient@gateway
        // this will be changed in the very next PR
        console.log("send", this.topology)
        const sphinxPacket = wasm.create_sphinx_packet(JSON.stringify(this.topology), message, recipient);
        this.gateway.conn.send(sphinxPacket);
        this.onMessageSend();
    }

    onGatewayMessage(event) {
        if (event.data instanceof Blob) {
            this.onBlobResponse(event);
        } else {
            const receivedData = JSON.parse(event.data);
            switch (receivedData.type) {
                case "send": return this.onSendConfirmation(event);
                case "authenticate": return this.onAuthenticateResponse(event);
                case "error": return this.onErrorResponse(event);
                case "register": return this.onRegisterResponse(event);
                default: return this.onUnknownResponse(event);
            }
        }
    }

    // all the callbacks that can be overwritten

    onUpdatedTopology() {
        console.log("Default: Updated topology")
    }

    onEstablishedGatewayConnection(event) {
        console.log("Default: Established gateway connection", event);
    }

    onGatewayConnectionClose(event) {
        console.log("Default: The the connection to gateway was closed", event);
    }

    onGatewayConnectionError(event) {
        console.error("Default: Gateway connection error: ", event);
    }

    onAuthenticateRequestSend() {
        console.log("Default: sent authentication request");
    }

    onRegisterRequestSend() {
        console.log("Default: sent register request");
    }

    onMessageSend() {
        console.log("Default: sent message through gateway to the mixnet");
    }

    onAuthenticated() {
        console.log("Default: we are authenticated");
    }

    onSendConfirmation(event) {
        console.log("Default: received send confirmation", event.data);
    }

    onRegisterResponse(event) {
        console.log("Default: received register response", event.data);
    }

    onAuthenticateResponse(event) {
        console.log("Default: received authentication response", event.data);
    }

    onErrorResponse(event) {
        console.error("Received error response", event.data);
    }

    onUnknownResponse(event) {
        console.error("Received unknown response", event);
    }

    // Gateway returns any received data from the mix network as a Blob,
    // So most likely this is your best bet to override
    onBlobResponse(event) {
        // note that the actual handling depends on the expected content, however,
        // in this example we're just sending a text messages to ourselves and hence
        // we can safely read it as text
        let reader = new FileReader();

        reader.onload = () => {
            this.onParsedBlobResponse(reader.result)
        };

        reader.readAsText(event.data);
    }

    // Alternatively you may use default implementation and get everything as a text
    onParsedBlobResponse(data) {
        console.log("Default: parsed the following data", data);
    }
}


function makeRegisterRequest(address) {
    return JSON.stringify({ "type": "register", "address": address });
}

function makeAuthenticateRequest(address, token) {
    return JSON.stringify({ "type": "authenticate", "address": address, "token": token });
}

function http(method, url) {
    return new Promise(function (resolve, reject) {
        let xhr = new XMLHttpRequest();
        xhr.open(method, url);
        xhr.onload = function () {
            if (this.status >= 200 && this.status < 300) {
                resolve(xhr.response);
            } else {
                reject({
                    status: this.status,
                    statusText: xhr.statusText
                });
            }
        };
        xhr.onerror = function () {
            reject({
                status: this.status,
                statusText: xhr.statusText
            });
        };
        xhr.send();
    });
}

import * as wasm from ".";

/**
 * A Nym identity, consisting of a public/private keypair and a Nym
 * gateway address.
 */
export class Identity {
    // in the future this should allow for loading from local storage
    constructor() {
        const raw_identity = JSON.parse(wasm.keygen());
        this.address = raw_identity.address;
        this.privateKey = raw_identity.private_key;
        this.publicKey = raw_identity.public_key;
    }
}

/**
 * A Client which connects to a Nym gateway via websocket. All communication
 * with the Nym network happens through this connection.
 */
export class Client {
    constructor(directoryUrl, identity, authToken) {
        this.authToken = authToken
        this.gateway = null; // {socketAddress, mixAddress, conn}
        this.identity = identity;
        this.topology = null;
        this.topologyEndpoint = directoryUrl + "/api/presence/topology";
    }

    /**
     * @return {string} a user-pubkey@nym-gateway recipient address
     */
    formatAsRecipient() {
        return `${this.identity.address}@${this.gateway.mixAddress}`
    }

    /**
     * Get the current network topology, then connect to this client's Nym gateway 
     * via websocket.
     */
    async start() {
        await this.updateTopology();
        this._getInitialGateway();
        await this.connect();
        // TODO: a way to somehow await for our authenticate response to be processed
    }

    _isRegistered() {
        return this.authToken !== null
    }

    /**
     * Update the Nym network topology.
     * 
     * @returns an object containing the current Nym network topology
     */
    async updateTopology() {
        let response = await http('get', this.topologyEndpoint);
        let topology = JSON.parse(response); // make sure it's a valid json
        this.topology = topology;
        this.onUpdatedTopology();
        return topology;
    }

    /**
     * Gets the address of a Nym gateway to send the Sphinx packet to.
     * At present we choose the first gateway as the network should only be 
     * running one. Later, we will implement multiple gateways.
     */
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

    /**
     * Connect to the client's defined Nym gateway via websocket. 
     */
    connect() {
        return new Promise((resolve, reject) => {
            const conn = new WebSocket(this.gateway.socketAddress);
            conn.onclose = this.onConnectionClose;
            conn.onerror = (event) => {
                this.onConnectionError(event);
                reject(event);
            };
            conn.onmessage = (event) => this.onMessage(event);
            conn.onopen = (event) => {
                this.onConnect(event);
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

    /**
     * Sends a registration request to a Nym gateway node. Use it only if you
     * haven't registered this client before.
     */
    sendRegisterRequest() {
        const registerReq = buildRegisterRequest(this.identity.address);
        this.gateway.conn.send(registerReq);
        this.onRegisterRequestSend();
    }

    /**
     * Authenticates with a Nym gateway for this client.
     * 
     * @param {string} token 
     */
    sendAuthenticateRequest(token) {
        const authenticateReq = buildAuthenticateRequest(this.identity.address, token);
        this.conn.send(authenticateReq);
        this.onAuthenticateRequestSend();
    }

    /**
     * Sends a message up the websocket to this client's Nym gateway.
     * 
     * NOTE: this currently does not implement chunking and messages over ~1KB
     * will cause a panic. This will be fixed in a future version.
     * 
     * `message` must be a {string} at the moment. Binary `Blob` and `ArrayBuffer`
     * will be supported soon. 
     * 
     * @param {*} message 
     * @param {string} recipient 
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
        const sphinxPacket = wasm.create_sphinx_packet(JSON.stringify(this.topology), message, recipient);
        this.gateway.conn.send(sphinxPacket);
        this.onMessageSend();
    }

    /**
     * A callback triggered when a message is received from this client's Nym
     * gateway. 
     * 
     * The `event` may be a binary blob which was the payload of a Sphinx packet,
     * or it may be a JSON control message (for example, the result of an
     * authenticate request).
     * @param {*} event 
     */
    onMessage(event) {
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

    /**
     * A callback that fires when network topology is updated.
     */
    onUpdatedTopology() {
        console.log("Default: Updated topology")
    }

    /**
     * 
     * @param {*} event 
     */
    onConnect(event) {
        console.log("Default: Established gateway connection", event);
    }

    onConnectionClose(event) {
        console.log("Default: The the connection to gateway was closed", event);
    }

    onConnectionError(event) {
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
            this.onText(reader.result)
        };

        reader.readAsText(event.data);
    }

    /** 
     * @callback that makes a best-effort attempt to return decrypted Sphinx bytes as text.
     * 
     * Note that no checks are performed to determine whether something is
     * really text. If the received data is in fact binary, you'll get 
     * binary-as-text from this callback.
     */
    onText(data) {
        console.log("Default: parsed the following data", data);
    }
}

/**
 * Build a JSON registration request.
 * 
 * @param {string} address 
 */
function buildRegisterRequest(address) {
    return JSON.stringify({ "type": "register", "address": address });
}

/**
 * Build a JSON authentication request. 
 * 
 * @param {string} address 
 * @param {string} token 
 */
function buildAuthenticateRequest(address, token) {
    return JSON.stringify({ "type": "authenticate", "address": address, "token": token });
}

/**
 * Make an HTTP request.
 * @param {string} method 
 * @param {string} url 
 */
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

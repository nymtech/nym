import * as wasm from "nym-client-wasm";

export function makeRegisterRequest(address) {
    return JSON.stringify({ "type": "register", "address": address });
}

export function makeAuthenticateRequest(address, token) {
    return JSON.stringify({ "type": "authenticate", "address": address, "token": token });
}

/* Gets the address of a Nym gateway to send the Sphinx packet to.

   At present we choose the first gateway as the network should only be running
   one. Later, we will implement multiple gateways. */
export async function getInitialGatewayAddress(directoryUrl) {
    const topology = await getTopology(directoryUrl);
    if (topology.gatewayNodes.length > 0) {
        return topology.gatewayNodes[0].clientListener;
    }
    throw "Unable to get Nym gateway address from " + directoryUrl;
}

/* Creates a Sphinx packet ready to send to a Nym Gateway server. 

NOTE: this currently does not implement chunking and messages over 1KB 
will cause a panic. This will be fixed in a future version.
*/
export function createSphinxPacket(topology, message, recipient) {
    return wasm.create_sphinx_packet(topology, message, recipient);
}



/* Make an HTTP request */
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

import * as wasm from "nym-client-wasm";

export function makeRegisterRequest(address) {
    return JSON.stringify({ "type": "register", "address": address });
}

export function makeAuthenticateRequest(address, token) {
    return JSON.stringify({ "type": "authenticate", "address": address, "token": token });
}

// NOTE: this currently does not implement chunking and too long messages will cause a panic
export function makeSendablePacket(topology, message, recipient) {
    return wasm.create_gateway_sphinx_packet(topology, message, recipient);
}

export async function getTopology(directoryUrl) {
    let response = await http('get', directoryUrl);
    let topology = JSON.parse(response);
    return topology;
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

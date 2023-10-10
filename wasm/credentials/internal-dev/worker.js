// Copyright 2020-2023 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

const RUST_WASM_URL = "nym_credential_client_wasm_bg.wasm"

importScripts('nym_credential_client_wasm.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const {
    acquireCredential,
} = wasm_bindgen;

async function testGetCredential() {
    self.onmessage = async event => {
        if (event.data && event.data.kind) {
            switch (event.data.kind) {
                case 'GetCredential': {
                    const { amount, mnemonic } = event.data.args;

                    // TODO: this should just use cosmjs' coin
                    let coin = `${amount}unym`
                    console.log(`getting credential for ${coin}`);

                    let credential = await acquireCredential(mnemonic, coin, { useSandbox: true })

                    self.postMessage({
                        kind : 'ReceivedCredential',
                        args: {
                            credential
                        }
                    })
                }
            }
        }
    };
}

async function main() {
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN START");

    // load rust WASM package
    await wasm_bindgen(RUST_WASM_URL);
    console.log('Loaded RUST WASM');

    // run test on simplified and dedicated tester:
    await testGetCredential();
    //
    console.log(">>>>>>>>>>>>>>>>>>>>> JS WORKER MAIN END")
}

// Let's get started!
main();
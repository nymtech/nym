// Copyright 2020-2022 Nym Technologies SA
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

importScripts('nym_client_wasm.js');

console.log('Initializing worker');

// wasm_bindgen creates a global variable (with the exports attached) that is in scope after `importScripts`
const { default_debug, get_gateway, NymClient, set_panic_hook, Config } = wasm_bindgen;

class ClientWrapper {
  constructor(config, onMessageHandler) {
    this.rustClient = new NymClient(config);
    this.rustClient.set_on_message(onMessageHandler);
    this.rustClient.set_on_gateway_connect(this.onConnect);
  }

  selfAddress = () => {
    return this.rustClient.self_address();
  };

  onConnect = () => {
    console.log('Established (and authenticated) gateway connection!');
  };

  start = async () => {
    // this is current limitation of wasm in rust - for async methods you can't take self by reference...
    // I'm trying to figure out if I can somehow hack my way around it, but for time being you have to re-assign
    // the object (it's the same one)
    this.rustClient = await this.rustClient.start();
  };

  sendMessage = async (recipient, message) => {
    this.rustClient = await this.rustClient.send_message(recipient, message);
  };

  sendBinaryMessage = async (recipient, message) => {
    this.rustClient = await this.rustClient.send_binary_message(recipient, message);
  };
}

let client = null;

async function main() {
  // load WASM package
  await wasm_bindgen('nym_client_wasm_bg.wasm');

  console.log('Loaded WASM');

  // sets up better stack traces in case of in-rust panics
  set_panic_hook();

  // validator server we will use to get topology from
  const validator = 'https://validator.nymtech.net/api'; //"http://localhost:8081";
  const preferredGateway = 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM';

  const gatewayEndpoint = await get_gateway(validator, preferredGateway);

  // only really useful if you want to adjust some settings like traffic rate
  // (if not needed you can just pass a null)
  const debug = default_debug();
  // note: we still have poisson distribution so, on average, we will be sending SOME packet every 20ms
  // debug.disable_main_poisson_packet_distribution = true;
  // debug.disable_loop_cover_traffic_stream = true;
  // debug.average_packet_delay_ms = BigInt(10);
  // debug.average_ack_delay_ms = BigInt(10);
  // debug.ack_wait_addition_ms = BigInt(3000);
  // debug.ack_wait_multiplier = 10;

  // this is currently disabled, i.e. we'll keep using the topology we get at startup
  debug.topology_refresh_rate_ms = BigInt(60000)

  const config = new Config('my-awesome-wasm-client', validator, gatewayEndpoint, debug);

  const onMessageHandler = (message) => {
    self.postMessage({
      kind: 'ReceiveMessage',
      args: {
        message,
      },
    });
  };

  console.log('Instantiating WASM client...');
  client = new ClientWrapper(config, onMessageHandler);
  console.log('Web worker creating WASM client...');
  await client.start();
  console.log('WASM client running!');

  const selfAddress = client.rustClient.self_address();
  console.log(`Client address is ${selfAddress}`);
  self.postMessage({
    kind: 'Ready',
    args: {
      selfAddress,
    },
  });

  // Set callback to handle messages passed to the worker.
  self.onmessage = async event => {
    if (event.data && event.data.kind) {
      switch (event.data.kind) {
        case 'SendMessage': {
          const { message, recipient } = event.data.args;
          await client.sendMessage(message, recipient);
        }
      }
    }
  };
}

// Let's get started!
main();
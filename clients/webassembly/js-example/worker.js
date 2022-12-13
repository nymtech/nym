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
const { default_debug, NymClientBuilder, set_panic_hook, Config, GatewayEndpointConfig } = wasm_bindgen;

let client = null;

async function main() {
  // load WASM package
  await wasm_bindgen('nym_client_wasm_bg.wasm');

  console.log('Loaded WASM');

  // sets up better stack traces in case of in-rust panics
  set_panic_hook();

  // validator server we will use to get topology from
  const validator = 'https://qwerty-validator-api.qa.nymte.ch/api';
  
  const gatewayId = 'EVupP2tRUeZo5Y6RpBHAbm8kSntpgNyZNL6yCr7BDEoG';
  const gatewayOwner = 'n1rmlew3euapuq7rs4s4j9apv00whrsazr764kl7';
  const gatewayListener = 'ws://176.58.120.72:9000';
  const gatewayEndpoint = new GatewayEndpointConfig(gatewayId, gatewayOwner, gatewayListener)

  // only really useful if you want to adjust some settings like traffic rate
  // (if not needed you can just pass a null)
  const debug = default_debug();

  debug.disable_main_poisson_packet_distribution = true;
  debug.disable_loop_cover_traffic_stream = true;
  debug.use_extended_packet_size = true;
  // debug.average_packet_delay_ms = BigInt(10);
  // debug.average_ack_delay_ms = BigInt(10);
  // debug.ack_wait_addition_ms = BigInt(3000);
  // debug.ack_wait_multiplier = 10;

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
  
  let clientBuilder = new NymClientBuilder(config, onMessageHandler)
  console.log('Web worker creating WASM client...');
  let local_client = await clientBuilder.start_client();
  console.log('WASM client running!');
  
  const selfAddress = local_client.self_address();
  
  // set the global (I guess we don't have to anymore?)
  client = local_client;
  
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
          let uint8Array = new TextEncoder().encode(message);
          await client.send_regular_message(uint8Array, recipient);
        }
      }
    }
  };
}

// Let's get started!
main();
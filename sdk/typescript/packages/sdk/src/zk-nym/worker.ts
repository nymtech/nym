/* eslint-disable no-restricted-globals */
import * as Comlink from 'comlink';
//
// Rollup will replace wasmBytes with a function that loads the WASM bundle from a base64 string embedded in the output.
//
// Doing it this way, saves having to support a large variety of bundlers and their quirks.
//
// @ts-ignore
// eslint-disable-next-line import/no-extraneous-dependencies
import wasmBytes from '@nymproject/zk-nym-lib/zk_nym_lib_bg.wasm';
import init, { acquireCredential } from '@nymproject/zk-nym-lib/zk_nym_lib';
import type { INymZkNymClientWebWorker, ZkNymClientOpts, LoadedEvent } from './types';
import { EventKinds } from './types';

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
// eslint-disable-next-line no-restricted-globals
const postMessageWithType = <E>(event: E) => self.postMessage(event);

console.log('[Nym WASM client for zk-nyms] Starting Nym WASM web worker...');

// load WASM binary
async function main() {
  // rollup with provide a function to get the WASM bytes
  const bytes = await wasmBytes();

  // load rust WASM package
  const wasmPackage = await init(bytes);

  console.log('Loaded RUST WASM');

  wasmPackage.set_panic_hook();

  const webWorker: INymZkNymClientWebWorker = {
    async acquireCredential(coin: string, mnemonic: string, opts: ZkNymClientOpts) {
      console.log('[Worker] --- acquireCredential ---', { coin, mnemonic, opts });
      return acquireCredential(mnemonic, coin, opts);
    },
  };

  // start comlink listening for messages and handle them above
  Comlink.expose(webWorker);

  // notify any listeners that the web worker has loaded and is ready for testing
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}

main().catch((e: any) => console.error('Unhandled exception in zk-nym worker', e));

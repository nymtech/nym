/* eslint-disable no-restricted-globals */
import * as Comlink from 'comlink';
//
// Rollup will replace wasmBytes with a function that loads the WASM bundle from a base64 string embedded in the output.
//
// Doing it this way, saves having to support a large variety of bundlers and their quirks.
//
// @ts-ignore
// eslint-disable-next-line import/no-extraneous-dependencies
import wasmBytes from '@nymproject/nym-credential-client-wasm/zk_nym_faucet_lib_bg.wasm';
import init, { NymIssuanceBandwidthVoucher } from '@nymproject/zk-nym-faucet-lib/zk_nym_faucet_lib';
import type { INymZkNymFaucetClientWebWorker, ZkNymFaucetClientOpts, LoadedEvent } from './types';
import { EventKinds } from './types';

/**
 * Helper method to send typed messages.
 * @param event   The strongly typed message to send back to the calling thread.
 */
// eslint-disable-next-line no-restricted-globals
const postMessageWithType = <E>(event: E) => self.postMessage(event);

console.log('[Nym WASM client for zk-nym faucets] Starting Nym WASM web worker...');

// load WASM binary
async function main() {
  // rollup with provide a function to get the WASM bytes
  const bytes = await wasmBytes();

  // load rust WASM package
  const wasmPackage = await init(bytes);

  console.log('Loaded RUST WASM');

  wasmPackage.set_panic_hook();

  const webWorker: INymZkNymFaucetClientWebWorker = {
    async acquireCredential(faucetApiUrl: string, authToken: string) {
      console.log('getting opts');
      const res = await fetch(`${faucetApiUrl}/api/v1/bandwidth-voucher/prehashed-public-attributes`, {
        headers: new Headers({ Authorization: `Bearer ${authToken}` }),
      });
      const opts = await res.json();

      const issuanceVoucher = new NymIssuanceBandwidthVoucher(opts);
      const blindSignRequest = issuanceVoucher.getBlindSignRequest();

      console.log('getting partial vks');
      const partialVksRes = await fetch(`${faucetApiUrl}/api/v1/bandwidth-voucher/partial-verification-keys`, {
        headers: new Headers({ Authorization: `Bearer ${authToken}` }),
      });
      const partialVks = await partialVksRes.json();

      console.log('getting master vk');
      const masterVkRes = await fetch(`${faucetApiUrl}/api/v1/bandwidth-voucher/master-verification-key`, {
        headers: new Headers({ Authorization: `Bearer ${authToken}` }),
      });
      const masterVk = await masterVkRes.json();

      console.log('getting blinded shares');
      const sharesRes = await fetch(`${faucetApiUrl}/api/v1/bandwidth-voucher/obtain`, {
        method: 'POST',
        headers: new Headers({
          Authorization: `Bearer ${authToken}`,
          'Content-Type': 'application/json',
        }),
        body: JSON.stringify({
          blindSignRequest,
        }),
      });

      const credentialShares = await sharesRes.json();

      console.log('unblinding shares');
      const bandwidthVoucher = issuanceVoucher.unblindShares(credentialShares, partialVks);
      console.log('is valid: ', bandwidthVoucher.ensureIsValid(masterVk.bs58EncodedKey));

      const serialised = bandwidthVoucher.serialise();
      console.log('serialised:\n', serialised);
    },
  };

  // start comlink listening for messages and handle them above
  Comlink.expose(webWorker);

  // notify any listeners that the web worker has loaded and is ready for testing
  postMessageWithType<LoadedEvent>({ kind: EventKinds.Loaded, args: { loaded: true } });
}

main().catch((e: any) => console.error('Unhandled exception in zk-nym faucet worker', e));

import { createNymCredentialsClient } from '@nymproject/sdk';
import { appendOutput } from './utils';

async function main() {
  const mnemonic = document.getElementById('mnemonic') as HTMLInputElement;
  const coin = document.getElementById('coin') as HTMLInputElement;
  const button = document.getElementById('button') as HTMLButtonElement;

  const client = await createNymCredentialsClient();

  const generateCredential = async () => {
    const amount = coin.value;
    const mnemonicString = mnemonic.value;
    console.log({ amount, mnemonicString });
    const credential = await client.comlink.acquireCredential(amount, mnemonicString, { isSandbox: true }); // options: {isSandbox?: boolean; networkDetails?: {}}
    appendOutput(JSON.stringify(credential, null, 2));
  };

  if (button) {
    button.addEventListener('click', () => generateCredential());
  }
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});

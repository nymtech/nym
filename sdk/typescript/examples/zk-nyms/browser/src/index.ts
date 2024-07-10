import { createNymCredentialsClient } from '@nymproject/sdk';
import { appendOutput } from './utils';

async function main() {
  const mnemonic = document.getElementById('mnemonic') as HTMLInputElement;
  if (process.env.MNEMONIC) {
    mnemonic.defaultValue = process.env.MNEMONIC;
  }
  const coin = document.getElementById('coin') as HTMLInputElement;
  const button = document.getElementById('button') as HTMLButtonElement;

  const client = await createNymCredentialsClient();

  const generateCredential = async () => {
    const amount = coin.value;
    const mnemonicString = mnemonic.value;
    console.log({ amount, mnemonicString });
    try {
      appendOutput('About to get a credential... 🥁');
      const credential = await client.comlink.acquireCredential(amount, mnemonicString, { useSandbox: true }); // options: {useSandbox?: boolean; networkDetails?: {}}
      appendOutput('Success! 🎉');
      appendOutput(JSON.stringify(credential, null, 2));
    } catch (e) {
      console.error('Failed to get credential', e);
      appendOutput((e as any).message);
    }
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

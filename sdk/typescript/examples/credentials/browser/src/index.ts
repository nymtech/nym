import { createNymCredentialsClient } from '@nymproject/sdk';
import { appendOutput, appendImageOutput } from './utils';

async function main() {
  // options for mixFetch (you can also set these with the `createMixFetch` function
  const mixFetchOptions = {
    preferredGateway: 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM', // with WSS
    preferredNetworkRequester:
      'GiRjFWrMxt58pEMuusm4yT3RxoMD1MMPrR9M2N4VWRJP.3CNZBPq4vg7v7qozjGjdPMXcvDmkbWPCgbGCjQVw9n6Z@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW',
    mixFetchOverride: {
      requestTimeoutMs: 60_000,
    },
  };

  // disable CORS (in your app, you probably don't want to disable CORS, it is a good thing to leave it enabled)
  const args = { mode: 'unsafe-ignore-cors' };

  // this is the URL of standard list of allow hosts the you can request data from with mixFetch and the Nym SOCKS5
  // client - you can request to have more hosts added by getting in touch on Discord or Telegram
  let url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';

  appendOutput('Get a text file:');
  appendOutput(`Downloading ${url}...\n`);
  let resp = await mixFetch(url, args, mixFetchOptions); // NB: you only need to pass options to the 1st call
  console.log({ resp });

  const text = await resp.text();
  appendOutput(text);

  // get an image
  appendOutput('\nGet an image:\n');
  url = 'https://nymtech.net/favicon.svg';
  resp = await mixFetch(url, args);
  console.log({ resp });

  const buffer = await resp.arrayBuffer();
  const type = resp.headers.get('Content-Type') || 'image/svg';
  const blobUrl = URL.createObjectURL(new Blob([buffer], { type }));
  appendImageOutput(blobUrl);
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});

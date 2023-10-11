import { SetupMixFetchOps, mixFetch } from '@nymproject/mix-fetch';
import { appendOutput, appendImageOutput } from './utils';

async function main() {
  // options for mixFetch (you can also set these with the `createMixFetch` function
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const extra = {
    hiddenGateways: [
      {
        owner: 'n1ns3v70ul9gnl9l9fkyz8cyxfq75vjcmx8el0t3',
        host: 'sandbox-gateway1.nymtech.net',
        explicitIp: '35.158.238.80',
        identityKey: 'HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua',
        sphinxKey: 'BoXeUD7ERGmzRauMjJD3itVNnQiH42ncUb6kcVLrb3dy',
      },
    ],
  };


  const mixFetchOptionsForSandbox: SetupMixFetchOps = {
    preferredGateway: 'HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua', // with WSS
    preferredNetworkRequester:
      'AzGdJ4MU78Ex22NEWfeycbN7bt3PFZr1MtKstAdhfELG.GSxnKnvKPjjQm3FdtsgG5KyhP6adGbPHRmFWDH4XfUpP@HjNEDJuotWV8VD4ufeA1jeheTnfNJ7Jorevp57hgaZua',
    mixFetchOverride: {
      requestTimeoutMs: 60_000,
    },
    nymApiUrl: 'https://sandbox-nym-api1.nymtech.net/api',
    extra,
    forceTls: true
  };

  // options for mixFetch (you can also set these with the `createMixFetch` function
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const mixFetchOptionsMainnet = {
    preferredGateway: 'E3mvZTHQCdBvhfr178Swx9g4QG3kkRUun7YnToLMcMbM', // with WSS
    preferredNetworkRequester:
      'GiRjFWrMxt58pEMuusm4yT3RxoMD1MMPrR9M2N4VWRJP.3CNZBPq4vg7v7qozjGjdPMXcvDmkbWPCgbGCjQVw9n6Z@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW',
    mixFetchOverride: {
      requestTimeoutMs: 60_000,
    },
  };

  const mixFetchOptions = mixFetchOptionsForSandbox;


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

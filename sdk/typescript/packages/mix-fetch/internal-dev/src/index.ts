import { createMixFetch } from '@nymproject/mix-fetch';

function appendOutput(value: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  const text = document.createTextNode(`${value}\n`);
  el.appendChild(text);
}

function appendImageOutput(url: string) {
  const el = document.getElementById('outputImage') as HTMLPreElement;
  const imgNode = document.createElement('img');
  imgNode.src = url;
  el.appendChild(imgNode);
}

/**
 * The main entry point
 */
async function main() {
  appendOutput('Tester is starting up...');

  const worker = await createMixFetch();
  (window as any).mixFetch = worker;

  if (!worker) {
    console.error('Oh no! Could not create mixFetch');
    appendOutput('Oh no! Could not create mixFetch');
  }

  // only really useful if you want to adjust some settings like traffic rate
  // (if not needed you can just pass a null)

  appendOutput('About to set up mixFetch...');

  const addr =
    'BUFKbUncPWL4WkQPHc7JRusdXwXKi5omS8Fz5Rr34JoZ.8iM69a1pjcMwLEdQCkmq5jdGi8tsSpbjQqk6YWQAX6Ae@3ojQD6V7skM1bSXJX7fVQvscjmcgptzdixQEaAha2ixh';

  appendOutput('Instantiating Mix Fetch...');
  // await setupMixFetch(config, {storagePassphrase: "foomp"})

  await worker.setupMixFetch({
    preferredNetworkRequester: addr,
    clientId: 'my-new-client-15',
    clientOverride: {
      coverTraffic: { disableLoopCoverTrafficStream: true },
      traffic: { disableMainPoissonPacketDistribution: true },
    },
    mixFetchOverride: { requestTimeoutMs: 60000 },
  });

  appendOutput('Ready!');

  let url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';
  appendOutput(`Using mixFetch to get ${url}...`);
  const args = { mode: 'unsafe-ignore-cors' };

  let resp = await worker.mixFetch(url, args);
  console.log({ resp });

  appendOutput(JSON.stringify(resp, null, 2));

  // get an image
  url = 'https://nymtech.net/images/nav/nym-logo.svg';
  resp = await worker.mixFetch(url, args);
  console.log({ resp });
  appendOutput(JSON.stringify(resp, null, 2));
  appendImageOutput(resp.body.blobUrl);
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});

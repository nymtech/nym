import { createMixFetch, disconnectMixFetch } from '@nymproject/mix-fetch';

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

  // const addr =
  //   'EVdJ66jqpoVzmktVecy5UJxsTCEWo5gMn5zDZR7Hm8jy.GXNpoX7RcYcxKvBkV3dSHqC78WaPuWieweRPWzYqNhh5@GAjhJcrd6f1edaqUkfWCff6zdHoqo756qYrc2TfPuCXJ';
  const addr = undefined;

  appendOutput('About to set up mixFetch...');
  const { mixFetch } = await createMixFetch({
    preferredNetworkRequester: addr,
    clientId: 'my-new-client-16',
    clientOverride: {
      coverTraffic: { disableLoopCoverTrafficStream: true },
      traffic: { disableMainPoissonPacketDistribution: true },
    },
    mixFetchOverride: { requestTimeoutMs: 60000 },
    responseBodyConfigMap: {},
  });
  (window as any).mixFetch = mixFetch;

  if (!(window as any).mixFetch) {
    console.error('Oh no! Could not create mixFetch');
    appendOutput('Oh no! Could not create mixFetch');
  } else {
    appendOutput('Ready!');
  }

  let url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';
  appendOutput(`Using mixFetch to get ${url}...`);
  const args = { mode: 'unsafe-ignore-cors' };

  let resp = await mixFetch(url, args);
  console.log({ resp });
  const text = await resp.text();

  appendOutput(JSON.stringify(resp, null, 2));
  appendOutput(JSON.stringify({ text }, null, 2));

  console.log('disconnecting');
  await disconnectMixFetch();
  console.log('disconnected! all further usages should fail');

  // get an image
  url = 'https://nymtech.net/favicon.svg';
  resp = await mixFetch(url, args);
  console.log({ resp });
  const buffer = await resp.arrayBuffer();
  const type = resp.headers.get('Content-Type') || 'image/svg';
  const blobUrl = URL.createObjectURL(new Blob([buffer], { type }));
  appendOutput(JSON.stringify({ bufferBytes: buffer.byteLength, blobUrl }, null, 2));
  appendImageOutput(blobUrl);
}

// wait for the html to load
window.addEventListener('DOMContentLoaded', () => {
  // let's do this!
  main();
});

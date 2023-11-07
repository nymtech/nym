const { createMixFetch, disconnectMixFetch } = require('@nymproject/mix-fetch-node-commonjs');

/**
 * The main entry point
 */
(async () => {
  console.log('Tester is starting up...');

  const addr =
    'D274yd1h3L3pNJzdxE5VgJ7izAsAVMsDrQtFSkKUegfk.8J67cGbcwvrJKF3Kb16HVWWc9AnrFnEibNCm9zCkuVFu@Emswx6KXyjRfq1c2k4d4uD2e6nBSbH1biorCZUei8UNS';

  console.log('About to set up mixFetch...');
  const { mixFetch } = await createMixFetch({
    preferredNetworkRequester: addr,
    clientId: 'node-client1',
    clientOverride: {
      coverTraffic: { disableLoopCoverTrafficStream: true },
      traffic: { disableMainPoissonPacketDistribution: true },
    },
    mixFetchOverride: { requestTimeoutMs: 60000 },
    responseBodyConfigMap: {},
    extra: {},
  });

  globalThis.mixFetch = mixFetch;

  if (!globalThis.mixFetch) {
    console.error('Oh no! Could not create mixFetch');
  } else {
    console.log('Ready!');
  }

  let url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';
  console.log(`Using mixFetch to get ${url}...`);
  const args = { mode: 'unsafe-ignore-cors' };

  let resp = await mixFetch(url, args);
  console.log({ resp });
  const text = await resp.text();

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
  console.log(JSON.stringify({ bufferBytes: buffer.byteLength, blobUrl }, null, 2));
  console.log(blobUrl);
})();

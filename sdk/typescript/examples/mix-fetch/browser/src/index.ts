import { setupMixTunnel, mixFetch } from '@nymproject/mix-fetch';
import { appendOutput, appendImageOutput } from './utils';

async function main() {
  appendOutput('Setting up mixnet tunnel...');
  await setupMixTunnel();
  appendOutput('Tunnel ready.\n');

  // Standard allowlist for the Nym network-requester. The IPR enforces its own
  // exit policy, so the URL must pass that policy regardless of the source.
  let url = 'https://nymtech.net/.wellknown/network-requester/standard-allowed-list.txt';

  appendOutput('Get a text file:');
  appendOutput(`Downloading ${url}...\n`);
  let resp = await mixFetch(url);
  const text = await resp.text();
  appendOutput(text);

  appendOutput('\nGet an image:\n');
  url = 'https://nymtech.net/favicon.svg';
  resp = await mixFetch(url);
  const buffer = await resp.arrayBuffer();
  const type = resp.headers.get('Content-Type') || 'image/svg';
  const blobUrl = URL.createObjectURL(new Blob([buffer], { type }));
  appendImageOutput(blobUrl);
}

window.addEventListener('DOMContentLoaded', () => {
  main().catch((err) => {
    console.error(err);
    appendOutput(`Error: ${err}`);
  });
});

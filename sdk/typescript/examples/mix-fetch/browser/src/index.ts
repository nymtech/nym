import { setupMixTunnel, mixFetch, type SetupMixTunnelOpts } from '@nymproject/mix-fetch';
import { appendOutput, appendImageOutput } from './utils';

// Tunnel configuration. Every field is optional; uncomment to tweak.
//
// `debug: true` turns on smolmix-wasm's verbose tracing so you can watch the
// IPR handshake, DNS lookups, and per-request lifecycle in DevTools. Leave
// it off in production.
const setupOpts: SetupMixTunnelOpts = {
  debug: true,

  // Pin a specific exit IPR. Otherwise auto-discovered from the topology.
  // preferredIpr: 'D1rrUqJY9pesL3pTaMaxLnpZGGYQ4ZpZwpQXCqaeBXTW.6PpFkRvF...',

  // Anonymity / performance trade-off. Cover traffic + Poisson padding
  // smear timing patterns at the cost of bandwidth. Default: both on.
  // disableCoverTraffic: true,
  // disablePoissonTraffic: true,

  // Custom DNS resolvers (over UDP through the IPR). Default: 1.1.1.1 / 8.8.8.8.
  // primaryDns: '9.9.9.9',
  // fallbackDns: '149.112.112.112',

  // Connect / DNS budgets. Defaults: 60s / 30s respectively.
  // connectTimeoutMs: 30_000,
  // dnsTimeoutMs: 15_000,

  // mixFetch redirect chain depth. Default: 5.
  // maxRedirects: 10,
};

async function main() {
  appendOutput('Setting up mixnet tunnel...');
  await setupMixTunnel(setupOpts);
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
  url = 'https://httpbin.org/image/png';
  resp = await mixFetch(url);
  const buffer = await resp.arrayBuffer();
  const type = resp.headers.get('Content-Type') || 'image/png';
  const blobUrl = URL.createObjectURL(new Blob([buffer], { type }));
  appendImageOutput(blobUrl);

  // Per-request header override. smolmix-wasm ships a browser-shape header
  // shim (User-Agent + Accept + Accept-Language + Accept-Encoding); anything
  // you pass in `init.headers` wins over the shim defaults.
  appendOutput('\nOverride User-Agent for one request:\n');
  url = 'https://httpbin.org/headers';
  resp = await mixFetch(url, {
    headers: { 'User-Agent': 'mix-fetch-example/0.1' },
  });
  appendOutput(await resp.text());
}

window.addEventListener('DOMContentLoaded', () => {
  main().catch((err) => {
    console.error(err);
    appendOutput(`Error: ${err}`);
  });
});

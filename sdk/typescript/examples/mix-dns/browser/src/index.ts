import { setupMixTunnel, mixDNS, type SetupMixTunnelOpts } from '@nymproject/mix-dns';

function append(line: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  el.appendChild(document.createTextNode(`${line}\n`));
}

// Tunnel configuration. Every field is optional.
//
// `debug: true` turns on smolmix-wasm's verbose tracing so you can see the
// UDP DNS query and response in DevTools. Leave it off in production.
const setupOpts: SetupMixTunnelOpts = {
  debug: true,

  // Pin a specific exit IPR. Otherwise auto-discovered from the topology.
  // preferredIpr: 'D1rrUqJY9pesL3pTaMaxLnpZGGYQ4ZpZwpQXCqaeBXTW.6PpFkRvF...',

  // DoH resolver overrides, tried in order (IP-literal HTTPS URLs).
  // Default: Cloudflare, Quad9, Google. Set these to pin a specific resolver.
  // dohEndpoints: ['https://9.9.9.9/dns-query'],

  // Per-endpoint DoH timeout. Default: 8s (covers a cold TLS handshake).
  // dnsTimeoutMs: 8_000,
};

// Hostnames cover a mix of cases: the Nym site itself, the de facto smoke
// test (example.com), a major CDN (cloudflare), and a host that takes
// several A records.
const hostnames = ['nymtech.net', 'example.com', 'cloudflare.com', 'github.com'];

async function main() {
  append('Setting up mixnet tunnel...');
  await setupMixTunnel(setupOpts);
  append('Tunnel ready.\n');

  for (const host of hostnames) {
    const start = performance.now();
    try {
      const ip = await mixDNS(host);
      const ms = Math.round(performance.now() - start);
      append(`${host} -> ${ip}  (${ms} ms)`);
    } catch (err) {
      append(`${host} -> error: ${err instanceof Error ? err.message : String(err)}`);
    }
  }
}

window.addEventListener('DOMContentLoaded', () => {
  main().catch((err) => {
    console.error(err);
    append(`Error: ${err}`);
  });
});

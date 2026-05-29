import { setupMixTunnel, mixDNS } from '@nymproject/mix-dns';

function append(line: string) {
  const el = document.getElementById('output') as HTMLPreElement;
  el.appendChild(document.createTextNode(`${line}\n`));
}

async function main() {
  append('Setting up mixnet tunnel...');
  await setupMixTunnel();
  append('Tunnel ready.\n');

  const hostnames = ['nymtech.net', 'example.com', 'cloudflare.com'];
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

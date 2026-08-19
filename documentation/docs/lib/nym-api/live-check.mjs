#!/usr/bin/env node
// Manual integration check against the real Nym APIs. NOT part of the vitest
// suite (which is hermetic) so CI never depends on external uptime.
//
//   node lib/nym-api/live-check.mjs
//
// Mirrors client.ts by hand (client.ts is TS; this stays runnable under bare
// node). If an endpoint moves, this fails loudly and client.ts needs updating.

const NYM_API = 'https://validator.nymtech.net/api';
const NODE_STATUS_API = 'https://mainnet-node-status-api.nymtech.cc';

const get = async (url) => {
  const res = await fetch(url, { signal: AbortSignal.timeout(15_000) });
  if (!res.ok) throw new Error(`${res.status} ${url}`);
  return res.json();
};

const checks = [
  ['circulating-supply', async () => {
    const s = await get(`${NYM_API}/v1/circulating-supply`);
    return `${(Number(s.circulating_supply.amount) / 1e6).toLocaleString()} NYM circulating`;
  }],
  ['chain-status', async () => {
    const s = await get(`${NYM_API}/v1/network/chain-status`);
    return `connected_nyxd=${s.connected_nyxd}`;
  }],
  ['network summary', async () => {
    const s = await get(`${NODE_STATUS_API}/v2/summary`);
    return `${s.total_nodes} nodes, ${s.gateways.bonded.count} gateways (${s.gateways.bonded.entry} entry / ${s.gateways.bonded.exit} exit)`;
  }],
  ['list gateways', async () => {
    const p = await get(`${NODE_STATUS_API}/v2/gateways/skinny?page=0&size=1`);
    return `${p.total} total; sample ${p.items[0].gateway_identity_key} perf=${p.items[0].performance}`;
  }],
  ['gateway by id', async () => {
    const p = await get(`${NODE_STATUS_API}/v2/gateways/skinny?page=0&size=1`);
    const id = p.items[0].gateway_identity_key;
    const g = await get(`${NODE_STATUS_API}/v2/gateways/${id}`);
    return `${g.gateway_identity_key} bonded=${g.bonded} routing=${g.routing_score}`;
  }],
];

let failed = 0;
for (const [name, fn] of checks) {
  try {
    console.log(`  ok   ${name}: ${await fn()}`);
  } catch (e) {
    failed += 1;
    console.log(`  FAIL ${name}: ${e.message}`);
  }
}
console.log(failed ? `\n${failed} check(s) failed.` : '\nAll live checks passed.');
process.exit(failed ? 1 : 0);

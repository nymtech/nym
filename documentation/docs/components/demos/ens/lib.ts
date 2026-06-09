// ENS demo logic, ported from wasm/ens-demo/index.js and decoupled from the DOM.
// The interesting bit is buildProvider(): ethers v6 lets us swap its HTTP
// transport by assigning FetchRequest.getUrlFunc, so every JSON-RPC call routes
// through mixFetch. To ethers, mixFetch looks like any fetch; to the mixnet,
// ethers looks like any caller.

import { JsonRpcProvider, FetchRequest } from 'ethers';
import type { MixFetchFn } from '../shared/mixTunnel';
import type { Colour } from '../shared/ui';

export type LogFn = (msg: string, colour?: Colour) => void;
export type LogLinkFn = (prefix: string, url: string) => void;

export function formatSize(bytes: number): string {
  if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + ' MB';
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return bytes + ' B';
}

// base58btc decoder (Bitcoin/IPFS variant; alphabet excludes 0/O/I/l), used to
// take CIDv0 strings apart into their raw multihash bytes.
const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
const BASE58_MAP: Record<string, number> = (() => {
  const m: Record<string, number> = Object.create(null);
  for (let i = 0; i < BASE58_ALPHABET.length; i++) m[BASE58_ALPHABET[i]] = i;
  return m;
})();

function base58Decode(str: string): Uint8Array {
  let zeros = 0;
  while (zeros < str.length && str[zeros] === '1') zeros++;
  let value = 0n;
  for (const c of str) {
    if (!(c in BASE58_MAP)) throw new Error(`invalid base58 char: ${c}`);
    value = value * 58n + BigInt(BASE58_MAP[c]);
  }
  const bytes: number[] = [];
  while (value > 0n) {
    bytes.unshift(Number(value & 0xffn));
    value >>= 8n;
  }
  for (let i = 0; i < zeros; i++) bytes.unshift(0);
  return new Uint8Array(bytes);
}

// RFC 4648 base32, lowercase, no padding (the multibase 'b' alphabet CIDv1 uses).
const BASE32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

function base32Encode(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let output = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      output += BASE32_ALPHABET[(value >>> (bits - 5)) & 0x1f];
      bits -= 5;
    }
  }
  if (bits > 0) output += BASE32_ALPHABET[(value << (5 - bits)) & 0x1f];
  return output;
}

// Convert CIDv0 ("Qm...") to CIDv1 ("bafybe..."). Pure re-encoding: same content,
// same sha2-256 digest, different envelope. Needed for IPFS subdomain gateways,
// whose DNS labels are case-insensitive (base58btc is case-sensitive; base32
// lowercase survives the round trip). Pass through unchanged if not a canonical
// CIDv0.
export function cidV0ToV1(cid: string): string {
  if (!cid.startsWith('Qm') || cid.length !== 46) return cid;
  let decoded: Uint8Array;
  try {
    decoded = base58Decode(cid);
  } catch {
    return cid;
  }
  if (decoded.length !== 34 || decoded[0] !== 0x12 || decoded[1] !== 0x20) return cid;
  const v1 = new Uint8Array(36);
  v1[0] = 0x01; // CID version 1
  v1[1] = 0x70; // codec: dag-pb (preserves CIDv0's implicit codec)
  v1.set(decoded, 2); // copy multihash verbatim
  return 'b' + base32Encode(v1);
}

// Expand a gateway template against a CID. {cid} placeholder supports path form
// (https://gw/ipfs/{cid}/) and subdomain form (https://{cid}.ipfs.gw/); the
// latter forces CIDv0 -> CIDv1. Legacy no-placeholder form is path-only.
export function expandGatewayUrl(gateway: string, cid: string): string {
  if (gateway.includes('{cid}')) {
    const isSubdomain = /\{cid\}\.ipfs\./.test(gateway) || /\{cid\}\.ipns\./.test(gateway);
    const finalCid = isSubdomain ? cidV0ToV1(cid) : cid;
    return gateway.replace('{cid}', finalCid);
  }
  return `${gateway}${cid}/`;
}

export interface FlatResponse {
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: Uint8Array;
}

// Normalise headers to a lowercase-keyed plain object so downstream code only
// ever sees one shape (mix-fetch v2 returns a real Response; smolmix's older
// shape returned [k,v] tuples).
export function headersToObj(headers: unknown): Record<string, string> {
  const out: Record<string, string> = {};
  if (!headers) return out;
  if (headers instanceof Headers || headers instanceof Map) {
    for (const [k, v] of (headers as Headers).entries()) out[k.toLowerCase()] = v;
    return out;
  }
  if (Array.isArray(headers)) {
    for (const [k, v] of headers) out[String(k).toLowerCase()] = v;
    return out;
  }
  if (typeof headers === 'object') {
    for (const [k, v] of Object.entries(headers as Record<string, string>)) out[k.toLowerCase()] = v;
  }
  return out;
}

// Single boundary into the mixnet: flatten the Response to {status, headers
// (lowercased), body (bytes)} once, because downstream does its own
// decompression / TextDecoder work on the raw wire payload.
export async function callMixFetch(mixFetch: MixFetchFn, url: string, init?: RequestInit): Promise<FlatResponse> {
  const res = await mixFetch(url, init || {});
  const body = new Uint8Array(await res.arrayBuffer());
  return { status: res.status, statusText: res.statusText, headers: headersToObj(res.headers), body };
}

// Decompress a body if the server set Content-Encoding. Native fetch does this
// transparently; smolmix returns raw wire bytes. Brotli (br) is not in
// DecompressionStream; pass it through.
export async function decompressBody(body: Uint8Array, headers: Record<string, string>): Promise<Uint8Array> {
  if (!body || body.byteLength === 0) return body;
  const enc = (headers['content-encoding'] || '').toLowerCase().trim();
  if (!enc || enc === 'identity') return body;

  let format: 'gzip' | 'deflate' | 'deflate-raw' | null = null;
  if (enc === 'gzip' || enc === 'x-gzip') format = 'gzip';
  else if (enc === 'deflate') format = 'deflate';
  else if (enc === 'deflate-raw') format = 'deflate-raw';
  if (!format) return body;

  // body is always a plain ArrayBuffer-backed Uint8Array at runtime; the cast
  // sidesteps the TS 5.7 generic-typed-array vs BlobPart (ArrayBuffer) mismatch.
  const stream = new Blob([body as BlobPart]).stream().pipeThrough(new DecompressionStream(format));
  const buf = await new Response(stream).arrayBuffer();
  return new Uint8Array(buf);
}

export function stripContentEncoding(headers: Record<string, string>): Record<string, string> {
  const out = { ...headers };
  delete out['content-encoding'];
  return out;
}

export interface Fingerprint {
  title: string | null;
  h1: string | null;
  description: string | null;
  ogTitle: string | null;
  counts: { links: number; images: number; scripts: number; stylesheets: number; headings: number };
  bodyTextLen: number;
}

// Extract a human-meaningful fingerprint from HTML for visual comparison against
// the same page in another tab. DOMParser runs no scripts and fetches nothing.
export function htmlFingerprint(text: string): Fingerprint {
  const doc = new DOMParser().parseFromString(text, 'text/html');
  const get = (sel: string, attr?: string): string | null => {
    const el = doc.querySelector(sel);
    if (!el) return null;
    return attr ? el.getAttribute(attr) : el.textContent?.trim() ?? null;
  };
  return {
    title: get('title'),
    h1: get('h1'),
    description: get('meta[name="description"]', 'content') || get('meta[property="og:description"]', 'content'),
    ogTitle: get('meta[property="og:title"]', 'content'),
    counts: {
      links: doc.querySelectorAll('a').length,
      images: doc.querySelectorAll('img').length,
      scripts: doc.querySelectorAll('script').length,
      stylesheets: doc.querySelectorAll('link[rel="stylesheet"], style').length,
      headings: doc.querySelectorAll('h1, h2, h3, h4, h5, h6').length,
    },
    bodyTextLen: doc.body ? doc.body.textContent!.replace(/\s+/g, ' ').trim().length : 0,
  };
}

export function renderFingerprint(fp: Fingerprint, totalBytes: number): string {
  const orNone = (s: string | null) => s || '(none)';
  return [
    'page fingerprint:',
    '',
    `  title:        ${orNone(fp.title)}`,
    `  H1:           ${orNone(fp.h1)}`,
    `  description:  ${orNone(fp.description)}`,
    `  og:title:     ${orNone(fp.ogTitle)}`,
    '',
    `  ${fp.counts.links} links, ${fp.counts.images} images, ${fp.counts.scripts} scripts, ${fp.counts.stylesheets} stylesheets, ${fp.counts.headings} headings`,
    `  body text:    ${fp.bodyTextLen.toLocaleString()} chars after stripping tags`,
    `  HTML size:    ${formatSize(totalBytes)} raw response body`,
  ].join('\n');
}

// Bridge ethers' FetchRequest to mixFetch. Every JSON-RPC call routes through
// the mixnet; we decompress (smolmix doesn't) and log per-call timing/selector.
export function buildProvider(rpcUrl: string, mixFetch: MixFetchFn, ensLog: LogFn): JsonRpcProvider {
  const base = new FetchRequest(rpcUrl);

  base.getUrlFunc = async (req: FetchRequest) => {
    const t0 = performance.now();
    const raw = await callMixFetch(mixFetch, req.url, {
      method: req.method,
      headers: req.headers,
      body: (req.body ?? undefined) as BodyInit | undefined,
    });
    const ms = (performance.now() - t0).toFixed(0);

    const body = await decompressBody(raw.body, raw.headers);
    const headers = stripContentEncoding(raw.headers);

    ensLog(`  RPC ${req.method} ${req.url} -> ${raw.status} ${raw.statusText}`);
    ensLog(`      ${raw.body.byteLength}B wire, ${body.byteLength}B decoded, ${ms} ms`);

    if (req.body) {
      try {
        const parsed = JSON.parse(new TextDecoder().decode(req.body));
        const data: string = parsed.params?.[0]?.data || '';
        const selector = data.slice(0, 10);
        ensLog(`    -> method=${parsed.method} selector=${selector} args=0x${data.slice(10)}`);
      } catch {
        /* not all RPC bodies are eth_call with calldata */
      }
    }
    if (body.byteLength) {
      try {
        ensLog(`    <- ${new TextDecoder().decode(body)}`);
      } catch {
        /* binary body */
      }
    }

    return { statusCode: raw.status, statusMessage: raw.statusText, headers, body };
  };

  // staticNetwork skips the eth_chainId discovery probe: one round trip saved.
  return new JsonRpcProvider(base, 'mainnet', { staticNetwork: true });
}

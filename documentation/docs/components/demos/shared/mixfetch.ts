// Shared mixnet + ethers helpers for demos that route HTTP through mixFetch.
// railgun uses these. ens deliberately keeps its own buildProvider: it is the
// verbose teaching variant that logs per-call timing and the decoded eth_call
// selector, so it is not a candidate for consolidation onto the quiet one here.

import { FetchRequest, JsonRpcProvider, type Networkish } from 'ethers';
import type { MixFetchFn } from './mixTunnel';

export interface FlatResponse {
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: Uint8Array;
}

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

export async function callMixFetch(mixFetch: MixFetchFn, url: string, init?: RequestInit): Promise<FlatResponse> {
  const res = await mixFetch(url, init || {});
  const body = new Uint8Array(await res.arrayBuffer());
  return { status: res.status, statusText: res.statusText, headers: headersToObj(res.headers), body };
}

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
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

export function stripContentEncoding(headers: Record<string, string>): Record<string, string> {
  const out = { ...headers };
  delete out['content-encoding'];
  return out;
}

// The getUrl adapter ethers uses: route the request through mixFetch, decompress,
// rename the response fields ethers expects.
function makeGetUrl(mixFetch: MixFetchFn) {
  return async (req: FetchRequest) => {
    const raw = await callMixFetch(mixFetch, req.url, {
      method: req.method,
      headers: req.headers,
      body: (req.body ?? undefined) as BodyInit | undefined,
    });
    const body = await decompressBody(raw.body, raw.headers);
    return {
      statusCode: raw.status,
      statusMessage: raw.statusText,
      headers: stripContentEncoding(raw.headers),
      body,
    };
  };
}

// A JsonRpcProvider whose transport is mixFetch (per-instance override).
export function buildProvider(rpcUrl: string, mixFetch: MixFetchFn, network: Networkish): JsonRpcProvider {
  const base = new FetchRequest(rpcUrl);
  base.getUrlFunc = makeGetUrl(mixFetch);
  return new JsonRpcProvider(base, network, { staticNetwork: true });
}

// Install a single global FetchRequest URL handler so EVERY ethers HTTP request
// (including providers a library constructs internally from URL strings) routes
// through mixFetch. Requires a single ethers instance across the bundle: the
// `ethers$` alias in next.config.js enforces that.
let globalRoutingInstalled = false;
export function installGlobalMixFetchRouting(mixFetch: MixFetchFn): void {
  if (globalRoutingInstalled) return;
  FetchRequest.registerGetUrl(makeGetUrl(mixFetch));
  globalRoutingInstalled = true;
}

// Retry wrapper for mixnet-routed RPC calls: the first request on a cold mixnet
// path pays TCP-connect + TLS-handshake time, and the underlying ethers request
// can time out on it; the retry finds the pool warm.
export async function withRetry<T>(
  fn: () => Promise<T>,
  label: string,
  opts: { attempts?: number; delayMs?: number; log?: (msg: string, colour?: string) => void } = {},
): Promise<T> {
  const { attempts = 3, delayMs = 3000, log } = opts;
  let lastErr: unknown;
  for (let i = 1; i <= attempts; i++) {
    try {
      return await fn();
    } catch (e: any) {
      lastErr = e;
      const msg = e.shortMessage || e.message || String(e);
      if (i < attempts) {
        log?.(`${label} attempt ${i}/${attempts} failed (${msg}), retrying in ${delayMs / 1000}s...`, 'orange');
        await new Promise((r) => setTimeout(r, delayMs));
      } else {
        log?.(`${label} failed after ${attempts} attempts: ${msg}`, 'red');
      }
    }
  }
  throw lastErr;
}

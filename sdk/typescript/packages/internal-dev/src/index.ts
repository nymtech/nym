// smolmix SDK playground.
//
// Exercises the four packages (mix-tunnel, mix-fetch, mix-dns, mix-websocket)
// against a live mixnet. Each section is independent; you can drive them in
// any order once the tunnel is up.

import {
  setupMixTunnel,
  disconnectMixTunnel,
  getTunnelState,
  SetupMixTunnelOpts,
  TunnelState,
} from '@nymproject/mix-tunnel';
import { mixFetch } from '@nymproject/mix-fetch';
import { mixDNS } from '@nymproject/mix-dns';
import { MixWebSocket } from '@nymproject/mix-websocket';

const $ = <T extends HTMLElement = HTMLElement>(id: string): T => {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} not found`);
  return el as T;
};

const log = (id: string, msg: string) => {
  const el = $(id) as HTMLPreElement;
  el.textContent += `${msg}\n`;
  el.scrollTop = el.scrollHeight;
};

const setBtnDisabled = (id: string, disabled: boolean) => {
  ($(id) as HTMLButtonElement).disabled = disabled;
};

const renderTunnelState = (state: TunnelState) => {
  const el = $('tunnel-state');
  const label = state.reason ? `${state.state} (${state.reason})` : state.state;
  el.textContent = label;
  el.classList.remove('ready', 'failed');
  if (state.state === 'ready') el.classList.add('ready');
  if (state.state === 'failed') el.classList.add('failed');
};

// Tunnel ==============================================================

/** Read a string field, returning undefined for empty input. */
const strOpt = (id: string): string | undefined => {
  const v = ($(id) as HTMLInputElement).value.trim();
  return v.length === 0 ? undefined : v;
};

/** Read a number field, returning undefined for empty input. */
const numOpt = (id: string): number | undefined => {
  const v = ($(id) as HTMLInputElement).value.trim();
  if (v.length === 0) return undefined;
  const n = Number(v);
  return Number.isFinite(n) ? n : undefined;
};

/** Read a checkbox, returning undefined when unchecked (so we don't override
 *  a wasm-side default with an explicit `false`). */
const boolOpt = (id: string): boolean | undefined => {
  const checked = ($(id) as HTMLInputElement).checked;
  return checked ? true : undefined;
};

/** Build the `SetupMixTunnelOpts` bag from the form. Empty fields are omitted
 *  so the wasm-side defaults apply. */
const readSetupOpts = (): SetupMixTunnelOpts => {
  const opts: Record<string, unknown> = {};
  const maybeSet = (key: string, value: unknown) => {
    if (value !== undefined) opts[key] = value;
  };

  // Honour the "use random IPR" checkbox: when checked, omit preferredIpr
  // entirely so smolmix auto-discovers. When unchecked, use the input value.
  const useRandomIpr = ($('opt-randomIpr') as HTMLInputElement).checked;
  maybeSet('preferredIpr', useRandomIpr ? undefined : strOpt('opt-preferredIpr'));
  maybeSet('clientId', strOpt('opt-clientId'));
  maybeSet('storagePassphrase', strOpt('opt-storagePassphrase'));
  maybeSet('forceTls', boolOpt('opt-forceTls'));
  maybeSet('disablePoissonTraffic', boolOpt('opt-disablePoissonTraffic'));
  maybeSet('disableCoverTraffic', boolOpt('opt-disableCoverTraffic'));
  maybeSet('openReplySurbs', numOpt('opt-openReplySurbs'));
  maybeSet('dataReplySurbs', numOpt('opt-dataReplySurbs'));
  maybeSet('primaryDns', strOpt('opt-primaryDns'));
  maybeSet('fallbackDns', strOpt('opt-fallbackDns'));
  maybeSet('dnsTimeoutMs', numOpt('opt-dnsTimeoutMs'));
  maybeSet('connectTimeoutMs', numOpt('opt-connectTimeoutMs'));
  maybeSet('tcpKeepaliveMs', numOpt('opt-tcpKeepaliveMs'));
  maybeSet('tcpBufferSize', numOpt('opt-tcpBufferSize'));
  maybeSet('maxRedirects', numOpt('opt-maxRedirects'));
  maybeSet('debug', boolOpt('opt-debug'));

  return opts as SetupMixTunnelOpts;
};

$('btn-setup').addEventListener('click', async () => {
  setBtnDisabled('btn-setup', true);
  const opts = readSetupOpts();
  log('tunnel-log', `setupMixTunnel(${JSON.stringify(opts)})...`);
  const t0 = performance.now();
  try {
    await setupMixTunnel(opts);
    const ms = (performance.now() - t0).toFixed(0);
    log('tunnel-log', `tunnel ready in ${ms} ms`);
    renderTunnelState(await getTunnelState());
  } catch (e) {
    log('tunnel-log', `error: ${e}`);
  } finally {
    setBtnDisabled('btn-setup', false);
  }
});

$('btn-disconnect').addEventListener('click', async () => {
  log('tunnel-log', 'disconnectMixTunnel()...');
  try {
    await disconnectMixTunnel();
    log('tunnel-log', 'disconnected (tunnel unusable until page reload)');
    renderTunnelState(await getTunnelState());
  } catch (e) {
    log('tunnel-log', `disconnect error: ${e}`);
  }
});

// 2. mix-fetch ========================================================

async function runFetch(url: string, asImage: boolean) {
  // Read the UA override field — if non-empty, pass as a header on this
  // fetch. Since the wasm-side shim only injects headers the caller didn't
  // set, an override here wins. Empty field = wasm shim default applies.
  const uaOverride = ($('opt-userAgent') as HTMLInputElement).value.trim();
  const init: RequestInit | undefined = uaOverride
    ? { headers: { 'User-Agent': uaOverride } }
    : undefined;

  log('fetch-log', `GET ${url}${uaOverride ? ` (UA=${uaOverride.slice(0, 60)}${uaOverride.length > 60 ? '...' : ''})` : ''}`);
  const t0 = performance.now();
  try {
    const resp = await mixFetch(url, init);
    const ms = (performance.now() - t0).toFixed(0);
    log('fetch-log', `${resp.status} ${resp.statusText} (${ms} ms, ${resp.headers.get('content-type') ?? '?'})`);

    // On non-2xx, surface the body as text regardless of asImage — the
    // server's error message is what we need to diagnose 4xx/5xx, and
    // trying to render a 403 HTML error inside an <img> tag is useless.
    if (!resp.ok) {
      const errText = await resp.text();
      log('fetch-log', errText.length > 600 ? `${errText.slice(0, 600)}\n... (${errText.length} bytes total)` : errText);
      return;
    }

    if (asImage) {
      const buf = await resp.arrayBuffer();
      const type = resp.headers.get('content-type') ?? 'image/svg+xml';
      const blobUrl = URL.createObjectURL(new Blob([buf], { type }));
      const img = document.createElement('img');
      img.src = blobUrl;
      img.style.maxWidth = '64px';
      img.style.marginRight = '4px';
      $('fetch-image-output').appendChild(img);
    } else {
      const text = await resp.text();
      log('fetch-log', text.length > 400 ? `${text.slice(0, 400)}\n... (${text.length} bytes total)` : text);
    }
  } catch (e) {
    log('fetch-log', `error: ${e}`);
  }
}

$('btn-fetch-text').addEventListener('click', () => {
  const url = ($('fetch-url') as HTMLInputElement).value.trim();
  if (url) runFetch(url, false);
});

$('btn-fetch-image').addEventListener('click', (ev) => {
  const url = (ev.currentTarget as HTMLButtonElement).dataset.url;
  if (url) runFetch(url, true);
});

// Cloudflare's diagnostic endpoint. Returns plaintext key=value lines
// showing what cloudflare sees about our request — `uag` reflects the
// User-Agent header cloudflare actually received (confirms our shim is
// reaching their edge), `ip` shows the IPR-egress IP they observe,
// `tls` / `http` show the protocol versions negotiated, `colo` shows
// which cloudflare edge city handled us. Way more useful than
// "succeeded/failed" — it tells us the request's full profile from
// cloudflare's viewpoint.
$('btn-fetch-cf-trace').addEventListener('click', (ev) => {
  const url = (ev.currentTarget as HTMLButtonElement).dataset.url;
  if (url) runFetch(url, false);
});

// 3. mix-dns ==========================================================

$('btn-dns').addEventListener('click', async () => {
  const host = ($('dns-host') as HTMLInputElement).value.trim();
  if (!host) return;
  log('dns-log', `mixDNS('${host}')...`);
  const t0 = performance.now();
  try {
    const ip = await mixDNS(host);
    const ms = (performance.now() - t0).toFixed(0);
    log('dns-log', `${host} => ${ip} (${ms} ms)`);
  } catch (e) {
    log('dns-log', `error: ${e}`);
  }
});

// 4. mix-websocket ====================================================

let currentWs: MixWebSocket | undefined;

$('btn-ws-connect').addEventListener('click', async () => {
  const url = ($('ws-url') as HTMLInputElement).value.trim();
  if (!url) return;
  log('ws-log', `connect ${url}`);
  setBtnDisabled('btn-ws-connect', true);

  const ws = new MixWebSocket(url);
  currentWs = ws;

  ws.addEventListener('open', () => {
    log('ws-log', 'open');
    setBtnDisabled('btn-ws-send', false);
    setBtnDisabled('btn-ws-close', false);
  });
  ws.addEventListener('message', (ev) => {
    const e = ev as MessageEvent;
    const data = typeof e.data === 'string'
      ? `text: ${e.data}`
      : `binary: ${e.data instanceof ArrayBuffer ? `${e.data.byteLength} bytes` : String(e.data)}`;
    log('ws-log', `msg ${data}`);
  });
  ws.addEventListener('close', (ev) => {
    const e = ev as CloseEvent;
    log('ws-log', `close code=${e.code} reason=${JSON.stringify(e.reason)}`);
    setBtnDisabled('btn-ws-connect', false);
    setBtnDisabled('btn-ws-send', true);
    setBtnDisabled('btn-ws-close', true);
    currentWs = undefined;
  });
  ws.addEventListener('error', (ev) => {
    // MixWebSocket extends the standard `error` Event with an optional
    // `.message` so we can surface the underlying cause in the playground.
    const msg = (ev as Event & { message?: string }).message ?? '(no detail)';
    log('ws-log', `error: ${msg}`);
    setBtnDisabled('btn-ws-connect', false);
    setBtnDisabled('btn-ws-send', true);
    setBtnDisabled('btn-ws-close', true);
    currentWs = undefined;
  });
});

$('btn-ws-send').addEventListener('click', async () => {
  if (!currentWs) return;
  try {
    await currentWs.send('hello');
    log('ws-log', 'sent "hello"');
  } catch (e) {
    log('ws-log', `send error: ${e}`);
  }
});

$('btn-ws-close').addEventListener('click', async () => {
  if (!currentWs) return;
  try {
    await currentWs.close(1000, 'done');
  } catch (e) {
    log('ws-log', `close error: ${e}`);
  }
});

// Toggle the preferredIpr text field's disabled state based on the random-IPR
// checkbox. Mirrors the smolmix-wasm internal-dev's pattern: when "use random
// IPR" is checked, the pre-filled IPR address is greyed out and ignored.
$('opt-randomIpr').addEventListener('change', () => {
  const useRandom = ($('opt-randomIpr') as HTMLInputElement).checked;
  ($('opt-preferredIpr') as HTMLInputElement).disabled = useRandom;
});

// Initial state poll on page load.
window.addEventListener('DOMContentLoaded', async () => {
  // Pre-populate `clientId` with a fresh random handle each page load.
  // Matches wasm/smolmix/internal-dev's convention — keeps each run on a
  // clean key/storage slot so we don't accidentally pin one identity across
  // many test cycles (which would change cover-traffic + topology choice).
  ($('opt-clientId') as HTMLInputElement).value =
    'smolmix-' + Math.random().toString(36).slice(2, 8);

  try {
    renderTunnelState(await getTunnelState());
  } catch {
    // pre-setup state is `connecting`; ignore.
  }
});

import { openUrl } from '@tauri-apps/plugin-opener';

const ALLOWED_SCHEMES = new Set(['https:', 'http:']);

/**
 * Opens a URL in the system browser via Tauri opener.
 * Only http and https are allowed to reduce open-redirect / unexpected scheme risk.
 */
export async function safeOpenUrl(url: string): Promise<void> {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error('Invalid URL');
  }
  if (!ALLOWED_SCHEMES.has(parsed.protocol)) {
    throw new Error(`URL scheme not allowed: ${parsed.protocol}`);
  }
  await openUrl(url);
}

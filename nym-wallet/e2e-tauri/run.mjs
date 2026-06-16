// Skip-not-fail launcher for the optional native-webview leg (design D5).
// tauri-driver only works on Linux/Windows; on macOS (no WKWebView driver) or when the
// required drivers are missing, this exits 0 with a clear message instead of failing.
import { execSync } from 'node:child_process';
import { platform } from 'node:os';

const skip = (why) => {
  console.log(`[e2e:tauri] skipped — ${why}`);
  process.exit(0);
};

const has = (bin) => {
  try {
    execSync(`command -v ${bin}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
};

if (platform() === 'darwin') skip('macOS has no WKWebView driver (tauri-driver unsupported) — use the Playwright suite locally.');
if (platform() === 'win32' ? !has('msedgedriver') : !(has('WebKitWebDriver') || has('webkit2gtk-driver')))
  skip('platform webdriver not found (install webkit2gtk-driver on Linux / msedgedriver on Windows).');
if (!has('tauri-driver')) skip('tauri-driver not found — run `cargo install tauri-driver --locked`.');

execSync('wdio run ./wdio.conf.ts', { stdio: 'inherit' });

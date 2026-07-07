import { spawn, ChildProcess } from 'node:child_process';
import path from 'node:path';

/**
 * Optional native-webview e2e config (design D4): drives the packaged Tauri binary through
 * tauri-driver + WebdriverIO. Launched via `e2e-tauri/run.mjs` (skip-not-fail on macOS).
 *
 * The mock binary (`pnpm tauri:build:mock`, built with WALLET_MOCK_FAMILIES=on and the
 * `tauri.mock.conf.json` override) boots its window directly into `main.mock.html` — the mock
 * app shell, no Tauri auth/login — so the owner journey runs on launch. Other personas are
 * reached by in-webview navigation (see `appUrl` in the spec); on Linux/WebKitGTK the asset
 * scheme is `tauri://localhost/` (the mock config sets `useHttpsScheme: false`).
 */

let tauriDriver: ChildProcess | undefined;

// `--no-bundle` release binary. The Cargo workspace target dir is at the wallet root
// (`<wallet>/target/release`), not `src-tauri/target`. mainBinaryName is "NymWallet".
const APPLICATION = path.resolve(__dirname, 'target/release/NymWallet');

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./e2e-tauri/**/*.tauri.ts'],
  maxInstances: 1,
  capabilities: [
    {
      // tauri-driver reads this and attaches to the native webview.
      // @ts-expect-error tauri:options is a tauri-driver extension, not in the base WdIO types.
      'tauri:options': { application: APPLICATION },
    },
  ],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 120_000 },
  reporters: ['spec'],
  logLevel: 'warn',
  // tauri-driver listens on 4444 by default.
  hostname: '127.0.0.1',
  port: 4444,

  onPrepare: () => {
    tauriDriver = spawn('tauri-driver', [], { stdio: [null, process.stdout, process.stderr] });
  },
  onComplete: () => {
    tauriDriver?.kill();
  },
};

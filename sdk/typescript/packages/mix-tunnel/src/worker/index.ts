import { loadWasm } from './wasm-loading';
import { run } from './main';

async function main() {
  await loadWasm();
  await run();
}

main().catch((e: unknown) => {
  // eslint-disable-next-line no-console
  console.error('Unhandled exception in mix-tunnel worker', e);
});

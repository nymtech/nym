import { loadWasm } from './wasm-loading';
import { run } from './main';

async function main() {
  await loadWasm();
  await run();
}

main().catch((e: any) => console.error('Unhandled exception in mixFetch worker', e));

// Async wrapper — ensures WASM dependencies resolve before index.js runs.
import('./index.js').catch((e) =>
  console.error('Failed to load index.js:', e)
);

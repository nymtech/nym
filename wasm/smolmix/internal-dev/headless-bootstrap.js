// Async wrapper — ensures WASM dependencies resolve before headless.js runs.
import('./headless.js').catch((e) =>
  console.error('Failed to load headless.js:', e)
);

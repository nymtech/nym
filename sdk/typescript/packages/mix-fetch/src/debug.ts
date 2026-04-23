declare global {
  var __MIX_FETCH_DEBUG__: boolean | undefined;
}

/**
 * Rollup replaces `globalThis.__MIX_FETCH_DEBUG__` at build time.
 */
export const isMixFetchDebugEnabled = () => globalThis.__MIX_FETCH_DEBUG__ === true;

/**
 * Use a callback so debug-only work stays behind the flag.
 */
export const runIfDebugEnabled = (fn: () => void) => {
  if (!isMixFetchDebugEnabled()) {
    return;
  }

  fn();
};

export {};

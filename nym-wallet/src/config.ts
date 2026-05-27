/** Opt-in for production builds: set `NYM_WALLET_INTERNAL_DOCS=true` when running webpack (rare debugging). */
const internalDocsEnvOverride = process.env.NYM_WALLET_INTERNAL_DOCS === 'true';

export const config = {
  IS_DEV_MODE: process.env.NODE_ENV === 'development',
  LOG_TAURI_OPERATIONS: process.env.NODE_ENV === 'development',
  /** Arbitrary `invoke` playground; off in production unless `NYM_WALLET_INTERNAL_DOCS` is set at build time. */
  INTERNAL_DOCS_ENABLED: process.env.NODE_ENV !== 'production' || internalDocsEnvOverride,
};

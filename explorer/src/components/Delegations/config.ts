export const config = {
  IS_DEV_MODE: process.env.NODE_ENV === 'development',
  LOG_TAURI_OPERATIONS: process.env.NODE_ENV === 'development',
};

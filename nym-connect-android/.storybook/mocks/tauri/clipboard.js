/**
 * This is a mock for Tauri's API package (@tauri-apps/api/clipboard), to prevent stories from being excluded, because they either use
 * or import dependencies that use Tauri.
 */

module.exports = {
  writeText: async () => undefined,
}

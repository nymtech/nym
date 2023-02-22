/**
 * This is a mock for Tauri's API package (@tauri-apps/api), to prevent stories from being excluded, because they either use
 * or import dependencies that use Tauri.
 */
module.exports = {
  invoke: (operation, args) => {
    console.error(
      `Tauri cannot be used in Storybook. The operation requested was "${operation}". You can add mock responses to "nym_connect/.storybook/mocks/tauri.js" if you need. The default response is "void".`,
    );
    return new Promise((resolve, reject) => {
      reject(new Error(`Tauri operation ${operation} not available in storybook.`));
    });
  },
};

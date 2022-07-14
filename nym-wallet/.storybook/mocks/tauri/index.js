/**
 * This is a mock for Tauri's API package (@tauri-apps/api), to prevent stories from being excluded, because they either use
 * or import dependencies that use Tauri.
 */
module.exports = {
  invoke: (operation, args) => {
    switch (operation) {
      case 'get_balance': {
        return {
          amount: {
            amount: '100',
            denom: 'nymt',
          },
          printable_balance: '100 NYMT',
        };
      }
      case 'delegate_to_mixnode': {
        return {
          logs_json: '[]',
          data_json: '{}',
          transaction_hash: '12345',
        };
      }
      case 'simulate_send': {
        return {
          amount: {
            amount: '0.01',
            denom: 'nym',
          },
        };
      }
    }

    console.error(
      `Tauri cannot be used in Storybook. The operation requested was "${operation}". You can add mock responses to "nym_wallet/.storybook/mocks/tauri.js" if you need. The default response is "void".`,
    );
    return new Promise((resolve, reject) => {
      reject(new Error(`Tauri operation ${operation} not available in storybook.`));
    });
  },
};

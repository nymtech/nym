const delegations = [
  {
    mix_id: 1234,
    node_identity: 'FiojKW7oY9WQmLCiYAsCA21tpowZHS6zcUoyYm319p6Z',
    delegated_on_iso_datetime: new Date(2021, 1, 1).toDateString(),
    unclaimed_rewards: { amount: '0.05', denom: 'nym' },
    amount: { amount: '10', denom: 'nym' },
    owner: '',
    block_height: BigInt(100),
    cost_params: {
      profit_margin_percent: '0.04',
      interval_operating_cost: {
        amount: '20',
        denom: 'nym',
      },
    },
    stake_saturation: '0.2',
    avg_uptime_percent: 0.5,
    accumulated_by_delegates: { amount: '0', denom: 'nym' },
    accumulated_by_operator: { amount: '0', denom: 'nym' },
    uses_vesting_contract_tokens: false,
    pending_events: [],
    mixnode_is_unbonding: false,
    errors: null,
  },
  {
    mix_id: 5678,
    node_identity: 'DT8S942S8AQs2zKHS9SVo1GyHmuca3pfL2uLhLksJ3D8',
    unclaimed_rewards: { amount: '0.1', denom: 'nym' },
    amount: { amount: '100', denom: 'nym' },
    delegated_on_iso_datetime: new Date(2021, 1, 2).toDateString(),
    owner: '',
    block_height: BigInt(4000),
    stake_saturation: '0.5',
    avg_uptime_percent: 0.1,
    cost_params: {
      profit_margin_percent: '0.04',
      interval_operating_cost: {
        amount: '60',
        denom: 'nym',
      },
    },
    accumulated_by_delegates: { amount: '0', denom: 'nym' },
    accumulated_by_operator: { amount: '0', denom: 'nym' },
    uses_vesting_contract_tokens: true,
    pending_events: [],
    mixnode_is_unbonding: false,
    errors: null,
  },
];


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
      case 'get_delegation_summary': {
        return {
          delegations,
          total_delegations: {
            amount: '1000',
            denom: 'nymt',
          },
          total_rewards: {
            amount: '42',
            denom: 'nymt',
          },
        };
      }
      case 'get_pending_delegation_events' : {
        return [];
      }
      case 'migrate_vested_delegations': {
        delegations[1].uses_vesting_contract_tokens = false;
        return {};
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

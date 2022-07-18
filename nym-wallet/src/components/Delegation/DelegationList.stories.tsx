import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { DelegationWithEverything } from '@nymproject/types';
import { DelegationList } from './DelegationList';

export default {
  title: 'Delegation/Components/Delegation List',
  component: DelegationList,
} as ComponentMeta<typeof DelegationList>;

const explorerUrl = 'https://sandbox-explorer.nymtech.net/network-components/mixnodes';

export const items: DelegationWithEverything[] = [
  {
    node_identity: 'FiojKW7oY9WQmLCiYAsCA21tpowZHS6zcUoyYm319p6Z',
    delegated_on_iso_datetime: new Date(2021, 1, 1).toDateString(),
    accumulated_rewards: { amount: '0.05', denom: 'nym' },
    amount: { amount: '10', denom: 'nym' },
    profit_margin_percent: 0.1122323949234,
    owner: '',
    block_height: BigInt(100),
    stake_saturation: 0.5,
    avg_uptime_percent: 0.5,
    total_delegation: { amount: '0', denom: 'nym' },
    pledge_amount: { amount: '0', denom: 'nym' },
    pending_events: [],
    history: [],
    uses_vesting_contract_tokens: false,
  },
  {
    node_identity: 'DT8S942S8AQs2zKHS9SVo1GyHmuca3pfL2uLhLksJ3D8',
    accumulated_rewards: { amount: '0.1', denom: 'nym' },
    amount: { amount: '100', denom: 'nym' },
    delegated_on_iso_datetime: new Date(2021, 1, 2).toDateString(),
    profit_margin_percent: 0.89,
    owner: '',
    block_height: BigInt(4000),
    stake_saturation: 0.5,
    avg_uptime_percent: 0.1,
    total_delegation: { amount: '0', denom: 'nym' },
    pledge_amount: { amount: '0', denom: 'nym' },
    pending_events: [],
    history: [],
    uses_vesting_contract_tokens: true,
  },
];

export const WithData = () => <DelegationList items={items} explorerUrl={explorerUrl} />;

export const Empty = () => <DelegationList items={[]} explorerUrl={explorerUrl} />;

export const OneItem = () => <DelegationList items={[items[0]]} explorerUrl={explorerUrl} />;

export const Loading = () => <DelegationList isLoading explorerUrl={explorerUrl} />;

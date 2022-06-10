import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { DelegationActions } from './DelegationActions';

export default {
  title: 'Delegation/Components/Delegation List Item Actions',
  component: DelegationActions,
} as ComponentMeta<typeof DelegationActions>;

export const Default = () => <DelegationActions />;

export const RedeemingDisabled = () => <DelegationActions disableRedeemingRewards />;

export const PendingDelegation = () => <DelegationActions isPending={{ actionType: 'delegate', blockHeight: 1000 }} />;

export const PendingUndelegation = () => (
  <DelegationActions isPending={{ actionType: 'undelegate', blockHeight: 1000 }} />
);

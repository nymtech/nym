import * as React from 'react';
import { DelegationPage } from './index';
import { MockDelegationContextProvider } from '../../context/mocks/delegations';
import { MockRewardsContextProvider } from '../../context/mocks/rewards';

export default {
  title: 'Delegation/Flows/Mock',
};

export const Default = () => (
  <MockDelegationContextProvider>
    <MockRewardsContextProvider>
      <DelegationPage />
    </MockRewardsContextProvider>
  </MockDelegationContextProvider>
);

import React from 'react';
import { DelegationPage } from './index';
import { MockDelegationContextProvider } from '../../context/mocks/delegations';
import { MockRewardsContextProvider } from '../../context/mocks/rewards';
import { MockMainContextProvider } from '../../context/mocks/main';

export default {
  title: 'Delegation/Flows/Mock',
};

export const Default = () => (
  <MockMainContextProvider>
    <MockDelegationContextProvider>
      <MockRewardsContextProvider>
        <DelegationPage isStorybook />
      </MockRewardsContextProvider>
    </MockDelegationContextProvider>
  </MockMainContextProvider>
);

import * as React from 'react';
import { BondingPage } from './index';
import { MockBondingContextProvider } from '../../context/mocks/delegations';

export default {
  title: 'Bonding/Flows/Mock',
};

export const Default = () => (
  <MockBondingContextProvider>
    <BondingPage />
  </MockBondingContextProvider>
);

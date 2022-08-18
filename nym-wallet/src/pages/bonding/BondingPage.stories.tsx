import * as React from 'react';
import { BondingPage } from './Bonding';
import { MockBondingContextProvider } from '../../context/mocks/bonding';

export default {
  title: 'Bonding/Flows/Mock',
};

export const Default = () => (
  <MockBondingContextProvider>
    <BondingPage />
  </MockBondingContextProvider>
);

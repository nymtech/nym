import React from 'react';
import { FamiliesContextProvider } from 'src/context/FamiliesContextProvider';
import { BondingContextProvider } from 'src/context';
import { FamilySettingsPage } from './FamilySettingsPage';

export const FamilySettingsPageWithProvider = () => (
  <BondingContextProvider>
    <FamiliesContextProvider>
      <FamilySettingsPage />
    </FamiliesContextProvider>
  </BondingContextProvider>
);

import React from 'react';
import { FamiliesContextProvider } from 'src/context/FamiliesContextProvider';
import { BondingContextProvider } from 'src/context';
import { FamilyPage } from './FamilyPage';

/**
 * Route-level entry: wraps the page in the real (Tauri-backed) FamiliesContext
 * provider. Kept separate from `FamilyPage` so Storybook can render the page with
 * the mock provider without pulling in real Tauri code.
 *
 * `BondingContextProvider` sits above it so the families provider can derive the
 * account's controlled node ids from the bonded node (design D3).
 */
export const FamilyPageWithProvider = () => (
  <BondingContextProvider>
    <FamiliesContextProvider>
      <FamilyPage />
    </FamiliesContextProvider>
  </BondingContextProvider>
);

import React from 'react';
import { FamiliesContextProvider } from 'src/context/FamiliesContextProvider';
import { FamilyPage } from './FamilyPage';

/**
 * Route-level entry: wraps the page in the real (Tauri-backed) FamiliesContext
 * provider. Kept separate from `FamilyPage` so Storybook can render the page with
 * the mock provider without pulling in real Tauri code.
 */
export const FamilyPageWithProvider = () => (
  <FamiliesContextProvider>
    <FamilyPage />
  </FamiliesContextProvider>
);

import React from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import { DelegationContextProvider } from '../context/delegations';
import { RewardsContextProvider } from '../context/rewards';
import { ApplicationLayout } from '../layouts';
import { Terminal } from '../pages/terminal';
import { Send } from '../components/Send';
import { Receive } from '../components/Receive';
import { config } from '../config';
import {
  Balance,
  InternalDocs,
  DelegationPage,
  Admin,
  BondingPage,
  NodeSettingsPage,
  BuyPage,
  Settings,
} from '../pages';

export const AppRoutes = () => (
  <DelegationContextProvider>
    <RewardsContextProvider>
      <ApplicationLayout>
        <Terminal />
        <Send />
        <Receive />
        <Routes>
          <Route path="/" element={<Navigate to="/balance" />} />
          <Route path="/balance" element={<Balance />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/bonding" element={<BondingPage />} />
          <Route path="/bonding/node-settings" element={<NodeSettingsPage />} />
          <Route path="/delegation" element={<DelegationPage />} />
          {config.INTERNAL_DOCS_ENABLED && <Route path="/docs" element={<InternalDocs />} />}
          <Route path="/admin" element={<Admin />} />
          <Route path="/buy" element={<BuyPage />} />
        </Routes>
      </ApplicationLayout>
    </RewardsContextProvider>
  </DelegationContextProvider>
);

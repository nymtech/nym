import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { ApplicationLayout } from '../layouts';
import { Terminal } from '../pages/terminal';
import { Send } from '../components/Send';
import { Receive } from '../components/Receive';
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
  <ApplicationLayout>
    <Terminal />
    <Send />
    <Receive />
    <Routes>
      <Route path="/balance" element={<Balance />} />
      <Route path="/settings" element={<Settings />} />
      <Route path="/bonding" element={<BondingPage />} />
      <Route path="/bonding/node-settings" element={<NodeSettingsPage />} />
      <Route path="/delegation" element={<DelegationPage />} />
      <Route path="/docs" element={<InternalDocs />} />
      <Route path="/admin" element={<Admin />} />
      <Route path="/buy" element={<BuyPage />} />
    </Routes>
  </ApplicationLayout>
);

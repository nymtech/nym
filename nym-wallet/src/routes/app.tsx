import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { ApplicationLayout } from 'src/layouts';
import { Terminal } from 'src/pages/terminal';
import { Send } from 'src/components/Send';
import { Receive } from '../components/Receive';
import { Balance, InternalDocs, Unbond, DelegationPage, Admin, BondingPage, NodeSettingsPage } from '../pages';

export const AppRoutes = () => (
  <ApplicationLayout>
    <Terminal />
    <Send />
    <Receive />
    <Routes>
      <Route path="/balance" element={<Balance />} />
      <Route path="/bonding" element={<BondingPage />} />
      <Route path="/bonding/node-settings" element={<NodeSettingsPage />} />
      <Route path="/unbond" element={<Unbond />} />
      <Route path="/delegation" element={<DelegationPage />} />
      <Route path="/docs" element={<InternalDocs />} />
      <Route path="/admin" element={<Admin />} />
    </Routes>
  </ApplicationLayout>
);

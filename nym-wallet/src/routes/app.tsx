import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { ApplicationLayout } from 'src/layouts';
import { Terminal } from 'src/pages/terminal';
import { Send } from 'src/components/Send';
import { Bond, Balance, InternalDocs, Receive, Unbond, DelegationPage, Admin, Settings, BondingPage } from '../pages';

export const AppRoutes = () => (
  <ApplicationLayout>
    <Terminal />
    <Settings />
    <Send />
    <Routes>
      <Route path="/balance" element={<Balance />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/bond" element={<Bond />} />
      <Route path="/bonding" element={<BondingPage />} />
      <Route path="/unbond" element={<Unbond />} />
      <Route path="/delegation" element={<DelegationPage />} />
      <Route path="/docs" element={<InternalDocs />} />
      <Route path="/admin" element={<Admin />} />
    </Routes>
  </ApplicationLayout>
);

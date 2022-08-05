import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { ApplicationLayout } from 'src/layouts';
import { Terminal } from 'src/pages/terminal';
import { Send } from 'src/components/Send';
import { Receive } from '../components/Receive';
import { Bond, Balance, InternalDocs, Unbond, DelegationPage, Admin, Settings } from '../pages';

export const AppRoutes = () => (
  <ApplicationLayout>
    <Terminal />
    <Settings />
    <Send />
    <Receive />
    <Routes>
      <Route path="/balance" element={<Balance />} />
      <Route path="/bond" element={<Bond />} />
      <Route path="/unbond" element={<Unbond />} />
      <Route path="/delegation" element={<DelegationPage />} />
      <Route path="/docs" element={<InternalDocs />} />
      <Route path="/admin" element={<Admin />} />
    </Routes>
  </ApplicationLayout>
);

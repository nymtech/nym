import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { ApplicationLayout } from 'src/layouts';
import { Terminal } from 'src/pages/terminal';
import { Bond, Balance, InternalDocs, Receive, Send, Unbond, DelegationPage, Admin, Settings } from '../pages';

export const AppRoutes = () => (
  <ApplicationLayout>
    <Terminal />
    <Settings />
    <Routes>
      <Route path="/balance" element={<Balance />} />
      <Route path="/send" element={<Send />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/bond" element={<Bond />} />
      <Route path="/unbond" element={<Unbond />} />
      <Route path="/delegation" element={<DelegationPage />} />
      <Route path="/docs" element={<InternalDocs />} />
      <Route path="/admin" element={<Admin />} />
    </Routes>
  </ApplicationLayout>
);

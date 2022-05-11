import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { ApplicationLayout } from 'src/layouts';
import { Terminal } from 'src/pages/terminal';
import { Bond, Balance, Delegate, InternalDocs, Receive, Send, Unbond, Undelegate, DelegationPage } from '../pages';

export const AppRoutes = () => (
  <ApplicationLayout>
    <Terminal />
    <Routes>
      <Route path="/balance" element={<Balance />} />
      <Route path="/send" element={<Send />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/bond" element={<Bond />} />
      <Route path="/unbond" element={<Unbond />} />
      <Route path="/delegate" element={<Delegate />} />
      <Route path="/undelegate" element={<Undelegate />} />
      <Route path="/delegation" element={<DelegationPage />} />
      <Route path="/docs" element={<InternalDocs />} />
    </Routes>
  </ApplicationLayout>
);

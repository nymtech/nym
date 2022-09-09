import { Route, Routes } from 'react-router-dom';
import { Send } from 'src/components/Send';
import { ApplicationLayout } from 'src/layouts';
import { Terminal } from 'src/pages/terminal';
import { Receive } from '../components/Receive';
import {
  Admin,
  Balance,
  BondingPage,
  DelegationPage,
  InternalDocs,
  NodeSettingsPage,
  TestNode,
  Unbond,
} from '../pages';

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

import React, { useEffect } from 'react';
import { Route, Routes, useNavigate } from 'react-router-dom';
import { useAppContext } from 'src/context';
import { Delegation, BalancePage, Receive, Send, Settings } from 'src/pages';
import { AccountRoutes } from './accounts/accounts';

export const UserRoutes = () => {
  const { client } = useAppContext();
  const navigate = useNavigate();

  useEffect(() => {
    if (!client) navigate('/login');
  }, [client]);

  return (
    <Routes>
      <Route path="/accounts/*" element={<AccountRoutes />} />
      <Route path="/balance" element={<BalancePage />} />
      <Route path="/delegation" element={<Delegation />} />
      <Route path="/receive" element={<Receive />} />
      <Route path="/send" element={<Send />} />
      <Route path="/settings" element={<Settings />} />
    </Routes>
  );
};

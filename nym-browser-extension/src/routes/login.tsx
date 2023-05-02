import React, { useEffect } from 'react';
import { Route, Routes, useNavigate } from 'react-router-dom';
import { useAppContext } from 'src/context';
import { ForgotPassword, Login } from 'src/pages/auth';

export const LoginRoutes = () => {
  const { client } = useAppContext();
  const navigate = useNavigate();

  useEffect(() => {
    if (client) navigate('/user/balance');
    else navigate('/login');
  }, [client]);

  return (
    <Routes>
      <Route index element={<Login />} />
      <Route path="/forgot-password" element={<ForgotPassword />} />
    </Routes>
  );
};

import React, { useEffect, useState } from 'react';
import { BrowserRouter, MemoryRouter, Route, Routes } from 'react-router-dom';
import { Home } from 'src/pages';
import { ExtensionStorage } from '@nymproject/extension-storage';
import { RegisterRoutes } from './register';
import { UserRoutes } from './user';
import { LoginRoutes } from './login';

const Router = process.env.NODE_ENV === 'development' ? BrowserRouter : MemoryRouter;

export const AppRoutes = () => {
  const [userHasAccount, setUserHasAccount] = useState(null);

  useEffect(() => {
    const checkUserHasAccount = async () => {
      const hasAccount = await ExtensionStorage.exists();
      setUserHasAccount(hasAccount);
    };

    checkUserHasAccount();
  }, []);

  if (userHasAccount === null) return null;

  return (
    <Router>
      <Routes>
        <Route path="/" element={userHasAccount ? <LoginRoutes /> : <Home />} />
        <Route path="/login/*" element={<LoginRoutes />} />
        <Route path="/register/*" element={<RegisterRoutes />} />
        <Route path="/user/*" element={<UserRoutes />} />
      </Routes>
    </Router>
  );
};

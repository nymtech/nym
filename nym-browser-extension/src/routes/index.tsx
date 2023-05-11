import React from 'react';
import { BrowserRouter, MemoryRouter, Route, Routes } from 'react-router-dom';
import { Home } from 'src/pages/';
import { RegisterRoutes } from './register';
import { UserRoutes } from './user';
import { LoginRoutes } from './login';

const Router = process.env.NODE_ENV === 'development' ? BrowserRouter : MemoryRouter;

export const AppRoutes = () => {
  // hack to work on redirect until password capability is set up
  const userHasAccount = localStorage.getItem('nym-browser-extension');

  console.log(userHasAccount);

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

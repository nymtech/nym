import React from 'react';
import { BrowserRouter, MemoryRouter, Route, Routes } from 'react-router-dom';
// import { Home } from 'src/pages/';
import { RegisterRoutes } from './register';
import { UserRoutes } from './user';
import { LoginRoutes } from './login';

const Router = process.env.NODE_ENV === 'development' ? BrowserRouter : MemoryRouter;

export const AppRoutes = () => (
  <Router>
    <Routes>
      <Route path="/" element={<LoginRoutes />} />
      <Route path="/login/*" element={<LoginRoutes />} />
      <Route path="/register/*" element={<RegisterRoutes />} />
      <Route path="/user/*" element={<UserRoutes />} />
    </Routes>
  </Router>
);

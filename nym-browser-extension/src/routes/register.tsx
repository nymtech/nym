import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { Register } from 'src/pages/register';
import { SetupComplete } from 'src/pages/register/Complete';
import { CreatePassword } from 'src/pages/register/CreatePassword';

export const RegisterRoutes = () => (
  <Routes>
    <Route index element={<Register />} />
    <Route path="/create-password" element={<CreatePassword />} />
    <Route path="/complete" element={<SetupComplete />} />
  </Routes>
);

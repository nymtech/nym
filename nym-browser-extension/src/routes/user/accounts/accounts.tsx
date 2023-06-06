import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { RegisterContextProvider } from 'src/context/register';
import { Accounts, AddAccount, ConfirmPassword, ImportAccount, NameAccount, SetupComplete } from 'src/pages';

export const AccountRoutes = () => (
  <RegisterContextProvider>
    <Routes>
      <Route path="/" element={<Accounts />} />
      <Route path="/add-account" element={<AddAccount />} />
      <Route path="/import-account" element={<ImportAccount />} />
      <Route path="/name-account" element={<NameAccount />} />
      <Route path="/confirm-password" element={<ConfirmPassword />} />
      <Route path="/complete" element={<SetupComplete />} />
    </Routes>
  </RegisterContextProvider>
);

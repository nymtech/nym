import React from 'react';
import { Route, Routes } from 'react-router-dom';
import { RegisterContextProvider } from 'src/context/register';
import { Accounts } from 'src/pages';
import { ConfirmPassword, ImportAccount, NameAccount } from 'src/pages/accounts';
import { SetupComplete } from 'src/pages/accounts/Complete';

export const AccountRoutes = () => {
  return (
    <RegisterContextProvider>
      <Routes>
        <Route path="/" element={<Accounts />} />
        <Route path="/add-account" element={<div />} />
        <Route path="/import-account" element={<ImportAccount />} />
        <Route path="/import-account/name-account" element={<NameAccount />} />
        <Route path="/import-account/confirm-password" element={<ConfirmPassword />} />
        <Route path="/import-account/complete" element={<SetupComplete />} />
      </Routes>
    </RegisterContextProvider>
  );
};

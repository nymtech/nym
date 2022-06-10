import React from 'react';
import { Routes, Route } from 'react-router-dom';
import { AuthProvider } from 'src/context';
import { AuthLayout } from 'src/layouts/AuthLayout';
import {
  CreateMnemonic,
  CreatePassword,
  ExistingAccount,
  SignInMnemonic,
  SignInPassword,
  VerifyMnemonic,
  WelcomeContent,
  ConnectPassword,
} from 'src/pages/auth/pages';
import { ConfirmMnemonic } from 'src/pages/auth/pages/confirm-mnemonic';
import { AuthTheme } from 'src/theme';

export const AuthRoutes = () => (
  <AuthProvider>
    <AuthTheme>
      <AuthLayout>
        <Routes>
          <Route path="/" element={<WelcomeContent />} />
          <Route path="/existing-account" element={<ExistingAccount />} />
          <Route path="/create-mnemonic" element={<CreateMnemonic />} />
          <Route path="/verify-mnemonic" element={<VerifyMnemonic />} />
          <Route path="/create-password" element={<CreatePassword />} />
          <Route path="/sign-in-mnemonic" element={<SignInMnemonic />} />
          <Route path="/sign-in-password" element={<SignInPassword />} />
          <Route path="/confirm-mnemonic" element={<ConfirmMnemonic />} />
          <Route path="/connect-password" element={<ConnectPassword />} />
        </Routes>
      </AuthLayout>
    </AuthTheme>
  </AuthProvider>
);

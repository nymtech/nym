import React from 'react';
import { useContext } from 'react';
import { Routes, Route } from 'react-router-dom';
import { AppContext, AuthProvider } from '@src/context';
import { AuthLayout } from '@src/layouts/AuthLayout';
import {
  CreateMnemonic,
  CreatePassword,
  ExistingAccount,
  SignInMnemonic,
  SignInPassword,
  VerifyMnemonic,
  WelcomeContent,
  ConnectPassword,
} from '@src/pages/auth/pages';
import { ConfirmMnemonic } from '@src/pages/auth/pages/confirm-mnemonic';
import { ForgotPassword } from '@src/pages/auth/pages/forgot-password';
import { AuthTheme } from '@src/theme';
import { createMainWindow } from '../requests/app';

export const AuthRoutes = () => {
  const { clientDetails, keepState } = useContext(AppContext);

  const switchWindows = async () => {
    if (clientDetails) {
      // stash some of the state in the Rust process, because this React app is about the unload
      // when the window is closed
      try {
        await keepState();
        await createMainWindow();
      } catch (e) {
        console.error(e);
      }

      // close the window and open the main app in a new window
    }
  };

  React.useEffect(() => {
    switchWindows();
  }, [clientDetails]);

  return (
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
            <Route path="/forgot-password" element={<ForgotPassword />} />
          </Routes>
        </AuthLayout>
      </AuthTheme>
    </AuthProvider>
  );
};

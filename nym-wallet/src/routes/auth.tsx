import React from 'react';
import { Switch, Route } from 'react-router-dom';
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
        <Switch>
          <Route path="/" exact>
            <WelcomeContent />
          </Route>
          <Route path="/existing-account">
            <ExistingAccount />
          </Route>
          <Route path="/create-mnemonic">
            <CreateMnemonic />
          </Route>
          <Route path="/verify-mnemonic">
            <VerifyMnemonic />
          </Route>
          <Route path="/create-password">
            <CreatePassword />
          </Route>
          <Route path="/sign-in-mnemonic">
            <SignInMnemonic />
          </Route>
          <Route path="/sign-in-password">
            <SignInPassword />
          </Route>
          <Route path="/confirm-mnemonic">
            <ConfirmMnemonic />
          </Route>
          <Route path="/connect-password">
            <ConnectPassword />
          </Route>
        </Switch>
      </AuthLayout>
    </AuthTheme>
  </AuthProvider>
);

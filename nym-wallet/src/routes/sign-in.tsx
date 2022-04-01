import React from 'react';
import { Switch, Route } from 'react-router-dom';
import { PageLayout } from 'src/pages/sign-in/components';
import {
  CreateMnemonic,
  CreatePassword,
  ExistingAccount,
  SignInMnemonic,
  SignInPassword,
  VerifyMnemonic,
  WelcomeContent,
  ConnectPassword,
} from 'src/pages/sign-in/pages';
import { ConfirmMnemonic } from 'src/pages/sign-in/pages/confirm-mnemonic';

export const SignInRoutes = () => (
  <PageLayout>
    <Switch>
      <Route path="/welcome">
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
  </PageLayout>
);

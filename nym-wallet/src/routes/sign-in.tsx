import React from 'react';
import { Switch, Route } from 'react-router-dom';
import { CreateMnemonic, VerifyMnemonic, WelcomeContent } from 'src/pages/welcome/pages';

export const SignInRoutes = () => (
  <Switch>
    <Route path="/welcome">
      <WelcomeContent />
    </Route>
    <Route path="/existing-account">
      <h1>Existing account</h1>
    </Route>
    <Route path="/create-mnemonic">
      <CreateMnemonic />
    </Route>
    <Route path="/verify-mnemonic">
      <VerifyMnemonic />
    </Route>
  </Switch>
);

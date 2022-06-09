import React from 'react';
import { Route, Switch } from 'react-router-dom';

export const Routes: React.FC = () => (
  <Switch>
    <Route path="/">
      <div>Root</div>
    </Route>
  </Switch>
);

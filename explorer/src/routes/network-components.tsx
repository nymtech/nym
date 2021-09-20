import * as React from 'react';
import { Switch, Route } from 'react-router-dom';

export const NetworkComponentsRoutes: React.FC = () => (
  <Switch>
    <Route path="/network-components/mixnodes">
      <h1>Mixnodes</h1>
    </Route>
    <Route path="/network-components/gateways">
      <h1>Gateways</h1>
    </Route>
  </Switch>
);

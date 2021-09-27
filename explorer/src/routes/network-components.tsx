import * as React from 'react';
import { Switch, Route } from 'react-router-dom';
import { PageMixnodeDetail } from 'src/pages/MixnodeDetail';
import { PageMixnodes } from '../pages/Mixnodes';

export const NetworkComponentsRoutes: React.FC = () => (
  <Switch>
    <Route exact path="/network-components/mixnodes">
      <PageMixnodes />
    </Route>
    <Route path="/network-components/mixnodes/:id">
      <PageMixnodeDetail />
    </Route>
    <Route path="/network-components/gateways">
      <h1>Gateways</h1>
    </Route>
  </Switch>
);

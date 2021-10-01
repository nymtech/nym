import * as React from 'react';
import { Switch, Route } from 'react-router-dom';
import { PageGateways } from 'src/pages/Gateways';
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
      <PageGateways />
    </Route>
    <Route path="/network-components/gateways/:id">
      <h1> Specific Gateways ID</h1>
    </Route>
  </Switch>
);

import * as React from 'react';
import { Switch, Route } from 'react-router-dom';
import { PageMixnodeInfo } from 'src/pages/MixnodeInfo';
import { PageMixnodes } from '../pages/Mixnodes';

export const NetworkComponentsRoutes: React.FC = () => (
  <Switch>
    <Route exact path="/network-components/mixnodes">
      <PageMixnodes />
    </Route>
    <Route path="/network-components/mixnodes/:id">
      <PageMixnodeInfo />
    </Route>
    <Route path="/network-components/gateways">
      <h1>Gateways</h1>
    </Route>
  </Switch>
);

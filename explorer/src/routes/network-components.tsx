import * as React from 'react';
import { Switch, Route, RouteComponentProps } from 'react-router-dom';
import { BIG_DIPPER } from '../api/constants';
import { PageGateways } from '../pages/Gateways';
import { PageMixnodeDetail } from '../pages/MixnodeDetail';
import { PageMixnodes } from '../pages/Mixnodes';

export const NetworkComponentsRoutes: React.FC = () => (
  <Switch>
    <Route exact path="/network-components/mixnodes/:status">
      <PageMixnodes />
    </Route>
    <Route exact path="/network-components/mixnodes">
      <PageMixnodes />
    </Route>
    <Route path="/network-components/mixnode/:id">
      <PageMixnodeDetail />
    </Route>
    <Route path="/network-components/gateways">
      <PageGateways />
    </Route>
    <Route
      path="/network-components/validators"
      component={(props: RouteComponentProps) => {
        window.open(`${BIG_DIPPER}/validators`);
        props.history.goBack();
        return null;
      }}
    />
    <Route path="/network-components/gateways/:id">
      <h1> Specific Gateways ID</h1>
    </Route>
  </Switch>
);

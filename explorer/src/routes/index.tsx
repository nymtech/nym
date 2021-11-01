import * as React from 'react';
import { Route, Switch, Redirect } from 'react-router-dom';
import { PageOverview } from 'src/pages/Overview';
import { PageMixnodesMap } from 'src/pages/MixnodesMap';
import { NetworkComponentsRoutes } from './network-components';

export const Routes: React.FC = () => (
  <Switch>
    <Route exact path="/">
      <Redirect to="/overview" />
    </Route>
    <Route exact path="/overview">
      <PageOverview />
    </Route>
    <Route path="/network-components">
      <NetworkComponentsRoutes />
    </Route>
    <Route path="/nodemap">
      <PageMixnodesMap />
    </Route>
  </Switch>
);

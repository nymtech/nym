import * as React from 'react';
import { BrowserRouter as Router, Route, Switch } from 'react-router-dom';
import { Nav } from 'src/components/Nav';
import { PageOverview } from 'src/pages/Overview';
import { PageNetworkComponents } from 'src/pages/NetworkComponents';
import { PageMixnodesMap } from 'src/pages/MixnodesMap';

export const Routes: React.FC = () => (
  <Router>
    <Nav>
      <Switch>
        <Route exact path="/">
          <PageOverview />
        </Route>
        <Route exact path="/overview">
          <PageOverview />
        </Route>
        <Route exact path="/network-components">
          <PageNetworkComponents />
        </Route>
        <Route path="/nodemap">
          <PageMixnodesMap />
        </Route>
      </Switch>
    </Nav>
  </Router>
);

import * as React from 'react';
import {
  BrowserRouter as Router,
  Route,
  Switch,
  // NavLink,
} from 'react-router-dom';
import Nav from 'src/components/Nav';

import {
  PageOverview,
  PageMixnodes,
  PageMixnodesMap,
  PageGateways,
} from 'src/pages';

export const Routes: React.FC = () => (
  <Router>
    <div style={{ height: 72, width: '100vw', backgroundColor: '#070B15' }}>
      top menu
    </div>
    <Nav />
    <Switch>
      <Route exact path="/">
        <PageOverview />
      </Route>
      <Route exact path="/overview">
        <PageOverview />
      </Route>
      <Route exact path="/network-components">
        <PageMixnodes />
      </Route>
      <Route path="/nodemap">
        <PageMixnodesMap />
      </Route>
    </Switch>
  </Router>
);

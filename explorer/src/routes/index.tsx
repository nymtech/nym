import * as React from 'react';
import {
  BrowserRouter as Router,
  Route,
  Switch,
  NavLink,
} from 'react-router-dom';
import {
  PageOverview,
  PageMixnodes,
  PageMixnodesMap,
  PageGateways,
} from 'src/pages';

export const Routes: React.FC = () => (
  <Router>
    <div style={{ display: 'flex', flexDirection: 'column', margin: 30 }}>
      <NavLink to="/">Overview</NavLink>
      <NavLink to="/mixnodes">Mix Nodes</NavLink>
      <NavLink to="/mixnodes/map">Mix Nodes MAP</NavLink>
      <NavLink to="/gateways">Gateways</NavLink>
    </div>
    <Switch>
      <Route exact path="/">
        <PageOverview />
      </Route>
      <Route exact path="/mixnodes">
        <PageMixnodes />
      </Route>
      <Route path="/mixnodes/map">
        <PageMixnodesMap />
      </Route>
      <Route path="/gateways">
        <PageGateways />
      </Route>
    </Switch>
  </Router>
);

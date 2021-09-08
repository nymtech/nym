import * as React from 'react';
import {
  BrowserRouter as Router,
  Route,
  Switch,
  NavLink,
} from 'react-router-dom';

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
        <h1>OVERVIEW</h1>
      </Route>
      <Route exact path="/mixnodes">
        <h1>MIX-NODES</h1>
      </Route>
      <Route path="/mixnodes/map">
        <h1>THE MIX-NODES MAP</h1>
      </Route>
      <Route path="/gateways">
        <h1>GATEWAYS</h1>
      </Route>
    </Switch>
  </Router>
);

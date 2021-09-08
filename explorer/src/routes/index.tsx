import * as React from 'react';
import {
  BrowserRouter as Router,
  Route,
  Switch,
  Link,
  NavLink,
} from 'react-router-dom';

export const Routes: React.FC = () => (
  <Router>
    <Switch>
      <Route exact path="/">
        <h1>OVERVIEW</h1>
      </Route>
      <Route path="/mixnodes">
        <h1>MIX NODES</h1>
      </Route>
      <Route path="/gateways">
        <h1>GATEWAYS</h1>
      </Route>
    </Switch>
  </Router>
);

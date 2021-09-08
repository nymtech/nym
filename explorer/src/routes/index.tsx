import * as React from 'react';
import { BrowserRouter as Router, Route, Switch } from 'react-router-dom';

export const Routes: React.FC = () => (
  <Router>
    <Switch>
      <Route path="/">
        <div>Home</div>
      </Route>
      <Route path="/mixnodes">
        <div>Mixnodes</div>
      </Route>
      <Route path="/gateways">
        <div>Gateways</div>
      </Route>
    </Switch>
  </Router>
);

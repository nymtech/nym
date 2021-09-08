import * as React from 'react';
import { BrowserRouter as Router, Route, Switch } from 'react-router-dom';

export const Routes: React.FC = () => (
  <Router>
    <Switch>
      <Route path="/">
        <div>Home</div>
      </Route>
    </Switch>
  </Router>
);

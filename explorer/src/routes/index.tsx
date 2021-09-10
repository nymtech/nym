import * as React from 'react';
import { BrowserRouter as Router, Route, Switch } from 'react-router-dom';
import { Nav } from 'src/components/Nav';
import { TopMenu } from 'src/components/TopMenu';

import { PageOverview, PageMixnodes, PageMixnodesMap } from 'src/pages';

export const Routes: React.FC = (props) => {
  console.log('Routes props ', props);
  return (
    <Router>
      <TopMenu />
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
};

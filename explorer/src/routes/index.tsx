import * as React from 'react';
import { Routes as ReactRouterRoutes, Route } from 'react-router-dom';
import { PageOverview } from '../pages/Overview';
import { PageMixnodesMap } from '../pages/MixnodesMap';
import { Page404 } from '../pages/404';
import { NetworkComponentsRoutes } from './network-components';

export const Routes: React.FC = () => (
  <ReactRouterRoutes>
    <Route path="/" element={<PageOverview />} />
    <Route path="/network-components/*" element={<NetworkComponentsRoutes />} />
    <Route path="/nodemap" element={<PageMixnodesMap />} />
    <Route path="*" element={Page404} />
  </ReactRouterRoutes>
);

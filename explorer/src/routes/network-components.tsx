import * as React from 'react';
import { Routes as ReactRouterRoutes, Route, useNavigate } from 'react-router-dom';
import { BIG_DIPPER } from '../api/constants';
import { PageGateways } from '../pages/Gateways';
import { PageMixnodeDetail } from '../pages/MixnodeDetail';
import { PageMixnodes } from '../pages/Mixnodes';

const ValidatorRoute: React.FC = () => {
  const navigate = useNavigate();
  window.open(`${BIG_DIPPER}/validators`);
  navigate(-1);
  return null;
};

export const NetworkComponentsRoutes: React.FC = () => (
  <ReactRouterRoutes>
    <Route path="mixnodes/:status" element={<PageMixnodes />} />
    <Route path="mixnodes" element={<PageMixnodes />} />
    <Route path="mixnode/:id" element={<PageMixnodeDetail />} />
    <Route path="gateways" element={<PageGateways />} />
    <Route path="validators" element={<ValidatorRoute />} />
    <Route path="gateways/:id" element={<h1> Specific Gateways ID</h1>} />
  </ReactRouterRoutes>
);

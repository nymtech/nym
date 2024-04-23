import * as React from 'react';
import { Routes as ReactRouterRoutes, Route, useNavigate } from 'react-router-dom';
import { BIG_DIPPER } from '../api/constants';
import { PageGateways } from '../pages/Gateways';
import { PageGatewayDetail } from '../pages/GatewayDetail';
import { PageMixnodeDetail } from '../pages/MixnodeDetail';
import { PageMixnodes } from '../pages/Mixnodes';

const ValidatorRoute: FCWithChildren = () => {
  const navigate = useNavigate();
  window.open(`${BIG_DIPPER}/validators`);
  navigate(-1);
  return null;
};

export const NetworkComponentsRoutes: FCWithChildren = () => (
  <ReactRouterRoutes>
    <Route path="mixnodes/:status" element={<PageMixnodes />} />
    <Route path="mixnodes" element={<PageMixnodes />} />
    <Route path="mixnode/:id" element={<PageMixnodeDetail />} />
    <Route path="gateways" element={<PageGateways />} />
    <Route path="gateway/:id" element={<PageGatewayDetail />} />
    <Route path="validators" element={<ValidatorRoute />} />
  </ReactRouterRoutes>
);

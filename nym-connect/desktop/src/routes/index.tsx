import React from 'react';
import { Routes, Route } from 'react-router-dom';
import * as Sentry from '@sentry/react';
import { ConnectionPage } from 'src/pages/connection';
import { Menu } from 'src/pages/menu';
import { CompatibleApps } from 'src/pages/menu/Apps';
import { HelpGuide } from 'src/pages/menu/Guide';
import { SettingsMenu } from 'src/pages/menu/settings';
import { GatewaySettings } from 'src/pages/menu/settings/GatewaySettings';
import { ServiceProviderSettings } from 'src/pages/menu/settings/ServiceProviderSettings';
import { MonitoringSettings } from '../pages/menu/settings/MonitoringSettings';
import { useClientContext } from '../context/main';

const SentryRoutes = Sentry.withSentryReactRouterV6Routing(Routes);

export const AppRoutes = () => {
  const { monitoringEnabled } = useClientContext();

  const RoutesContainer = monitoringEnabled ? SentryRoutes : Routes;

  return (
    <RoutesContainer>
      <Route index path="/" element={<ConnectionPage />} />
      <Route path="menu">
        <Route index element={<Menu />} />
        <Route path="apps" element={<CompatibleApps />} />
        <Route path="guide" element={<HelpGuide />} />
        <Route path="monitoring" element={<MonitoringSettings />} />
        <Route path="settings">
          <Route index element={<SettingsMenu />} />
          <Route path="gateway" element={<GatewaySettings />} />
          <Route path="service-provider" element={<ServiceProviderSettings />} />
        </Route>
      </Route>
    </RoutesContainer>
  );
};

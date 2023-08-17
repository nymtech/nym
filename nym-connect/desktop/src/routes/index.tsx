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
import { ErrorReporting } from '../pages/menu/reporting/ErrorReporting';
import { PrivacyLevelSettings } from '../pages/menu/PrivacyLevelSettings';
import { useClientContext } from '../context/main';
import { ReportingMenu } from '../pages/menu/reporting';
import { UserFeedback } from '../pages/menu/reporting/UserFeedback';

const SentryRoutes = Sentry.withSentryReactRouterV6Routing(Routes);

export const AppRoutes = () => {
  const { userData } = useClientContext();

  const RoutesContainer = userData?.monitoring ? SentryRoutes : Routes;

  return (
    <RoutesContainer>
      <Route index path="/" element={<ConnectionPage />} />
      <Route path="menu">
        <Route index element={<Menu />} />
        <Route path="apps" element={<CompatibleApps />} />
        <Route path="guide" element={<HelpGuide />} />
        <Route path="privacy-level" element={<PrivacyLevelSettings />} />
        <Route path="settings">
          <Route index element={<SettingsMenu />} />
          <Route path="gateway" element={<GatewaySettings />} />
          <Route path="service-provider" element={<ServiceProviderSettings />} />
        </Route>
        <Route path="reporting">
          <Route index element={<ReportingMenu />} />
          <Route path="error-reporting" element={<ErrorReporting />} />
          <Route path="user-feedback" element={<UserFeedback />} />
        </Route>
      </Route>
    </RoutesContainer>
  );
};

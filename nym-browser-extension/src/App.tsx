import React from 'react';
import { NymBrowserExtThemeWithMode } from './theme/NymBrowserExtensionTheme';
import { AppRoutes } from './routes';
import { AppLayout } from './layouts/AppLayout';

export const App = () => (
  <NymBrowserExtThemeWithMode mode="light">
    <AppLayout>
      <AppRoutes />
    </AppLayout>
  </NymBrowserExtThemeWithMode>
);

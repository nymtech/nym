import React from 'react';
import { NymBrowserExtThemeWithMode } from './theme/NymBrowserExtensionTheme';
import { AppRoutes } from './routes';
import { AppLayout } from './layouts/AppLayout';
import { AppProvider } from './context';

export const App = () => (
  <NymBrowserExtThemeWithMode mode="light">
    <AppProvider>
      <AppLayout>
        <AppRoutes />
      </AppLayout>
    </AppProvider>
  </NymBrowserExtThemeWithMode>
);

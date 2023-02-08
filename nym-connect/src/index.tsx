import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymMixnetTheme } from './theme';
import { AppWindowFrame } from './components/AppWindowFrame';
import { TestAndEarnContextProvider } from './components/Growth/context/TestAndEarnContext';
import { BrowserRouter as Router } from 'react-router-dom';
import { AppRoutes } from './routes';
import { GlobalStyles } from '@mui/material';

const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <Router>
        <ClientContextProvider>
          <TestAndEarnContextProvider>
            <GlobalStyles styles={{ html: { borderRadius: 10 } }} />
            <NymMixnetTheme mode="dark">
              <AppWindowFrame>
                <AppRoutes />
              </AppWindowFrame>
            </NymMixnetTheme>
          </TestAndEarnContextProvider>
        </ClientContextProvider>
      </Router>
    </ErrorBoundary>,
  );
}

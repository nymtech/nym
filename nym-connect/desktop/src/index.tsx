import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { BrowserRouter as Router } from 'react-router-dom';
import { GlobalStyles } from '@mui/material';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymMixnetTheme } from './theme';
import { AppWindowFrame } from './components/AppWindowFrame';
import { AppRoutes } from './routes';

const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <Router>
        <ClientContextProvider>
          <GlobalStyles styles={{ html: { borderRadius: 10 } }} />
          <NymMixnetTheme mode="dark">
            <AppWindowFrame>
              <AppRoutes />
            </AppWindowFrame>
          </NymMixnetTheme>
        </ClientContextProvider>
      </Router>
    </ErrorBoundary>,
  );
}

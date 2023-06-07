import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import {
  BrowserRouter as Router,
  createRoutesFromChildren,
  matchRoutes,
  useNavigationType,
  useLocation,
} from 'react-router-dom';
import * as Sentry from '@sentry/react';
import { CaptureConsole } from '@sentry/integrations';
import { getVersion } from '@tauri-apps/api/app';
import { GlobalStyles } from '@mui/material';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymMixnetTheme } from './theme';
import { AppWindowFrame } from './components/AppWindowFrame';
import { TestAndEarnContextProvider } from './components/Growth/context/TestAndEarnContext';
import { AppRoutes } from './routes';

const elem = document.getElementById('root');

Sentry.init({
  dsn: 'https://625e2658da4945a7a253f3ee04413a31@o967446.ingest.sentry.io/4505306292289536',
  integrations: [
    new Sentry.BrowserTracing({
      // Set `tracePropagationTargets` to control for which URLs distributed tracing should be enabled
      tracePropagationTargets: ['localhost'],
      routingInstrumentation: Sentry.reactRouterV6Instrumentation(
        React.useEffect,
        useLocation,
        useNavigationType,
        createRoutesFromChildren,
        matchRoutes,
      ),
    }),
    new Sentry.Replay(),
    // captures all Console API calls and redirects them to Sentry
    new CaptureConsole(),
  ],

  // TODO adjust this in the future, 100% is not recommended for production
  tracesSampleRate: 1.0,

  // Capture Replay for 10% of all sessions,
  // plus for 100% of sessions with an error
  replaysSessionSampleRate: 0.1,
  replaysOnErrorSampleRate: 1.0,

  environment: process.env.NODE_ENV,
});

(async () => {
  getVersion().then((version) => {
    Sentry.setTag('app_version', version);
  });
})();

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

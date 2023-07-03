import React from 'react';
import { createRoutesFromChildren, matchRoutes, useLocation, useNavigationType } from 'react-router-dom';
import * as Sentry from '@sentry/react';
import { CaptureConsole } from '@sentry/integrations';
import { getVersion } from '@tauri-apps/api/app';

const SENTRY_DSN = 'https://625e2658da4945a7a253f3ee04413a31@o967446.ingest.sentry.io/4505306292289536';

async function initSentry() {
  console.log('⚠ performance monitoring and error reporting enabled');
  console.log('initializing sentry');

  Sentry.init({
    dsn: SENTRY_DSN,
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
      // captures Console API calls
      new CaptureConsole({ levels: ['error', 'warn'] }),
    ],

    // TODO adjust this in the future, 100% is not recommended for production
    tracesSampleRate: 1.0,

    // Capture Replay for 10% of all sessions,
    // plus for 100% of sessions with an error
    replaysSessionSampleRate: 0.1,
    replaysOnErrorSampleRate: 1.0,

    environment: process.env.NODE_ENV,
  });

  getVersion().then((version) => {
    Sentry.setTag('app_version', version);
  });

  Sentry.setUser({ id: 'nym', ip_address: undefined });
}

export default initSentry;

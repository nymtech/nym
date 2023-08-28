import React from 'react';
import { createRoutesFromChildren, matchRoutes, useLocation, useNavigationType } from 'react-router-dom';
import { invoke } from '@tauri-apps/api';
import * as Sentry from '@sentry/react';
import { CaptureConsole } from '@sentry/integrations';
import { getVersion } from '@tauri-apps/api/app';

const SENTRY_DSN = 'SENTRY_DSN_JS';

async function initSentry() {
  console.log('⚠ performance monitoring and error reporting enabled');
  console.log('initializing sentry');

  const dsn = await invoke<string | undefined>('get_env', { variable: SENTRY_DSN });

  if (!dsn) {
    console.warn(`unable to initialize sentry, ${SENTRY_DSN} env var not set`);
    return;
  }

  Sentry.init({
    dsn,
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

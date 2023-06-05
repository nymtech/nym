import React from 'react';
import * as Sentry from '@sentry/react';
import { createRoot } from 'react-dom/client';
import { getVersion } from '@tauri-apps/api/app';
import { AppRoutes } from './routes/app';
import { AppCommon } from './common';

if (process.env.NODE_ENV === 'production') {
  Sentry.init({
    dsn: 'https://8002d583e6eb4660a33f741122cfacd2@o967446.ingest.sentry.io/4505306294386688',
    integrations: [
      new Sentry.BrowserTracing({
        // Set `tracePropagationTargets` to control for which URLs distributed tracing should be enabled
        tracePropagationTargets: ['localhost'],
      }),
      new Sentry.Replay(),
    ],

    // TODO adjust this in the future, 100% is not recommended for production
    tracesSampleRate: 1.0,

    // Capture Replay for 10% of all sessions,
    // plus for 100% of sessions with an error
    replaysSessionSampleRate: 0.1,
    replaysOnErrorSampleRate: 1.0,
  });

  Sentry.setTag('app_env', 'production');

  (async () => {
    getVersion().then((version) => {
      Sentry.setTag('app_version', version);
    });
  })();
}

const MainApp = () => (
  <AppCommon>
    <AppRoutes />
  </AppCommon>
);
const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(<MainApp />);
}

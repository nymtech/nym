import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymMixnetTheme } from './theme';
import { App } from './App';
import { AppWindowFrame } from './components/AppWindowFrame';

const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <ClientContextProvider>
        <NymMixnetTheme mode="dark">
          <AppWindowFrame>
            <App />
          </AppWindowFrame>
        </NymMixnetTheme>
      </ClientContextProvider>
    </ErrorBoundary>,
  );
}

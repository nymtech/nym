import React from 'react';
import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from 'react-error-boundary';
import { ClientContextProvider } from './context/main';
import { ErrorFallback } from './components/Error';
import { NymShipyardTheme } from './theme';
import { TestAndEarnPopup } from './components/Growth/TestAndEarnPopup';
import { TestAndEarnContextProvider } from './components/Growth/context/TestAndEarnContext';

const elem = document.getElementById('root-growth');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <ClientContextProvider>
        <TestAndEarnContextProvider>
          <NymShipyardTheme mode="dark">
            <TestAndEarnPopup />
          </NymShipyardTheme>
        </TestAndEarnContextProvider>
      </ClientContextProvider>
    </ErrorBoundary>,
  );
}

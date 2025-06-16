import * as React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter as Router } from 'react-router-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { MainContextProvider } from './context/main';
import { NetworkExplorerThemeProvider } from './theme';
import { ErrorBoundaryContent } from './errors/ErrorBoundaryContent';
import CosmosKitProvider from './context/cosmos-kit';
import '@interchain-ui/react/styles';
import { App } from './App';
import { WalletProvider } from './context/wallet';
import { EnvironmentProvider } from './providers/EnvironmentProvider';
import './styles.css';

const elem = document.getElementById('app');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorBoundaryContent}>
      <EnvironmentProvider>
        <MainContextProvider>
          <CosmosKitProvider>
            <WalletProvider>
              <NetworkExplorerThemeProvider>
                <Router>
                  <App />
                </Router>
              </NetworkExplorerThemeProvider>
            </WalletProvider>
          </CosmosKitProvider>
        </MainContextProvider>
      </EnvironmentProvider>
    </ErrorBoundary>,
  );
}

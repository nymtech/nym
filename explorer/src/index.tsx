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
import './styles.css';

const elem = document.getElementById('app');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <ErrorBoundary FallbackComponent={ErrorBoundaryContent}>
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
    </ErrorBoundary>,
  );
}

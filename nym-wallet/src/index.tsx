import React, { useContext, useLayoutEffect } from 'react';
import ReactDOM from 'react-dom';
import { ErrorBoundary } from 'react-error-boundary';
import { BrowserRouter as Router } from 'react-router-dom';
import { SnackbarProvider } from 'notistack';
import { Routes } from './routes';
import { ClientContext, ClientContextProvider } from './context/main';
import { ApplicationLayout } from './layouts';
import { Admin, Welcome, Settings } from './pages';
import { ErrorFallback } from './components';
import { NymWalletTheme, WelcomeTheme } from './theme';
import { maximizeWindow } from './utils';
import { SignInProvider } from './pages/welcome/context';

const App = () => {
  const { clientDetails } = useContext(ClientContext);

  useLayoutEffect(() => {
    maximizeWindow();
  }, []);

  return !clientDetails ? (
    <WelcomeTheme>
      <SignInProvider>
        <Welcome />
      </SignInProvider>
    </WelcomeTheme>
  ) : (
    <NymWalletTheme>
      <ApplicationLayout>
        <Settings />
        <Admin />
        <Routes />
      </ApplicationLayout>
    </NymWalletTheme>
  );
};

const AppWrapper = () => (
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <Router>
      <SnackbarProvider
        anchorOrigin={{
          vertical: 'bottom',
          horizontal: 'right',
        }}
      >
        <ClientContextProvider>
          <App />
        </ClientContextProvider>
      </SnackbarProvider>
    </Router>
  </ErrorBoundary>
);

const root = document.getElementById('root');

ReactDOM.render(<AppWrapper />, root);

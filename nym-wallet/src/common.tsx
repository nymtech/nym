import React, { ComponentType, useEffect } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ErrorBoundary } from 'react-error-boundary';
import { BrowserRouter, HashRouter } from 'react-router-dom';
import { SnackbarProvider } from 'notistack';
import { AppProvider } from './context/main';
import { ErrorFallback } from './components';
import { NymWalletTheme } from './theme';
import { maximizeWindow } from './utils';
import { config } from './config';
import { useTauriTextEditingClipboard } from './hooks/useTauriTextEditingClipboard';

type RouterComponent = ComponentType<{ children?: React.ReactNode }>;

const walletQueryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

const ClipboardBridge: FCWithChildren = ({ children }) => {
  useTauriTextEditingClipboard();
  return children;
};

/** Auth (`index.html`) uses normal paths. Main (`main.html`) must pass {@link HashRouter} - see `main.tsx`. */
function selectRouter(): RouterComponent {
  if (typeof window === 'undefined') {
    return BrowserRouter;
  }
  const { pathname } = window.location;
  if (pathname === '/main.html' || pathname.endsWith('/main.html')) {
    return HashRouter;
  }
  return BrowserRouter;
}

export const AppCommon = ({
  children,
  Router: RouterProp,
}: {
  children: React.ReactNode;
  /** When set (main wallet entry), overrides auto-detect. Production Tauri often uses pathname `/` while still loading `main.html`, so main always passes `HashRouter`. */
  Router?: RouterComponent;
}) => {
  const Router = RouterProp ?? selectRouter();

  useEffect(() => {
    // do not maximise in dev mode, because it happens on hot reloading
    if (!config.IS_DEV_MODE) {
      maximizeWindow();
    }
  }, []);

  return (
    <ErrorBoundary FallbackComponent={ErrorFallback}>
      <Router>
        <QueryClientProvider client={walletQueryClient}>
          <SnackbarProvider
            anchorOrigin={{
              vertical: 'bottom',
              horizontal: 'right',
            }}
          >
            <AppProvider>
              <NymWalletTheme>
                <ClipboardBridge>{children}</ClipboardBridge>
              </NymWalletTheme>
            </AppProvider>
          </SnackbarProvider>
        </QueryClientProvider>
      </Router>
    </ErrorBoundary>
  );
};

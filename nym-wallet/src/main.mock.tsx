import React from 'react';
import { createRoot } from 'react-dom/client';
import { HashRouter, Navigate, Route, Routes } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ErrorBoundary } from 'react-error-boundary';
import { SnackbarProvider } from 'notistack';
import { ErrorFallback } from './components';
import { ApplicationLayout } from './layouts';
import { NymWalletTheme } from './theme';
import { FamilyPage } from './pages/families/FamilyPage';
import { FamilySettingsPage } from './pages/families/FamilySettingsPage';
import { MockMainContextProvider } from './context/mocks/main';
import { MockFamiliesContextProvider } from './context/mocks/families';
import {
  buildOperatorFlowStore,
  buildOwnerFlowStore,
  buildSeededStore,
  MOCK_OPERATOR_ADDRESS,
  MOCK_OWNER_ADDRESS,
} from './context/mocks/families.fixtures';
import type { MockStore } from './context/mocks/familiesMockState';

/**
 * Mock-wired entry for e2e (design D2). Mounts the real router + layout + Family page
 * but with the Storybook mocks ({@link MockMainContextProvider} for the app bootstrap,
 * {@link MockFamiliesContextProvider} for families) so it runs in a plain browser with
 * NO Tauri runtime and NO login gate. Built only when `WALLET_MOCK_FAMILIES=on`; the
 * production `main` entry and the real `/family` route are untouched.
 */

// Persona is chosen at runtime from `?persona=...` (default `owner`), read off
// `window.location.search` so it survives HashRouter (which owns the `#` fragment).
// Each persona maps to one of the deterministic Storybook fixture stores + its sender.
const PERSONAS: Record<string, { makeStore: () => MockStore; sender: string }> = {
  owner: { makeStore: buildOwnerFlowStore, sender: MOCK_OWNER_ADDRESS }, // owner lifecycle
  operator: { makeStore: buildOperatorFlowStore, sender: MOCK_OPERATOR_ADDRESS }, // operator lifecycle
  'operator-seeded': { makeStore: buildSeededStore, sender: MOCK_OPERATOR_ADDRESS }, // multi-node invite states
};
const personaKey = new URLSearchParams(window.location.search).get('persona') ?? 'owner';
const { makeStore, sender } = PERSONAS[personaKey] ?? PERSONAS.owner;
const store = makeStore();

// HashRouter starts at `#/`; seed it to the Family page so a bare URL lands there.
if (!window.location.hash || window.location.hash === '#' || window.location.hash === '#/') {
  const { pathname, search } = window.location;
  window.history.replaceState(window.history.state, '', `${pathname}${search}#/family`);
}

// Deterministic client for e2e: no retries, no cache carry-over between runs.
const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } });

const FamiliesMockShell = ({ children }: { children: React.ReactNode }) => (
  <MockFamiliesContextProvider store={store} sender={sender} latencyMs={0}>
    {children}
  </MockFamiliesContextProvider>
);

const MockApp = () => (
  <ErrorBoundary FallbackComponent={ErrorFallback}>
    <HashRouter>
      <QueryClientProvider client={queryClient}>
        <SnackbarProvider anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}>
          <MockMainContextProvider>
            <NymWalletTheme>
              <ApplicationLayout>
                <Routes>
                  <Route path="/" element={<Navigate to="/family" />} />
                  <Route
                    path="/family"
                    element={
                      <FamiliesMockShell>
                        <FamilyPage />
                      </FamiliesMockShell>
                    }
                  />
                  <Route
                    path="/family/settings"
                    element={
                      <FamiliesMockShell>
                        <FamilySettingsPage />
                      </FamiliesMockShell>
                    }
                  />
                </Routes>
              </ApplicationLayout>
            </NymWalletTheme>
          </MockMainContextProvider>
        </SnackbarProvider>
      </QueryClientProvider>
    </HashRouter>
  </ErrorBoundary>
);

const elem = document.getElementById('root');
if (elem) {
  createRoot(elem).render(<MockApp />);
}

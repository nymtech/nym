import React from 'react';
import { createRoot } from 'react-dom/client';
import { HashRouter } from 'react-router-dom';
import { AppRoutes } from './routes/app';
import { AppCommon } from './common';

/** Tauri may keep the document at `/` while still serving the main bundle; `BrowserRouter` + `/balance` then points at a non-existent path and the content pane stays blank until navigation. Hash history keeps the asset URL stable. */
function seedMainWalletHashToBalance() {
  if (typeof window === 'undefined') {
    return;
  }
  const h = window.location.hash;
  if (!h || h === '#' || h === '#/') {
    const { pathname, search } = window.location;
    window.history.replaceState(window.history.state, '', `${pathname}${search}#/balance`);
  }
}

seedMainWalletHashToBalance();

const MainApp = () => (
  <AppCommon Router={HashRouter}>
    <AppRoutes />
  </AppCommon>
);
const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(<MainApp />);
}

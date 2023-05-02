import React from 'react';
import { createRoot } from 'react-dom/client';
import { AppRoutes } from './routes/app';
import { AppCommon } from './common';

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

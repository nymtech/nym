import React from 'react';
import { Routes, Route } from 'react-router-dom';
import { ConnectionPage } from 'src/pages/connection';
import { Menu } from 'src/pages/menu';
import { CompatibleApps } from 'src/pages/menu/Apps';
import { HelpGuide } from 'src/pages/menu/Guide';

export const AppRoutes = () => (
  <Routes>
    <Route index path="/" element={<ConnectionPage />} />
    <Route path="menu">
      <Route index element={<Menu />} />
      <Route path="apps" element={<CompatibleApps />} />
      <Route path="guide" element={<HelpGuide />} />
    </Route>
  </Routes>
);

import { Routes, Route } from 'react-router-dom';
import { Menu } from 'src/pages/menu';
import { CompatibleApps } from 'src/pages/menu/apps';
import { HelpGuide } from 'src/pages/menu/guide';

export const AppRoutes = () => {
  return (
    <Routes>
      <Route index path="/" element={<div />} />
      <Route path="menu">
        <Route index element={<Menu />} />
        <Route path="apps" element={<CompatibleApps />} />
        <Route path="guide" element={<HelpGuide />} />
      </Route>
    </Routes>
  );
};

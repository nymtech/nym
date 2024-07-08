import { createRoot } from 'react-dom/client';
import { AppRoutes } from './routes/app';
import { AppCommon } from './common';

const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <AppCommon>
      <AppRoutes />
    </AppCommon>,
  );
}

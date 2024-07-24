import { createRoot } from 'react-dom/client';
import { AppCommon } from './common';
import { AuthRoutes } from './routes/auth';

const elem = document.getElementById('root');

if (elem) {
  const root = createRoot(elem);
  root.render(
    <AppCommon>
      <AuthRoutes />
    </AppCommon>,
  );
}

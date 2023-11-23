import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { Home, Settings, Error, PageLayout } from './pages';
import { MainStateProvider } from './state';
import './i18n/config';
import { ThemeSetter } from './ui';

const router = createBrowserRouter([
  {
    element: <PageLayout />,
    children: [
      {
        path: '/',
        element: <Home />,
        errorElement: <Error />,
      },
      {
        path: '/settings',
        element: <Settings />,
        errorElement: <Error />,
      },
    ],
  },
]);

function App() {
  return (
    <MainStateProvider>
      <ThemeSetter>
        <RouterProvider router={router} />
      </ThemeSetter>
    </MainStateProvider>
  );
}

export default App;

import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { Home, Settings, Error, PageLayout } from './pages';
import { MainStateProvider } from './state';
import './i18n/config';

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
      <RouterProvider router={router} />
    </MainStateProvider>
  );
}

export default App;

import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { Home, Settings, Error } from './pages';

const router = createBrowserRouter([
  {
    path: '/',
    element: <Home />,
    errorElement: <Error />,
  },
  {
    path: '/settings',
    element: <Settings />,
  },
]);

function App() {
  return <RouterProvider router={router} />;
}

export default App;

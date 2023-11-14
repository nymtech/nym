import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { Home, Settings } from './pages';

const router = createBrowserRouter([
  {
    path: '/',
    element: <Home />,
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

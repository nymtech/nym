import { createBrowserRouter } from 'react-router-dom';
import { Error, Home, NavLayout, NodeLocation, Settings } from './pages';

const router = createBrowserRouter([
  {
    element: <NavLayout />,
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
      {
        path: '/entry-node-location',
        // eslint-disable-next-line react/jsx-no-undef
        element: <NodeLocation node="entry" />,
        errorElement: <Error />,
      },
      {
        path: '/exit-node-location',
        element: <NodeLocation node="exit" />,
        errorElement: <Error />,
      },
    ],
  },
]);

export default router;

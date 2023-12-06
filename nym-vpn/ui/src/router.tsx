import { createBrowserRouter } from 'react-router-dom';
import { Error, Home, NavLayout, NodeLocation, Settings } from './pages';
import { routes } from './constants';

const router = createBrowserRouter([
  {
    path: routes.root,
    element: <NavLayout />,
    children: [
      {
        element: <Home />,
        errorElement: <Error />,
        index: true,
      },
      {
        path: routes.settings,
        element: <Settings />,
        errorElement: <Error />,
      },
      {
        path: routes.entryNodeLocation,
        // eslint-disable-next-line react/jsx-no-undef
        element: <NodeLocation node="entry" />,
        errorElement: <Error />,
      },
      {
        path: routes.exitNodeLocation,
        element: <NodeLocation node="exit" />,
        errorElement: <Error />,
      },
    ],
  },
]);

export default router;

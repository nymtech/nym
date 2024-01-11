import { createBrowserRouter } from 'react-router-dom';
import {
  Display,
  Error,
  Feedback,
  Home,
  Legal,
  MainLayout,
  NodeLocation,
  Settings,
  SettingsLayout,
} from './pages';
import { routes } from './constants';

const router = createBrowserRouter([
  {
    path: routes.root,
    element: <MainLayout />,
    children: [
      {
        element: <Home />,
        errorElement: <Error />,
        index: true,
      },
      {
        path: routes.settings,
        element: <SettingsLayout />,
        errorElement: <Error />,
        children: [
          {
            element: <Settings />,
            errorElement: <Error />,
            index: true,
          },
          {
            path: routes.display,
            element: <Display />,
            errorElement: <Error />,
          },
          {
            path: routes.feedback,
            element: <Feedback />,
            errorElement: <Error />,
          },
          {
            path: routes.legal,
            element: <Legal />,
            errorElement: <Error />,
          },
        ],
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

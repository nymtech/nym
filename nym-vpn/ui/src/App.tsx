import { RouterProvider } from 'react-router-dom';
import router from './router';
import { MainStateProvider } from './state';
import './i18n/config';
import { ThemeSetter } from './ui';

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

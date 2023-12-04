import { RouterProvider } from 'react-router-dom';
import dayjs from 'dayjs';
import { useTranslation } from 'react-i18next';
import router from './router';
import { MainStateProvider } from './state';
import './i18n/config';
import { ThemeSetter } from './ui';

function App() {
  const { i18n } = useTranslation();
  dayjs.locale(i18n.language);

  return (
    <MainStateProvider>
      <ThemeSetter>
        <RouterProvider router={router} />
      </ThemeSetter>
    </MainStateProvider>
  );
}

export default App;

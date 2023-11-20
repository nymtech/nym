import { useContext } from 'react';
import { MainStateContext } from '../contexts';
import { useTranslation } from 'react-i18next';

function Home() {
  const state = useContext(MainStateContext);
  const { t } = useTranslation();

  return (
    <div>
      <h2>NymVPN</h2>
      connection state: {state.state}
      <button>{t('connect')}</button>
    </div>
  );
}

export default Home;

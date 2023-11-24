import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api';
import { useMainDispatch, useMainState } from '../contexts';
import { StateDispatch } from '../types';

function Home() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const { t } = useTranslation();

  const handleClick = async () => {
    if (state.state === 'Connected') {
      dispatch({ type: 'disconnect' });
      invoke('disconnect').then((result) => {
        console.log(result);
      });
    } else if (state.state === 'Disconnected') {
      dispatch({ type: 'connect' });
      invoke('connect').then((result) => {
        console.log(result);
      });
    }
  };

  return (
    <div>
      connection state: {state.state}
      {state.loading ? (
        'loading…'
      ) : (
        <button onClick={handleClick}>
          {state.state === 'Disconnected' ? t('connect') : t('disconnect')}
        </button>
      )}
    </div>
  );
}

export default Home;

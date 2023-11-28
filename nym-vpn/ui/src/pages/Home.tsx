import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
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

  const statusBadgeDynStyles = {
    Connected: [
      'bg-blanc-nacre-icicle',
      'text-vert-menthe',
      'dark:bg-baltic-sea-quartzite',
    ],
    Disconnected: [
      'bg-blanc-nacre-platinum',
      'text-coal-mine-light',
      'dark:bg-baltic-sea-oil',
      'dark:text-coal-mine-dark',
    ],
    Connecting: [
      'bg-blanc-nacre-platinum',
      'text-baltic-sea',
      'dark:bg-baltic-sea-oil',
      'dark:text-white',
    ],
    Disconnecting: [
      'bg-blanc-nacre-platinum',
      'text-baltic-sea',
      'dark:bg-baltic-sea-oil',
      'dark:text-white',
    ],
    Error: [
      'bg-blanc-nacre-platinum',
      'text-baltic-sea',
      'dark:bg-baltic-sea-oil',
      'dark:text-white',
    ],
  };

  return (
    <div>
      <div className="h-80">
        <div
          className={clsx([
            ...statusBadgeDynStyles[state.state],
            'font-bold py-4 px-6 rounded-full',
          ])}
        >
          {state.state}
        </div>
      </div>
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

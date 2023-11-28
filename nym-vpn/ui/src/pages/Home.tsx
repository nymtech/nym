import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
import { useMainDispatch, useMainState } from '../contexts';
import { ConnectionState, StateDispatch } from '../types';
import { ConnectionTimer } from '../ui';

function Home() {
  const state = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;

  const { t } = useTranslation('home');

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
    Unknown: [
      'bg-blanc-nacre-platinum',
      'text-coal-mine-light',
      'dark:bg-baltic-sea-oil',
      'dark:text-coal-mine-dark',
    ],
  };

  const getStatusText = (state: ConnectionState) => {
    switch (state) {
      case 'Connected':
        return t('status.connected');
      case 'Disconnected':
        return t('status.disconnected');
      case 'Connecting':
        return t('status.connecting');
      case 'Disconnecting':
        return t('status.disconnecting');
      case 'Unknown':
        return t('status.unknown');
    }
  };

  return (
    <div>
      <div className="h-80 flex flex-col justify-center items-center">
        <div
          className={clsx([
            ...statusBadgeDynStyles[state.state],
            'font-bold py-4 px-6 rounded-full text-lg',
          ])}
        >
          {getStatusText(state.state)}
        </div>
        {state.loading && state.progressMessages.length > 0 && (
          <p className="text-dim-gray dark:text-mercury-mist font-bold">
            {state.progressMessages[state.progressMessages.length - 1]}
          </p>
        )}
        {state.state === 'Connected' && <ConnectionTimer />}
        {state.error && (
          <p className="text-teaberry font-bold">{state.error}</p>
        )}
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

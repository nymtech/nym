import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
import { Button } from '@mui/base';
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

  const getButtonText = (state: ConnectionState) => {
    switch (state) {
      case 'Connected':
        return t('disconnect');
      case 'Disconnected':
        return t('connect');
      case 'Connecting':
        return (
          <div className="flex justify-center items-center animate-spin">
            <span className="font-icon text-2xl font-medium">autorenew</span>
          </div>
        );
      case 'Disconnecting':
        return (
          <div className="flex justify-center items-center animate-spin">
            <span className="font-icon text-2xl font-medium">autorenew</span>
          </div>
        );
      case 'Unknown':
        return t('status.unknown');
    }
  };

  return (
    <div className="h-full flex flex-col p-4">
      <div className="h-80 flex flex-col justify-center items-center gap-y-2">
        <div className="flex flex-1 items-end">
          <div
            className={clsx([
              ...statusBadgeDynStyles[state.state],
              'font-bold py-4 px-6 rounded-full text-lg',
            ])}
          >
            {getStatusText(state.state)}
          </div>
        </div>
        <div className="flex flex-1">
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
      </div>
      <div className="flex grow flex-col justify-between gap-y-2">
        <div />
        <Button
          className={clsx([
            'rounded-lg text-lg font-bold py-4 px-6 h-16',
            (state.state === 'Disconnected' || state.state === 'Connecting') &&
              'bg-melon text-white dark:text-baltic-sea',
            (state.state === 'Connected' || state.state === 'Disconnecting') &&
              'bg-cornflower text-white dark:text-baltic-sea',
          ])}
          onClick={handleClick}
          disabled={state.loading}
        >
          {getButtonText(state.state)}
        </Button>
      </div>
    </div>
  );
}

export default Home;

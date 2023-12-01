import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
import { Button } from '@mui/base';
import { useMainDispatch, useMainState } from '../../contexts';
import { ConnectionState, StateDispatch } from '../../types';
import NetworkModeSelect from './NetworkModeSelect';
import ConnectionStatus from './ConnectionStatus';

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
      <ConnectionStatus />
      <div className="flex grow flex-col justify-between gap-y-2">
        <div className="flex flex-col justify-between">
          <NetworkModeSelect />
          <div></div>
        </div>
        <Button
          className={clsx([
            'rounded-lg text-lg font-bold py-4 px-6 h-16 focus:outline-none focus:ring-4 focus:ring-black focus:dark:ring-white shadow',
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

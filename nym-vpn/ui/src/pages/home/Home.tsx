import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api';
import clsx from 'clsx';
import { Button } from '@mui/base';
import { useNavigate } from 'react-router-dom';
import { useMainDispatch, useMainState } from '../../contexts';
import { CmdError, StateDispatch } from '../../types';
import { routes } from '../../constants';
import NetworkModeSelect from './NetworkModeSelect';
import ConnectionStatus from './ConnectionStatus';
import HopSelect from './HopSelect';

function Home() {
  const {
    state,
    loading,
    entryNodeLocation,
    exitNodeLocation,
    entrySelector,
    defaultNodeLocation,
  } = useMainState();
  const dispatch = useMainDispatch() as StateDispatch;
  const navigate = useNavigate();
  const { t } = useTranslation('home');

  const handleClick = async () => {
    dispatch({ type: 'disconnect' });
    if (state === 'Connected') {
      invoke('disconnect')
        .then((result) => {
          console.log('disconnect result');
          console.log(result);
        })
        .catch((e: CmdError) => {
          console.warn(`backend error: ${e.source} - ${e.message}`);
          dispatch({ type: 'set-error', error: e.message });
        });
    } else if (state === 'Disconnected') {
      dispatch({ type: 'connect' });
      invoke('connect')
        .then((result) => {
          console.log('connect result');
          console.log(result);
        })
        .catch((e: CmdError) => {
          console.warn(`backend error: ${e.source} - ${e.message}`);
          dispatch({ type: 'set-error', error: e.message });
        });
    }
  };

  const getButtonText = useCallback(() => {
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
  }, [state, t]);

  return (
    <div className="h-full flex flex-col p-4">
      <div className="grow">
        <ConnectionStatus />
      </div>
      <div className="flex flex-col justify-between gap-y-8">
        <div className="flex flex-col justify-between gap-y-4">
          <NetworkModeSelect />
          <div className="flex flex-col gap-6">
            <div className="mt-3 text-base font-semibold">
              {t('select-node-title')}
            </div>
            <div className="flex flex-col gap-5">
              {entrySelector && (
                <HopSelect
                  country={entryNodeLocation || defaultNodeLocation}
                  onClick={() => {
                    if (state === 'Disconnected') {
                      navigate(routes.entryNodeLocation);
                    }
                  }}
                  nodeHop="entry"
                />
              )}
              <HopSelect
                country={exitNodeLocation || defaultNodeLocation}
                onClick={() => {
                  if (state === 'Disconnected') {
                    navigate(routes.exitNodeLocation);
                  }
                }}
                nodeHop="exit"
              />
            </div>
          </div>
        </div>
        <Button
          className={clsx([
            'flex justify-center items-center',
            'rounded-lg text-lg font-bold py-3 px-6',
            'focus:outline-none focus:ring-4 focus:ring-black focus:dark:ring-white shadow',
            (state === 'Disconnected' || state === 'Connecting') &&
              'bg-melon text-white dark:text-baltic-sea',
            (state === 'Connected' || state === 'Disconnecting') &&
              'bg-cornflower text-white dark:text-baltic-sea',
          ])}
          onClick={handleClick}
          disabled={loading}
        >
          {getButtonText()}
        </Button>
      </div>
    </div>
  );
}

export default Home;

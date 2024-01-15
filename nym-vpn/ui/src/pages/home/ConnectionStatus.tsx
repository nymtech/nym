import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { ConnectionState } from '../../types';
import { useMainState } from '../../contexts';
import ConnectionTimer from './ConnectionTimer';

function ConnectionStatus() {
  const state = useMainState();

  const { t } = useTranslation('home');

  const statusBadgeDynStyles = {
    Connected: ['text-vert-menthe', 'bg-vert-prasin bg-opacity-10'],
    Disconnected: [
      'bg-cement-feet bg-opacity-10',
      'text-coal-mine-light',
      'dark:bg-oil dark:bg-opacity-15',
      'dark:text-coal-mine-dark',
    ],
    Connecting: [
      'bg-cement-feet bg-opacity-10',
      'text-baltic-sea',
      'dark:bg-oil dark:bg-opacity-15',
      'dark:text-white',
    ],
    Disconnecting: [
      'bg-cement-feet bg-opacity-10',
      'text-baltic-sea',
      'dark:bg-oil dark:bg-opacity-15',
      'dark:text-white',
    ],
    Unknown: [
      'bg-cement-feet bg-opacity-10',
      'text-coal-mine-light',
      'dark:bg-oil dark:bg-opacity-15',
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
    <div className="h-full min-h-52 flex flex-col justify-center items-center gap-y-2">
      <div className="flex flex-1 items-end select-none hover:cursor-default">
        <div
          className={clsx([
            ...statusBadgeDynStyles[state.state],
            'text-lg font-bold py-3 px-6 rounded-full',
          ])}
        >
          {getStatusText(state.state)}
        </div>
      </div>
      <div className="w-full flex flex-col flex-1 items-center overflow-hidden">
        {state.loading && state.progressMessages.length > 0 && !state.error && (
          <div className="w-4/5 h-2/3 overflow-auto break-words text-center">
            <p className="text-sm text-dim-gray dark:text-mercury-mist font-bold">
              {t(
                `connection-progress.${
                  state.progressMessages[state.progressMessages.length - 1]
                }`,
                {
                  ns: 'backendMessages',
                },
              )}
            </p>
          </div>
        )}
        {state.state === 'Connected' && <ConnectionTimer />}
        {state.error && (
          <div className="w-4/5 h-2/3 overflow-auto break-words text-center">
            <p className="text-sm text-teaberry font-bold">{state.error}</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default ConnectionStatus;

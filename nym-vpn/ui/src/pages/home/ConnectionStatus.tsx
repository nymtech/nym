import clsx from 'clsx';
import { useTranslation } from 'react-i18next';
import { ConnectionState } from '../../types';
import { useMainState } from '../../contexts';
import ConnectionTimer from './ConnectionTimer';

function ConnectionStatus() {
  const state = useMainState();

  const { t } = useTranslation('home');
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
    <div className="h-72 flex flex-col justify-center items-center gap-y-2">
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
      <div className="w-full flex flex-col flex-1 items-center overflow-hidden">
        {state.loading && state.progressMessages.length > 0 && !state.error && (
          <div className="w-4/5 h-2/3 overflow-scroll break-words text-center">
            <p className="text-dim-gray dark:text-mercury-mist font-bold">
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
          <div className="w-4/5 h-2/3 overflow-scroll break-words text-center">
            <p className="text-teaberry font-bold">{state.error}</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default ConnectionStatus;

import React from 'react';
import { Typography } from '@mui/material';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { useClientContext } from 'src/context/main';
import { InfoModal } from 'src/components/InfoModal';
import { Error } from 'src/types/error';
import { ExperimentalWarning } from 'src/components/ExperimentalWarning';
import { ServiceProvider, Services } from 'src/types/directory';
import { ConnectionStatusKind } from 'src/types';
import { ConnectionButton } from 'src/components/ConnectionButton';

export const Disconnected: FCWithChildren<{
  error?: Error;
  status: ConnectionStatusKind;
  services?: Services;
  busy?: boolean;
  isError?: boolean;
  serviceProvider?: ServiceProvider;
  clearError: () => void;
  onConnectClick?: (status: ConnectionStatusKind) => void;
}> = ({ status, error, busy, isError, onConnectClick, clearError, serviceProvider }) => {
  return (
    <>
      {error && <InfoModal show title={error.title} description={error.message} onClose={clearError} />}
      <ConnectionStatus status={'disconnected'} />
      <ConnectionTimer />
      <ConnectionButton
        status={status}
        disabled={serviceProvider === undefined}
        busy={busy}
        isError={isError}
        onClick={onConnectClick}
      />
      <Typography
        fontWeight={600}
        textTransform="uppercase"
        textAlign="center"
        fontSize="12px"
        sx={{ wordSpacing: 1.5, letterSpacing: 1.5 }}
        color="warning.main"
      >
        You are not protected
      </Typography>
      <ExperimentalWarning />
    </>
  );
};

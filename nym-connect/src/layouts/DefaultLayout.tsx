import React from 'react';
import { Box, Typography } from '@mui/material';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { InfoModal } from 'src/components/InfoModal';
import { Error } from 'src/types/error';
import { ConnectionButton } from '../components/ConnectionButton';
import { useClientContext } from '../context/main';
import { ConnectionStatusKind } from '../types';
import { Services } from '../types/directory';
import { ExperimentalWarning } from 'src/components/ExperimentalWarning';

export const DefaultLayout: FCWithChildren<{
  error?: Error;
  status: ConnectionStatusKind;
  services?: Services;
  busy?: boolean;
  isError?: boolean;
  clearError: () => void;
  onConnectClick?: (status: ConnectionStatusKind) => void;
}> = ({ status, error, busy, isError, onConnectClick, clearError }) => {
  const context = useClientContext();

  return (
    <>
      {error && <InfoModal show title={error.title} description={error.message} onClose={clearError} />}
      <ConnectionStatus status={ConnectionStatusKind.disconnected} />
      <ConnectionTimer />
      <ConnectionButton
        status={status}
        disabled={context.serviceProvider === undefined}
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

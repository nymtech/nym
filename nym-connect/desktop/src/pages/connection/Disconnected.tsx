import React from 'react';
import { Alert, Link, Stack, Typography } from '@mui/material';
import { Link as RouterLink } from 'react-router-dom';
import { ConnectionStatus } from 'src/components/ConnectionStatus';
import { ConnectionTimer } from 'src/components/ConntectionTimer';
import { InfoModal } from 'src/components/InfoModal';
import { Error } from 'src/types/error';
import { ExperimentalWarning } from 'src/components/ExperimentalWarning';
import { ServiceProvider, Services } from 'src/types/directory';
import { ConnectionStatusKind } from 'src/types';
import { PowerButton } from 'src/components/PowerButton/PowerButton';
import { Box } from '@mui/system';
import { ConnectionLayout } from 'src/layouts/ConnectionLayout';
import { useClientContext } from '../../context/main';

export const Disconnected: FCWithChildren<{
  error?: Error;
  status: ConnectionStatusKind;
  services?: Services;
  busy?: boolean;
  isError?: boolean;
  serviceProvider?: ServiceProvider;
  clearError: () => void;
  onConnectClick: (status: ConnectionStatusKind) => void;
}> = ({ status, error, onConnectClick, clearError }) => {
  const { showFeedbackNote, setShowFeedbackNote } = useClientContext();

  return (
    <>
      {error && <InfoModal show title={error.title} description={error.message} onClose={clearError} />}
      <ConnectionLayout
        TopContent={
          <Box>
            <ConnectionStatus status={ConnectionStatusKind.disconnected} gatewayPerformance="Good" />
            <ConnectionTimer />
          </Box>
        }
        ConnectButton={<PowerButton onClick={onConnectClick} status={status} disabled={status === 'connecting'} />}
        BottomContent={
          <Stack justifyContent="space-between" pt={1}>
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

            {showFeedbackNote ? (
              <Alert variant="outlined" icon={false} onClose={() => setShowFeedbackNote(false)}>
                Help improve NymConnect
                <br />
                <Link
                  to="/menu/reporting/user-feedback"
                  onClick={() => setShowFeedbackNote(false)}
                  component={RouterLink}
                  color="secondary"
                  underline="hover"
                >
                  Send feedback
                </Link>
              </Alert>
            ) : (
              <ExperimentalWarning />
            )}
          </Stack>
        }
      />
    </>
  );
};

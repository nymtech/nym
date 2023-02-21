import React from 'react';
import { Box, CircularProgress, LinearProgress, Stack, Typography } from '@mui/material';
import { useClientContext } from '../../context/main';
import ErrorContent from './content/TestAndEarn/Error.mdx';
import ContentStep0 from './content/TestAndEarn/Stage0_intro.mdx';
import ContentNotAvailable from './content/TestAndEarnNotAvaialble.mdx';
import { ConnectionStatusKind } from '../../types';
import { useTestAndEarnContext } from './context/TestAndEarnContext';
import { TestAndEarnWinnerWithState } from './TestAndEarnWinner';
import { TestAndEarnCurrentDrawWithState } from './TestAndEarnCurrentDraw';
import { TestAndEarnDrawsWithState } from './TestAndEarnDraws';

enum Stages {
  mustRegister = 'mustRegister',
  registered = 'registered',
}

export const TestAndEarnPopupContent: React.FC<{
  stage?: string;
  connectionStatus?: ConnectionStatusKind;
  error?: string;
}> = ({ connectionStatus, error, stage = Stages.mustRegister }) => {
  if (error) {
    return (
      <Box p={4}>
        <ErrorContent error={error} />
      </Box>
    );
  }

  if (!connectionStatus || connectionStatus === ConnectionStatusKind.disconnected) {
    return (
      <Box p={4}>
        <ContentNotAvailable />
      </Box>
    );
  }

  if (connectionStatus === ConnectionStatusKind.connecting || connectionStatus === ConnectionStatusKind.disconnecting) {
    return (
      <Box p={4} justifyContent="center" alignItems="center" display="flex">
        <CircularProgress />
        <Typography ml={3}>Please wait...</Typography>
      </Box>
    );
  }

  switch (stage) {
    case Stages.mustRegister:
      return (
        <Box p={4}>
          <ContentStep0 />
        </Box>
      );
    case Stages.registered:
      return (
        <Box p={4}>
          <TestAndEarnWinnerWithState />
          <TestAndEarnCurrentDrawWithState />
          <TestAndEarnDrawsWithState />
        </Box>
      );
    default:
      return (
        <Box p={4}>
          <Stack direction="row" spacing={2} display="flex" alignItems="center">
            <CircularProgress />
            <Box>Waiting for task information...</Box>
          </Stack>
        </Box>
      );
  }
};

export const TestAndEarnPopup: React.FC = () => {
  const clientContext = useClientContext();
  const context = useTestAndEarnContext();

  React.useEffect(() => {
    if (clientContext.connectionStatus === ConnectionStatusKind.connected) {
      context.refresh();
    }
  }, [clientContext.connectionStatus]);

  const stage = React.useMemo<Stages>(() => {
    if (context.registration) {
      return Stages.registered;
    }
    return Stages.mustRegister;
  }, [context.registration?.id]);

  React.useEffect(() => {
    const interval = setInterval(context.refresh, 1000 * 60 * 5);
    return () => clearInterval(interval);
  }, []);

  if (!context.loadedOnce && clientContext.connectionStatus === ConnectionStatusKind.connected) {
    const message = 'Waiting for data to be transferred over the mixnet...';
    return (
      <Box p={4}>
        <Stack direction="row" spacing={2} display="flex" alignItems="center">
          <CircularProgress />
          <Box>{message}</Box>
          {/* {process.env.NODE_ENV === 'development' && <pre>{JSON.stringify(context, null, 2)}</pre>} */}
        </Stack>
      </Box>
    );
  }

  return (
    <>
      {context.loading && <LinearProgress />}
      {/* <Button onClick={context.refresh}>Refresh</Button> */}
      <TestAndEarnPopupContent connectionStatus={clientContext.connectionStatus} stage={stage} error={context.error} />
      {/* {process.env.NODE_ENV === 'development' && <pre>{JSON.stringify(context, null, 2)}</pre>} */}
    </>
  );
};

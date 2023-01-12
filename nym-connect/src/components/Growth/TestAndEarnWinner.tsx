import React from 'react';
import { Alert, AlertTitle, Button, Card, CardContent, CardMedia, Dialog, Typography } from '@mui/material';
import { SxProps } from '@mui/system';
import LoadingButton from '@mui/lab/LoadingButton';
import winner from './content/assets/winner.webp';
import { useTestAndEarnContext } from './context/TestAndEarnContext';
import { DrawEntry, DrawEntryStatus } from './context/types';
import { TestAndEarnEnterWalletAddress } from './TestAndEarnEnterWalletAddress';
import Content from './content/en.yaml';

export const TestAndEarnWinner: FCWithChildren<{
  sx?: SxProps;
  entry?: DrawEntry;
}> = ({ sx, entry }) => {
  const context = useTestAndEarnContext();
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string>();
  const [showWalletCapture, setShowWalletCapture] = React.useState(false);

  const clear = () => {
    setShowWalletCapture(false);
    setError(undefined);
    setBusy(false);
  };

  const handleStartWalletCapture = async () => {
    setBusy(true);
    setShowWalletCapture(true);
  };

  const cancelEndWalletCapture = async () => {
    setBusy(false);
    setShowWalletCapture(false);
  };

  const handleEndWalletCapture = async () => {
    setBusy(true);
    setShowWalletCapture(false);

    if (!context.walletAddress) {
      setError('Wallet address is not set');
      return;
    }
    if (!entry?.draw_id) {
      setError('Draw id is not set');
      return;
    }

    try {
      await context.claim(entry.draw_id, context.walletAddress);
    } catch (e) {
      const message = `${e}`;
      console.error('Failed to submit claim', entry.draw_id, context.walletAddress);
      setError(message);
    }
    setBusy(false);
  };

  return (
    <>
      {showWalletCapture && (
        <Dialog open fullWidth onBackdropClick={cancelEndWalletCapture}>
          <TestAndEarnEnterWalletAddress onSubmit={handleEndWalletCapture} />
        </Dialog>
      )}
      <Card sx={{ mb: 2 }}>
        <CardMedia component="img" height="165" image={winner} alt="winner" />
        <CardContent>
          <Typography color="warning.main" fontSize={20} fontWeight="bold">
            {Content.testAndEarn.winner.card.header}
          </Typography>
          <Typography mt={2}>
            {entry && (
              <>
                {Content.testAndEarn.winner.card.text} {entry.draw_id}.
              </>
            )}
            <LoadingButton
              loading={busy}
              variant="contained"
              sx={{ ml: 2, my: 2 }}
              size="small"
              onClick={handleStartWalletCapture}
            >
              {Content.testAndEarn.winner.claimButton.text}
            </LoadingButton>
          </Typography>
          {error && (
            <Alert severity="error" variant="filled">
              <AlertTitle>Oh no! Failed to submit claim</AlertTitle>
              {error}
              <Button variant="contained" color="secondary" size="small" onClick={() => clear()} sx={{ mx: 2 }}>
                Try again!
              </Button>
            </Alert>
          )}
        </CardContent>
      </Card>
    </>
  );
};

export const TestAndEarnWinnerWithState: FCWithChildren<{
  sx?: SxProps;
}> = ({ sx }) => {
  const context = useTestAndEarnContext();

  if (context.draws?.current?.entry?.status === DrawEntryStatus.winner) {
    return <TestAndEarnWinner sx={sx} entry={context.draws.current.entry} />;
  }

  // when the user does not have any unclaimed prizes, don't render anything
  return null;
};

import React from 'react';
import LoadingButton from '@mui/lab/LoadingButton';
import {
  Alert,
  AlertTitle,
  Button,
  Card,
  CardContent,
  Chip,
  Dialog,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableRow,
  Tooltip,
  Typography,
} from '@mui/material';
import { SxProps } from '@mui/system';
import { DateTime } from 'luxon';
import { useTestAndEarnContext } from './context/TestAndEarnContext';
import { DrawEntry, DrawEntryStatus } from './context/types';
import { CopyToClipboard } from '../CopyToClipboard';
import { TestAndEarnEnterWalletAddress } from './TestAndEarnEnterWalletAddress';
import Content from './content/en.yaml';

const statusToText = (status: string): string => Content.testAndEarn.status.chip[status] || '-';

const statusToColor = (status: string): 'info' | 'success' | 'warning' | undefined => {
  switch (status) {
    case DrawEntryStatus.pending:
      return 'info';
    case DrawEntryStatus.winner:
      return 'warning';
    case DrawEntryStatus.claimed:
      return 'success';
    default:
      return undefined;
  }
};

const StatusText: FCWithChildren<{ entry: DrawEntry }> = ({ entry }) => {
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
    if (!entry.draw_id) {
      setError('Task id is not set');
      return;
    }

    try {
      await context.claim(entry.draw_id, context.walletAddress);
    } catch (e) {
      const message = `${e}`;
      console.error('Failed to submit claim');
      setError(message);
    }
    setBusy(false);
  };

  if (error) {
    return (
      <Alert severity="error" variant="filled">
        <AlertTitle>Oh no! Failed to submit claim</AlertTitle>
        {error}
        <Button variant="contained" color="secondary" size="small" onClick={() => clear()} sx={{ mx: 2 }}>
          Try again!
        </Button>
      </Alert>
    );
  }

  if (showWalletCapture) {
    return (
      <Dialog open fullWidth onBackdropClick={cancelEndWalletCapture}>
        <TestAndEarnEnterWalletAddress onSubmit={handleEndWalletCapture} />
      </Dialog>
    );
  }

  switch (entry.status) {
    case DrawEntryStatus.pending:
      return <>{Content.testAndEarn.status.text.Pending}</>;
    case DrawEntryStatus.winner:
      return (
        <>
          {Content.testAndEarn.status.text.Winner}
          <LoadingButton
            loading={busy}
            disabled={busy}
            variant="contained"
            sx={{ ml: 2 }}
            size="small"
            onClick={handleStartWalletCapture}
          >
            {Content.testAndEarn.winner.claimButton.text}
          </LoadingButton>
        </>
      );
    case DrawEntryStatus.claimed:
      return <>{Content.testAndEarn.status.text.Claimed}</>;
    case DrawEntryStatus.noWin:
      return <>{Content.testAndEarn.status.text.NoWin}</>;
    default:
      return null;
  }
};

export const TestAndEarnDraws: FCWithChildren<{
  sx?: SxProps;
}> = ({ sx }) => {
  const context = useTestAndEarnContext();

  const draws = React.useMemo<DrawEntry[]>(
    () =>
      (context.draws?.draws || []).map((item) => ({
        ...item,
        timestamp: DateTime.fromISO(item.timestamp).toLocaleString(DateTime.DATETIME_FULL),
      })),
    [context.draws?.draws],
  );

  if (!context.draws) {
    return null;
  }

  return (
    <Card sx={{ mb: 2 }}>
      <CardContent>
        <Typography mb={2}>Here is a history of the tasks you have completed:</Typography>
        <TableContainer>
          <Table>
            <TableBody>
              {draws.map((entry) => (
                <TableRow key={entry.draw_id}>
                  <TableCell width="150px">{entry.timestamp}</TableCell>
                  <TableCell width="150px">
                    <Tooltip arrow title={`Task Id: ${entry.draw_id}`}>
                      <Chip label={statusToText(entry.status)} color={statusToColor(entry.status)} />
                    </Tooltip>
                  </TableCell>
                  <TableCell>
                    <StatusText entry={entry} />
                  </TableCell>
                  <TableCell>
                    {entry.id} <CopyToClipboard iconButton light text={entry.id} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </CardContent>
    </Card>
  );
};

export const TestAndEarnDrawsWithState: FCWithChildren<{
  sx?: SxProps;
}> = ({ sx }) => {
  const context = useTestAndEarnContext();

  const drawCount = context.draws?.draws?.length || 0;
  if (drawCount < 1) {
    return null;
  }

  return <TestAndEarnDraws sx={sx} />;
};

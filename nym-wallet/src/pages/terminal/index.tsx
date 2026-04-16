import React, { useContext, useState } from 'react';
import { Alert, Box, Dialog, IconButton, Paper, Stack, Typography, alpha, useTheme } from '@mui/material';
import TerminalIcon from '@mui/icons-material/Terminal';
import RefreshIcon from '@mui/icons-material/Refresh';
import CloseIcon from '@mui/icons-material/Close';
import UnfoldMoreIcon from '@mui/icons-material/UnfoldMore';
import UnfoldLessIcon from '@mui/icons-material/UnfoldLess';
import { Console } from 'src/utils/console';
import { AppContext } from '../../context/main';
import { NymCard } from '../../components';
import { getCurrentInterval, getAllPendingDelegations, getMixNodeDelegationsForCurrentAccount } from '../../requests';

const TerminalSection: FCWithChildren<{
  heading: string;
  children: React.ReactNode;
}> = ({ heading, children }) => {
  const [isCollapsed, setIsCollapsed] = useState<boolean>(true);

  return (
    <Box
      mb={1.5}
      sx={{
        borderRadius: 2,
        border: (t) => `1px solid ${t.palette.divider}`,
        overflow: 'hidden',
      }}
    >
      <Stack
        direction="row"
        alignItems="center"
        onClick={() => setIsCollapsed((prev) => !prev)}
        sx={{
          px: 2,
          py: 1.25,
          cursor: 'pointer',
          bgcolor: (t) => alpha(t.palette.primary.main, t.palette.mode === 'dark' ? 0.08 : 0.06),
          '&:hover': {
            bgcolor: (t) => alpha(t.palette.primary.main, t.palette.mode === 'dark' ? 0.12 : 0.09),
          },
        }}
      >
        <IconButton size="small" edge="start" sx={{ mr: 0.5 }} aria-expanded={!isCollapsed}>
          {isCollapsed ? <UnfoldMoreIcon fontSize="small" /> : <UnfoldLessIcon fontSize="small" />}
        </IconButton>
        <Typography variant="subtitle2" fontWeight={600} sx={{ flex: 1 }}>
          {heading}
        </Typography>
      </Stack>
      {!isCollapsed && (
        <Paper
          elevation={0}
          sx={{
            px: 2,
            py: 1.5,
            borderRadius: 0,
            bgcolor: 'background.default',
            borderTop: (t) => `1px solid ${t.palette.divider}`,
          }}
        >
          <Box
            component="pre"
            sx={{
              m: 0,
              fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
              fontSize: '0.75rem',
              lineHeight: 1.5,
              overflow: 'auto',
              maxHeight: 280,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
            }}
          >
            {children}
          </Box>
        </Paper>
      )}
    </Box>
  );
};

const TerminalInner: FCWithChildren = () => {
  const theme = useTheme();
  const { network, userBalance, clientDetails, handleShowTerminal, appEnv } = useContext(AppContext);
  const [mixnodeDelegations, setMixnodeDelegations] = useState<any>();
  const [pendingEvents, setPendingEvents] = useState<any>();
  const [pendingVestingEvents] = useState<any>();
  const [epoch, setEpoch] = useState<any>();
  const [isBusy, setIsBusy] = useState<boolean>();
  const [error, setError] = useState<any>();
  const [status, setStatus] = useState<string | undefined>();

  const withErrorCatch = async (fn: () => Promise<void>) => {
    try {
      await fn();
    } catch (e) {
      Console.error(e);
      setError(e);
    }
  };

  const refresh = async () => {
    setError(undefined);
    setIsBusy(true);
    setStatus('Getting all mixnode delegations for this account...');
    await withErrorCatch(async () => {
      setMixnodeDelegations(await getMixNodeDelegationsForCurrentAccount());
    });
    setStatus('Getting pending delegations...');
    await withErrorCatch(async () => {
      setPendingEvents(await getAllPendingDelegations());
    });
    setStatus('Getting current epoch...');
    await withErrorCatch(async () => {
      setEpoch(await getCurrentInterval());
    });
    setStatus('Fetching balance...');
    await withErrorCatch(async () => {
      await userBalance.fetchBalance();
    });
    setStatus('Fetching token allocation...');
    await withErrorCatch(async () => {
      await userBalance.fetchTokenAllocation();
    });
    setStatus(undefined);
    setIsBusy(false);
  };

  React.useEffect(() => {
    refresh();
  }, [network]);

  const successBg = alpha(theme.palette.success.main, theme.palette.mode === 'dark' ? 0.22 : 0.12);
  const successColor = theme.palette.mode === 'dark' ? theme.palette.success.light : theme.palette.success.dark;

  return (
    <Dialog
      open
      onClose={handleShowTerminal}
      maxWidth="md"
      fullWidth
      PaperComponent={Paper}
      PaperProps={{ elevation: 0, sx: { borderRadius: 3, overflow: 'hidden' } }}
    >
      <NymCard
        title={
          <Box width="100%" display="flex" justifyContent="space-between" alignItems="center">
            <Stack direction="row" alignItems="center" spacing={1}>
              <TerminalIcon sx={{ color: 'primary.main' }} />
              <Typography component="span" fontWeight={600}>
                Terminal
              </Typography>
              {!isBusy && (
                <IconButton size="small" onClick={refresh} aria-label="Refresh state">
                  <RefreshIcon fontSize="small" />
                </IconButton>
              )}
            </Stack>
            <IconButton size="small" onClick={handleShowTerminal} aria-label="Close">
              <CloseIcon fontSize="small" />
            </IconButton>
          </Box>
        }
        dataTestid="terminal-page"
        noPadding
      >
        <Box sx={{ px: 3, pb: 3, pt: 0 }}>
          <Typography variant="h6" fontWeight={700} sx={{ mb: 1.5 }}>
            State viewer
          </Typography>

          {error && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {String(error)}
            </Alert>
          )}

          {status ? (
            <Alert severity="info" icon={<RefreshIcon fontSize="inherit" />} sx={{ mb: 2, alignItems: 'center' }}>
              <Typography variant="body2" fontWeight={600}>
                {status}
              </Typography>
            </Alert>
          ) : (
            <Alert
              severity="success"
              variant="outlined"
              sx={{
                mb: 2,
                bgcolor: successBg,
                color: successColor,
                borderColor: alpha(theme.palette.success.main, 0.45),
                '& .MuiAlert-icon': { color: successColor },
              }}
            >
              <Typography variant="body2" fontWeight={600}>
                Data loading complete
              </Typography>
            </Alert>
          )}

          <TerminalSection heading="App environment">{JSON.stringify(appEnv, null, 2)}</TerminalSection>

          <TerminalSection heading="Client details">{JSON.stringify(clientDetails, null, 2)}</TerminalSection>

          <TerminalSection heading="User balance">{JSON.stringify(userBalance, null, 2)}</TerminalSection>

          <TerminalSection heading="Balance (useGetBalance hook)">
            {JSON.stringify(userBalance.balance, null, 2)}
          </TerminalSection>

          <TerminalSection heading="Vesting account info (useGetBalance hook)">
            {JSON.stringify(userBalance.vestingAccountInfo, null, 2)}
          </TerminalSection>

          <TerminalSection heading="Current vest period (useGetBalance hook)">
            {JSON.stringify(userBalance.currentVestingPeriod, null, 2)}
          </TerminalSection>

          <TerminalSection heading="Original vesting">
            {JSON.stringify(userBalance.originalVesting, null, 2)}
          </TerminalSection>

          <TerminalSection heading="Mixnode delegations">{JSON.stringify(mixnodeDelegations, null, 2)}</TerminalSection>

          <TerminalSection heading="Pending delegation events">
            {JSON.stringify(pendingEvents, null, 2)}
          </TerminalSection>

          <TerminalSection heading="Pending vesting delegation events">
            {JSON.stringify(pendingVestingEvents, null, 2)}
          </TerminalSection>

          <TerminalSection heading="Epoch">{JSON.stringify(epoch, null, 2)}</TerminalSection>
        </Box>
      </NymCard>
    </Dialog>
  );
};

export const Terminal: FCWithChildren = () => {
  const { showTerminal } = useContext(AppContext);

  if (!showTerminal) {
    return null;
  }

  return <TerminalInner />;
};

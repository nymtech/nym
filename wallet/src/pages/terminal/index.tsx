import React, { useContext, useState } from 'react';
import { Alert, Box, Paper, Dialog, Typography } from '@mui/material';
import TerminalIcon from '@mui/icons-material/Terminal';
import RefreshIcon from '@mui/icons-material/Refresh';
import CloseIcon from '@mui/icons-material/Close';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import ArrowDropUpIcon from '@mui/icons-material/ArrowDropUp';
import { Console } from '@src/utils/console';
import { AppContext } from '../../context/main';
import { NymCard } from '../../components';
import { getCurrentInterval, getAllPendingDelegations, getMixNodeDelegationsForCurrentAccount } from '../../requests';

const TerminalSection: FCWithChildren<{
  heading: React.ReactNode;
  children: React.ReactNode;
}> = ({ heading, children }) => {
  const [isCollapsed, setIsCollapsed] = useState<boolean>(true);

  const toggleCollapse = () => {
    setIsCollapsed((prev) => !prev);
  };

  return (
    <Box mb={2}>
      <Typography display="flex">
        {isCollapsed ? <ArrowDropDownIcon onClick={toggleCollapse} /> : <ArrowDropUpIcon onClick={toggleCollapse} />}
        <Typography ml={2} fontWeight={600}>
          {heading}
        </Typography>
      </Typography>
      {!isCollapsed && <Paper sx={{ px: 3, py: 0.5 }}>{children}</Paper>}
    </Box>
  );
};

const TerminalInner: FCWithChildren = () => {
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

  return (
    <Dialog
      open
      onClose={handleShowTerminal}
      maxWidth="md"
      fullWidth
      PaperComponent={Paper}
      PaperProps={{ elevation: 0 }}
    >
      <NymCard
        title={
          <Box width="100%" display="flex" justifyContent="space-between">
            <Box display="flex" alignItems="center">
              <TerminalIcon sx={{ mr: 1 }} />
              <Typography mr={4}>Terminal</Typography>
              {!isBusy && <RefreshIcon onClick={refresh} cursor="pointer" />}
            </Box>
            <CloseIcon onClick={handleShowTerminal} cursor="pointer" />
          </Box>
        }
        dataTestid="terminal-page"
      >
        <h2>State Viewer</h2>

        {error && <Alert color="error">{error}</Alert>}

        {status ? (
          <Alert color="info" icon={<RefreshIcon />} sx={{ mb: 2 }}>
            <Box sx={{ fontWeight: 600 }}>{status}</Box>
          </Alert>
        ) : (
          <Alert color="success" sx={{ mb: 2 }}>
            <Box sx={{ fontWeight: 600 }}>Data loading complete</Box>
          </Alert>
        )}

        <TerminalSection heading="App Environment">
          <pre>{JSON.stringify(appEnv, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Client Details">
          <pre>{JSON.stringify(clientDetails, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="User Balance">
          <pre>{JSON.stringify(userBalance, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection
          heading={
            <>
              <code>useGetBalance</code> Balance
            </>
          }
        >
          <pre>{JSON.stringify(userBalance.balance, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection
          heading={
            <>
              <code>useGetBalance</code> Vesting Account Info
            </>
          }
        >
          <pre>{JSON.stringify(userBalance.vestingAccountInfo, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection
          heading={
            <>
              <code>useGetBalance</code> Current Vest Period
            </>
          }
        >
          <pre>{JSON.stringify(userBalance.currentVestingPeriod, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Original Vesting">
          <pre>{JSON.stringify(userBalance.originalVesting, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Mixnode Delegations">
          <pre>{JSON.stringify(mixnodeDelegations, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Pending Delegation Events">
          <pre>{JSON.stringify(pendingEvents, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Pending Vesting Delegation Events">
          <pre>{JSON.stringify(pendingVestingEvents, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Epoch">
          <pre>{JSON.stringify(epoch, null, 2)}</pre>
        </TerminalSection>
      </NymCard>
    </Dialog>
  );
};

export const Terminal: FCWithChildren = () => {
  const { showTerminal } = useContext(AppContext);

  // this is a guard component, that only mounts the terminal component when shown
  if (!showTerminal) {
    return null;
  }

  return <TerminalInner />;
};

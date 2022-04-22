import React, { useContext, useState } from 'react';
import { Alert, Box, Dialog, Paper, Typography } from '@mui/material';
import TerminalIcon from '@mui/icons-material/Terminal';
import RefreshIcon from '@mui/icons-material/Refresh';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import ArrowDropUpIcon from '@mui/icons-material/ArrowDropUp';
import { ClientContext } from '../../context/main';
import { NymCard } from '../../components';
import { getCurrentEpoch, getPendingDelegations, getReverseMixDelegations } from '../../requests';
import { useGetBalance } from '../../hooks/useGetBalance';

const TerminalSection: React.FC<{
  heading: React.ReactNode;
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

export const Terminal: React.FC = () => {
  const { userBalance, clientDetails, showTerminal, handleShowTerminal } = useContext(ClientContext);
  const { balance, vestingAccountInfo, currentVestingPeriod, originalVesting, fetchBalance, fetchTokenAllocation } =
    useGetBalance(clientDetails?.client_address);
  const [mixnodeDelegations, setMixnodeDelegations] = useState<any>();
  const [pendingEvents, setPendingEvents] = useState<any>();
  const [epoch, setEpoch] = useState<any>();
  const [isBusy, setIsBusy] = useState<boolean>();
  const [error, setError] = useState<any>();
  const [status, setStatus] = useState<string | undefined>();

  const withErrorCatch = async (fn: () => Promise<void>) => {
    try {
      await fn();
    } catch (e) {
      setError(e);
    }
  };

  const refresh = async () => {
    setError(undefined);
    setIsBusy(true);
    setStatus('Getting reverse mix delegations...');
    await withErrorCatch(async () => {
      setMixnodeDelegations(await getReverseMixDelegations());
    });
    setStatus('Getting pending delegations...');
    await withErrorCatch(async () => {
      setPendingEvents(await getPendingDelegations());
    });
    setStatus('Getting current epoch...');
    await withErrorCatch(async () => {
      setEpoch(await getCurrentEpoch());
    });
    setStatus('Fetching balance...');
    await withErrorCatch(async () => {
      await fetchBalance();
    });
    setStatus('Fetching token allocation...');
    await withErrorCatch(async () => {
      await fetchTokenAllocation();
    });
    setStatus(undefined);
    setIsBusy(false);
  };

  React.useEffect(() => {
    refresh();
  }, []);

  if (!showTerminal) {
    return null;
  }

  return (
    <Dialog open onClose={handleShowTerminal} maxWidth="md" fullWidth>
      <NymCard
        title={
          <Box display="flex" alignItems="center">
            <TerminalIcon sx={{ mr: 1 }} />
            <Typography mr={4}>Terminal</Typography>
            {!isBusy && <RefreshIcon onClick={refresh} />}
          </Box>
        }
      >
        {error && <Alert color="error">{error}</Alert>}

        {status ? (
          <Alert color="info" sx={{ mb: 2 }}>
            <strong>{status}</strong>
          </Alert>
        ) : (
          <Alert color="success" sx={{ mb: 2 }}>
            <strong>Data loading complete</strong>
          </Alert>
        )}

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
          <pre>{JSON.stringify(balance, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection
          heading={
            <>
              <code>useGetBalance</code> Vesting Account Info
            </>
          }
        >
          <pre>{JSON.stringify(vestingAccountInfo, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection
          heading={
            <>
              <code>useGetBalance</code> Current Vest Period
            </>
          }
        >
          <pre>{JSON.stringify(currentVestingPeriod, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Original Vesting">
          <pre>{JSON.stringify(originalVesting, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Mixnode Delegations">
          <pre>{JSON.stringify(mixnodeDelegations, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Pending Delegation Events">
          <pre>{JSON.stringify(pendingEvents, null, 2)}</pre>
        </TerminalSection>

        <TerminalSection heading="Epoch">
          <pre>{JSON.stringify(epoch, null, 2)}</pre>
        </TerminalSection>
      </NymCard>
    </Dialog>
  );
};

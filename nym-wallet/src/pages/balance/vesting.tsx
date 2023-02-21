import React from 'react';
import { Refresh } from '@mui/icons-material';
import {
  Box,
  Button,
  IconButton,
  Table,
  TableBody,
  TableCell,
  TableCellProps,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { useSnackbar } from 'notistack';
import { useContext, useEffect, useState } from 'react';
import { NymCard } from 'src/components';
import { AppContext } from 'src/context/main';
import { Period } from 'src/types';
import { VestingTimeline } from './components/vesting-timeline';

const columnsHeaders: Array<{ title: string; align: TableCellProps['align'] }> = [
  { title: 'Locked', align: 'left' },
  { title: 'Period', align: 'left' },
  { title: 'Percentage Vested', align: 'left' },
  { title: 'Unlocked', align: 'right' },
];

const vestingPeriod = (current?: Period, original?: number) => {
  if (current === 'After') return 'Complete';

  if (typeof current === 'object' && typeof original === 'number') return `${current.In + 1}/${original}`;

  return 'N/A';
};

const VestingSchedule = () => {
  const { userBalance, clientDetails } = useContext(AppContext);
  const [vestedPercentage, setVestedPercentage] = useState(0);

  const calculatePercentage = () => {
    const { tokenAllocation, originalVesting } = userBalance;
    if (tokenAllocation?.vesting && tokenAllocation.vested && tokenAllocation.vested !== '0' && originalVesting) {
      const percentage = (+tokenAllocation.vested / +originalVesting.amount.amount) * 100;
      const rounded = percentage.toFixed(2);
      setVestedPercentage(+rounded);
    } else {
      setVestedPercentage(0);
    }
  };

  useEffect(() => {
    calculatePercentage();
  }, [userBalance.tokenAllocation, calculatePercentage]);

  return (
    <TableContainer sx={{ py: 1 }}>
      <Table>
        <TableHead>
          <TableRow>
            {columnsHeaders.map((header) => (
              <TableCell key={header.title} sx={{ color: (t) => t.palette.nym.text.muted }} align={header.align}>
                {header.title}
              </TableCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          <TableRow>
            <TableCell
              sx={{
                color: 'text.primary',
                borderBottom: 'none',
                textTransform: 'uppercase',
              }}
            >
              {userBalance.tokenAllocation?.vesting || 'n/a'} / {userBalance.originalVesting?.amount.amount}{' '}
              {clientDetails?.display_mix_denom.toUpperCase()}
            </TableCell>
            <TableCell
              align="left"
              sx={{
                color: 'text.primary',
                borderBottom: 'none',
              }}
            >
              {vestingPeriod(userBalance.currentVestingPeriod, userBalance.originalVesting?.number_of_periods)}
            </TableCell>
            <TableCell
              sx={{
                color: 'text.primary',
                borderBottom: 'none',
              }}
            >
              <Box display="flex" alignItems="center" gap={1}>
                <Typography variant="body2">{`${vestedPercentage}%`}</Typography>
                <VestingTimeline percentageComplete={vestedPercentage} />
              </Box>
            </TableCell>
            <TableCell
              sx={{
                color: 'text.primary',
                borderBottom: 'none',
                textTransform: 'uppercase',
              }}
              align="right"
            >
              {userBalance.tokenAllocation?.vested || 'n/a'} / {userBalance.originalVesting?.amount.amount}{' '}
              {clientDetails?.display_mix_denom.toUpperCase()}
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </TableContainer>
  );
};

const TokenTransfer = () => {
  const { userBalance, clientDetails } = useContext(AppContext);

  return (
    <Box sx={{ my: 3 }}>
      <Typography variant="subtitle2" sx={{ mb: 3, fontWeight: '600' }}>
        Unlocked transferable tokens
      </Typography>

      <Typography
        data-testid="refresh-success"
        sx={{ color: 'text.primary', fontWeight: '600', fontSize: 28 }}
        variant="h5"
        textTransform="uppercase"
      >
        {userBalance.tokenAllocation?.spendable || 'n/a'} {clientDetails?.display_mix_denom.toUpperCase()}
      </Typography>
    </Box>
  );
};

export const VestingCard = ({ onTransfer }: { onTransfer: () => Promise<void> }) => {
  const { userBalance } = useContext(AppContext);
  const { enqueueSnackbar, closeSnackbar } = useSnackbar();

  const refreshBalances = async () => {
    await userBalance.fetchBalance();
    await userBalance.fetchTokenAllocation();
  };

  useEffect(() => {
    closeSnackbar();
    userBalance.fetchTokenAllocation();
  }, []);

  if (!userBalance.originalVesting) return null;

  return (
    <NymCard
      title="Vesting Schedule"
      subheader={
        <Typography variant="caption" sx={{ color: 'nym.text.muted' }}>
          You can use up to 10% of your locked tokens for bonding and delegating
        </Typography>
      }
      borderless
      data-testid="check-unvested-tokens"
      Action={
        <IconButton
          onClick={async () => {
            await refreshBalances();
            enqueueSnackbar('Balances updated', { variant: 'success', preventDuplicate: true });
          }}
        >
          <Refresh />
        </IconButton>
      }
    >
      <VestingSchedule />
      <TokenTransfer />
      <Box display="flex" justifyContent="end" alignItems="center">
        <Button size="large" variant="contained" onClick={onTransfer} disableElevation>
          Transfer
        </Button>
      </Box>
    </NymCard>
  );
};

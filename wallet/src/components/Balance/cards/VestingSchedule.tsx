import { useContext, useState, useEffect } from 'react';
import {
  TableContainer,
  Table,
  TableHead,
  TableRow,
  TableCell,
  TableBody,
  Typography,
  TableCellProps,
  Card,
} from '@mui/material';
import { Period } from '@nymproject/types';
import { AppContext } from '@src/context';
import { VestingTimeline } from '../VestingTimeline';

const columnsHeaders: Array<{ title: string; align: TableCellProps['align'] }> = [
  { title: 'Locked', align: 'left' },
  { title: 'Period', align: 'left' },
  { title: 'Unlocked', align: 'right' },
];

const vestingPeriod = (current?: Period, original?: number) => {
  if (current === 'After') return 'Complete';

  if (typeof current === 'object' && typeof original === 'number') return `${current.in + 1}/${original}`;

  return 'N/A';
};

export const VestingSchedule = () => {
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
    <Card variant="outlined" sx={{ p: 3, height: '100%' }}>
      <TableContainer sx={{ mb: 2 }}>
        <Table>
          <TableHead>
            <TableRow>
              {columnsHeaders.map((header) => (
                <TableCell
                  key={header.title}
                  sx={{ color: 'nym.text.muted', pt: 0, border: 'none', pb: 0 }}
                  align={header.align}
                >
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
      <Typography variant="body2" sx={{ color: 'nym.text.muted', mb: 3 }}>
        Percentage
      </Typography>
      <VestingTimeline percentageComplete={vestedPercentage} />
    </Card>
  );
};

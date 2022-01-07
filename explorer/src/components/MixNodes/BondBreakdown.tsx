import * as React from 'react';
import { Alert, Box, CircularProgress, useMediaQuery } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { ExpandMore } from '@mui/icons-material';
import { currencyToString } from '../../utils/currency';
import { useMixnodeContext } from '../../context/mixnode';

export const BondBreakdownTable: React.FC = () => {
  const { mixNode, delegations } = useMixnodeContext();
  const [showDelegations, toggleShowDelegations] =
    React.useState<boolean>(false);

  const [bonds, setBonds] = React.useState({
    delegations: '0',
    pledges: '0',
    bondsTotal: '0',
    hasLoaded: false,
  });
  const theme = useTheme();
  const matches = useMediaQuery(theme.breakpoints.down('sm'));

  React.useEffect(() => {
    if (mixNode?.data) {
      // delegations
      const decimalisedDelegations = currencyToString(
        mixNode.data.total_delegation.amount.toString(),
        mixNode.data.total_delegation.denom,
      );

      // pledges
      const decimalisedPledges = currencyToString(
        mixNode.data.pledge_amount.amount.toString(),
        mixNode.data.pledge_amount.denom,
      );

      // bonds total (del + pledges)
      const pledgesSum = Number(mixNode.data.pledge_amount.amount);
      const delegationsSum = Number(mixNode.data.total_delegation.amount);
      const bondsTotal = currencyToString(
        (delegationsSum + pledgesSum).toString(),
      );

      setBonds({
        delegations: decimalisedDelegations,
        pledges: decimalisedPledges,
        bondsTotal,
        hasLoaded: true,
      });
    }
  }, [mixNode]);

  const expandDelegations = () => {
    if (delegations?.data && delegations.data.length > 0) {
      toggleShowDelegations(!showDelegations);
    }
  };
  const calcBondPercentage = (num: number) => {
    if (mixNode?.data) {
      const rawDelegationAmount = Number(mixNode.data.total_delegation.amount);
      const rawPledgeAmount = Number(mixNode.data.pledge_amount.amount);
      const rawTotalBondsAmount = rawDelegationAmount + rawPledgeAmount;
      return ((num * 100) / rawTotalBondsAmount).toFixed(1);
    }
    return 0;
  };

  if (mixNode?.isLoading || delegations?.isLoading) {
    return <CircularProgress />;
  }

  if (mixNode?.error) {
    return <Alert severity="error">Mixnode not found</Alert>;
  }
  if (delegations?.error) {
    return (
      <Alert severity="error">Unable to get delegations for mixnode</Alert>
    );
  }

  return (
    <>
      <TableContainer component={Paper}>
        <Table sx={{ minWidth: 650 }} aria-label="bond breakdown totals">
          <TableBody>
            <TableRow sx={matches ? { minWidth: '70vw' } : null}>
              <TableCell
                sx={{
                  fontWeight: 'bold',
                  width: '150px',
                }}
                align="left"
              >
                Bond total
              </TableCell>
              <TableCell align="left" data-testid="bond-total-amount">
                {bonds.bondsTotal}
              </TableCell>
            </TableRow>
            <TableRow>
              <TableCell align="left">Pledge total</TableCell>
              <TableCell align="left" data-testid="pledge-total-amount">
                {bonds.pledges}
              </TableCell>
            </TableRow>
            <TableRow>
              <TableCell onClick={expandDelegations} align="left">
                <Box
                  sx={{
                    display: 'flex',
                    alignItems: 'center',
                  }}
                >
                  Delegation total {'\u00A0'}
                  {delegations?.data && delegations?.data?.length > 0 && (
                    <ExpandMore />
                  )}
                </Box>
              </TableCell>
              <TableCell align="left" data-testid="delegation-total-amount">
                {bonds.delegations}
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>

        {showDelegations && (
          <Box
            sx={{
              maxHeight: 400,
              overflowY: 'scroll',
            }}
          >
            <Table stickyHeader>
              <TableHead>
                <TableRow>
                  <TableCell
                    sx={{ fontWeight: 'bold', background: '#242C3D' }}
                    align="left"
                  >
                    Delegators
                  </TableCell>
                  <TableCell
                    sx={{ fontWeight: 'bold', background: '#242C3D' }}
                    align="left"
                  >
                    Stake
                  </TableCell>
                  <TableCell
                    sx={{
                      fontWeight: 'bold',
                      background: '#242C3D',
                      width: '200px',
                    }}
                    align="left"
                  >
                    Share from bond
                  </TableCell>
                </TableRow>
              </TableHead>

              <TableBody>
                {delegations?.data?.map(
                  ({ owner, amount: { amount, denom } }) => (
                    <TableRow key={owner}>
                      <TableCell
                        sx={matches ? { width: 190 } : null}
                        align="left"
                      >
                        {owner}
                      </TableCell>
                      <TableCell align="left">
                        {currencyToString(amount.toString(), denom)}
                      </TableCell>
                      <TableCell align="left">
                        {calcBondPercentage(amount)}%
                      </TableCell>
                    </TableRow>
                  ),
                )}
              </TableBody>
            </Table>
          </Box>
        )}
      </TableContainer>
    </>
  );
};

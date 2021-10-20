import * as React from 'react';
import { printableCoin } from '@nymproject/nym-validator-client';
import {
  Alert,
  CircularProgress,
  useMediaQuery,
  useTheme,
} from '@mui/material';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import { MainContext } from 'src/context/main';

export function BondBreakdownTable() {
  const { mixnodeDetailInfo, delegations, mode } =
    React.useContext(MainContext);
  const [allContentLoaded, setAllContentLoaded] =
    React.useState<boolean>(false);
  const [showError, setShowError] = React.useState<boolean>(false);

  const [bonds, setBonds] = React.useState({
    delegations: '0',
    pledges: '0',
    bondsTotal: '0',
    hasLoaded: false,
  });
  const theme = useTheme();
  const matches = useMediaQuery(theme.breakpoints.down('sm'));

  React.useEffect(() => {
    if (mixnodeDetailInfo && mixnodeDetailInfo.data?.length) {
      const thisMixnode = mixnodeDetailInfo?.data[0];

      // delegations
      const decimalisedDelegations = printableCoin({
        amount: thisMixnode.total_delegation.amount.toString(),
        denom: thisMixnode.total_delegation.denom,
      });

      // pledges
      const decimalisedPledges = printableCoin({
        amount: thisMixnode.bond_amount.amount.toString(),
        denom: thisMixnode.bond_amount.denom,
      });

      // bonds total (del + pledges)
      const pledgesSum = Number(thisMixnode.bond_amount.amount);
      const delegationsSum = Number(thisMixnode.total_delegation.amount);
      const bondsTotal = printableCoin({
        amount: (delegationsSum + pledgesSum).toString(),
        denom: 'upunk',
      });

      setBonds({
        delegations: decimalisedDelegations,
        pledges: decimalisedPledges,
        bondsTotal,
        hasLoaded: true,
      });
    }
  }, [mixnodeDetailInfo]);

  React.useEffect(() => {
    const hasError = Boolean(mixnodeDetailInfo?.error || delegations?.error);
    const hasAllMixnodeInfo = Boolean(
      mixnodeDetailInfo?.data !== undefined &&
        mixnodeDetailInfo?.data[0].mix_node,
    );
    const hasAllDelegationsInfo = Boolean(
      delegations?.data !== undefined && delegations?.data,
    );
    const hasAllData = Boolean(
      !hasError && hasAllMixnodeInfo && hasAllDelegationsInfo,
    );
    setShowError(hasError);
    setAllContentLoaded(hasAllData);
  }, [mixnodeDetailInfo, delegations]);

  const calcBondPercentage = (num: number) => {
    if (mixnodeDetailInfo?.data !== undefined && mixnodeDetailInfo?.data[0]) {
      const rawDeligationAmount = Number(
        mixnodeDetailInfo.data[0].total_delegation.amount,
      );
      const rawPledgeAmount = Number(
        mixnodeDetailInfo.data[0].bond_amount.amount,
      );
      const rawTotalBondsAmount = rawDeligationAmount + rawPledgeAmount;
      return ((num * 100) / rawTotalBondsAmount).toFixed(1);
    }
    return 0;
  };

  if (mixnodeDetailInfo?.isLoading) {
    return <CircularProgress />;
  }

  if (showError) {
    return (
      <Alert severity="warning">
        We are unable to retrieve a Mixnode with that ID. Please try later or
        Contact Us.
      </Alert>
    );
  }

  if (!showError && allContentLoaded) {
    return (
      <>
        <TableContainer
          component={Paper}
          sx={{
            background:
              mode === 'dark'
                ? theme.palette.secondary.dark
                : theme.palette.primary.light,
          }}
        >
          <Table sx={{ minWidth: 650 }} aria-label="bond breakdown totals">
            <TableBody>
              <TableRow sx={matches ? { minWidth: '70vw' } : null}>
                <TableCell
                  sx={{
                    fontWeight: 'bold',
                    width: matches ? '90px' : 'auto',
                  }}
                  align="left"
                >
                  Bond total
                </TableCell>
                <TableCell align="left">{bonds.bondsTotal}</TableCell>
              </TableRow>
              <TableRow>
                <TableCell
                  sx={{
                    width: matches ? '90px' : 'auto',
                  }}
                  align="left"
                >
                  Pledge total
                </TableCell>
                <TableCell align="left">{bonds.pledges}</TableCell>
              </TableRow>
              <TableRow>
                <TableCell
                  sx={{
                    width: matches ? '90px' : 'auto',
                  }}
                  align="left"
                >
                  Delegation total
                </TableCell>
                <TableCell align="left">{bonds.delegations}</TableCell>
              </TableRow>
            </TableBody>
          </Table>

          {delegations?.data !== undefined && delegations?.data[0] && (
            <Table sx={{ minWidth: 650 }} aria-label="delegation totals">
              <TableHead>
                <TableRow>
                  <TableCell sx={{ fontWeight: 'bold' }} align="left">
                    Delegators
                  </TableCell>
                  <TableCell sx={{ fontWeight: 'bold' }} align="left">
                    Stake
                  </TableCell>
                  <TableCell sx={{ fontWeight: 'bold' }} align="left">
                    % of Bond
                  </TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {delegations.data.map(
                  ({ owner, amount: { amount, denom } }) => (
                    <TableRow key={owner}>
                      <TableCell
                        sx={matches ? { width: 190 } : null}
                        align="left"
                      >
                        {owner}
                      </TableCell>
                      <TableCell align="left">
                        {printableCoin({ amount: amount.toString(), denom })}
                      </TableCell>
                      <TableCell align="left">
                        {calcBondPercentage(amount)}%
                      </TableCell>
                    </TableRow>
                  ),
                )}
              </TableBody>
            </Table>
          )}
        </TableContainer>
      </>
    );
  }
  return null;
}

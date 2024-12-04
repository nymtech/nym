import * as React from 'react'
import { Alert, Box, CircularProgress, Typography } from '@mui/material'
import { useTheme } from '@mui/material/styles'
import Table from '@mui/material/Table'
import TableBody from '@mui/material/TableBody'
import TableCell from '@mui/material/TableCell'
import TableContainer from '@mui/material/TableContainer'
import TableHead from '@mui/material/TableHead'
import TableRow from '@mui/material/TableRow'
import Paper from '@mui/material/Paper'
import { ExpandMore } from '@mui/icons-material'
import { currencyToString } from '@/app/utils/currency'
import { useMixnodeContext } from '@/app/context/mixnode'
import { useIsMobile } from '@/app/hooks/useIsMobile'

export const BondBreakdownTable: FCWithChildren = () => {
  const { mixNode, delegations, uniqDelegations } = useMixnodeContext()
  const [showDelegations, toggleShowDelegations] =
    React.useState<boolean>(false)

  const [bonds, setBonds] = React.useState({
    delegations: '0',
    pledges: '0',
    bondsTotal: '0',
    hasLoaded: false,
  })
  const theme = useTheme()
  const isMobile = useIsMobile()

  React.useEffect(() => {
    if (mixNode?.data) {
      // delegations
      const decimalisedDelegations = currencyToString({
        amount: mixNode.data.total_delegation.amount.toString(),
        denom: mixNode.data.total_delegation.denom,
      })

      // pledges
      const decimalisedPledges = currencyToString({
        amount: mixNode.data.pledge_amount.amount.toString(),
        denom: mixNode.data.pledge_amount.denom,
      })

      // bonds total (del + pledges)
      const pledgesSum = Number(mixNode.data.pledge_amount.amount)
      const delegationsSum = Number(mixNode.data.total_delegation.amount)
      const bondsTotal = currencyToString({
        amount: (pledgesSum + delegationsSum).toString(),
      })

      setBonds({
        delegations: decimalisedDelegations,
        pledges: decimalisedPledges,
        bondsTotal,
        hasLoaded: true,
      })
    }
  }, [mixNode])

  const expandDelegations = () => {
    if (delegations?.data && delegations.data.length > 0) {
      toggleShowDelegations(!showDelegations)
    }
  }
  const calcBondPercentage = (num: number) => {
    if (mixNode?.data) {
      const rawDelegationAmount = Number(mixNode.data.total_delegation.amount)
      const rawPledgeAmount = Number(mixNode.data.pledge_amount.amount)
      const rawTotalBondsAmount = rawDelegationAmount + rawPledgeAmount
      return ((num * 100) / rawTotalBondsAmount).toFixed(1)
    }
    return 0
  }

  if (mixNode?.isLoading || delegations?.isLoading) {
    return <CircularProgress />
  }

  if (mixNode?.error) {
    return <Alert severity="error">Mixnode not found</Alert>
  }
  if (delegations?.error) {
    return <Alert severity="error">Unable to get delegations for mixnode</Alert>
  }

  return (
    <TableContainer component={Paper}>
      <Table sx={{ minWidth: 650 }} aria-label="bond breakdown totals">
        <TableBody>
          <TableRow sx={isMobile ? { minWidth: '70vw' } : null}>
            <TableCell
              sx={{
                fontWeight: 400,
                width: '150px',
              }}
              align="left"
            >
              Stake total
            </TableCell>
            <TableCell align="left" data-testid="bond-total-amount">
              {bonds.bondsTotal}
            </TableCell>
          </TableRow>
          <TableRow>
            <TableCell align="left">Bond</TableCell>
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
            p: 2,
            background: theme.palette.background.paper,
          }}
        >
          <Box
            sx={{
              display: 'flex',
              alignItems: 'baseline',
              width: '100%',
              p: 2,
              borderBottom: `1px solid ${theme.palette.divider}`,
            }}
            data-testid="delegations-total-amount"
          >
            <Typography
              sx={{
                fontSize: 16,
                fontWeight: 600,
              }}
            >
              Delegations&nbsp;&nbsp;
            </Typography>
          </Box>
          <Table stickyHeader>
            <TableHead>
              <TableRow>
                <TableCell
                  sx={{
                    fontWeight: 600,
                    background: theme.palette.background.paper,
                  }}
                  align="left"
                >
                  Delegators
                </TableCell>
                <TableCell
                  sx={{
                    fontWeight: 600,
                    background: theme.palette.background.paper,
                  }}
                  align="left"
                >
                  Amount
                </TableCell>
                <TableCell
                  sx={{
                    fontWeight: 600,
                    background: theme.palette.background.paper,
                    width: '200px',
                  }}
                  align="left"
                >
                  Share of stake
                </TableCell>
              </TableRow>
            </TableHead>

            <TableBody>
              {uniqDelegations?.data?.map(({ owner, amount: { amount } }) => (
                <TableRow key={owner}>
                  <TableCell sx={isMobile ? { width: 190 } : null} align="left">
                    {owner}
                  </TableCell>
                  <TableCell align="left">
                    {currencyToString({ amount: amount.toString() })}
                  </TableCell>
                  <TableCell align="left">
                    {calcBondPercentage(amount)}%
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Box>
      )}
    </TableContainer>
  )
}

import React, { useEffect, useContext, useState } from 'react'
import {
  IconButton,
  CircularProgress,
  LinearProgress,
  Table,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
  Box,
  Button,
  TableCellProps,
  Grid,
} from '@mui/material'
import { InfoOutlined, Refresh } from '@mui/icons-material'
import { useSnackbar } from 'notistack'
import { NymCard, InfoTooltip, Title, Fee } from '../../components'
import { ClientContext } from '../../context/main'
import { withdrawVestedCoins } from '../../requests'
import { Period } from '../../types'

export const VestingCard = () => {
  const { userBalance } = useContext(ClientContext)
  const [isLoading, setIsLoading] = useState(false)

  const { enqueueSnackbar, closeSnackbar } = useSnackbar()

  const refreshBalances = async () => {
    await userBalance.fetchBalance()
    await userBalance.fetchTokenAllocation()
  }

  useEffect(() => {
    return () => closeSnackbar()
  }, [])

  return (
    <NymCard
      title="Vesting Schedule"
      data-testid="check-unvested-tokens"
      Icon={InfoOutlined}
      Action={
        <IconButton
          onClick={async () => {
            await refreshBalances()
            enqueueSnackbar('Balances updated', { variant: 'success', preventDuplicate: true })
          }}
        >
          <Refresh />
        </IconButton>
      }
    >
      <VestingSchedule />
      <TokenTransfer />
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Fee feeType="Send" />
        <Button
          size="large"
          variant="contained"
          onClick={async () => {
            setIsLoading(true)
            try {
              await withdrawVestedCoins(userBalance.tokenAllocation?.spendable!)
              await refreshBalances()
              enqueueSnackbar('Token transfer succeeded', {
                variant: 'success',
                preventDuplicate: true,
              })
            } catch (e) {
              console.log(e)
              enqueueSnackbar('Token transfer failed. You may not have any transferable tokens at this time', {
                variant: 'error',
                preventDuplicate: true,
              })
            } finally {
              setIsLoading(false)
            }
          }}
          endIcon={isLoading && <CircularProgress size={16} color="inherit" />}
          disabled={isLoading}
          disableElevation
        >
          Transfer
        </Button>
      </Box>
    </NymCard>
  )
}

const columnsHeaders: Array<{ title: string; align: TableCellProps['align'] }> = [
  { title: 'Locked', align: 'left' },
  { title: 'Period', align: 'left' },
  { title: 'Percentage Vested', align: 'left' },
  { title: 'Unlocked', align: 'right' },
]

const VestingSchedule = () => {
  const { userBalance, currency } = useContext(ClientContext)
  const [vestedPercentage, setVestedPercentage] = useState(0)

  const calculatePercentage = () => {
    const { tokenAllocation, originalVesting } = userBalance
    if (tokenAllocation?.vesting && tokenAllocation.vested && tokenAllocation.vested !== '0' && originalVesting) {
      const percentage = Math.round((+tokenAllocation.vested / +originalVesting?.amount.amount) * 100)
      setVestedPercentage(percentage)
    } else {
      setVestedPercentage(0)
    }
  }

  useEffect(() => {
    calculatePercentage()
  }, [userBalance.tokenAllocation, calculatePercentage])

  return (
    <TableContainer>
      <Table>
        <TableHead>
          <TableRow>
            {columnsHeaders.map((header) => (
              <TableCell key={header.title} sx={{ color: 'grey.500' }} align={header.align}>
                {header.title}
              </TableCell>
            ))}
          </TableRow>
          <TableRow>
            <TableCell sx={{ borderBottom: 'none' }}>
              {userBalance.tokenAllocation?.vesting || 'n/a'} / {userBalance.originalVesting?.amount.amount}{' '}
              {currency?.major}
            </TableCell>
            <TableCell align="left" sx={{ borderBottom: 'none' }}>
              {vestingPeriod(userBalance.currentVestingPeriod, userBalance.originalVesting?.number_of_periods)}
            </TableCell>
            <TableCell sx={{ borderBottom: 'none' }}>
              <Box display="flex" alignItems="center" gap={1}>
                <Typography variant="caption">{`${vestedPercentage}%`}</Typography>
                <LinearProgress
                  sx={{ flexBasis: '99%' }}
                  variant="determinate"
                  value={vestedPercentage}
                  color="inherit"
                />
              </Box>
            </TableCell>
            <TableCell sx={{ borderBottom: 'none' }} align="right">
              {userBalance.tokenAllocation?.vested || 'n/a'} / {userBalance.originalVesting?.amount.amount}{' '}
              {currency?.major}
            </TableCell>
          </TableRow>
        </TableHead>
      </Table>
    </TableContainer>
  )
}

const vestingPeriod = (current?: Period, original?: number) => {
  if (current === 'After') return 'Complete'

  if (typeof current === 'object' && typeof original === 'number') return `${current.In + 1}/${original}`

  return 'N/A'
}

const TokenTransfer = () => {
  const { userBalance, currency } = useContext(ClientContext)
  return (
    <Grid container sx={{ my: 2 }} direction="column" spacing={2}>
      <Grid item>
        <Title
          title="Transfer unlocked tokens"
          Icon={() => {
            return (
              <Box sx={{ display: 'flex', mr: 1 }}>
                <InfoTooltip title="Unlocked tokens that are available to transfer to your balance" size="medium" />
              </Box>
            )
          }}
        />
      </Grid>
      <Grid item>
        <Typography variant="subtitle2" sx={{ color: 'grey.500', mt: 2 }}>
          Transferable tokens
        </Typography>

        <Typography data-testid="refresh-success" sx={{ color: 'nym.background.dark' }} variant="h5" fontWeight="700">
          {userBalance.tokenAllocation?.spendable || 'n/a'} {currency?.major}
        </Typography>
      </Grid>
    </Grid>
  )
}

import React, { useEffect, useContext, useState } from 'react'
import {
  IconButton,
  CircularProgress,
  Grid,
  LinearProgress,
  Table,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
  Box,
  Button,
} from '@mui/material'
import { InfoOutlined, Refresh } from '@mui/icons-material'
import { useSnackbar } from 'notistack'
import { NymCard, InfoTooltip } from '../../components'
import { ClientContext } from '../../context/main'
import { withdrawVestedCoins } from '../../requests'
import { Period } from '../../types'

export const VestingCard = () => {
  const { userBalance, currency } = useContext(ClientContext)
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
      <Grid container direction="column" spacing={3}>
        <Grid item container spacing={3}>
          <Grid item>
            <Typography variant="subtitle2" sx={{ color: 'grey.500', ml: 2, mb: 1 }}>
              Unlocked tokens
            </Typography>
            <Typography
              data-testid="refresh-success"
              sx={{ ml: 2, color: 'nym.background.dark' }}
              variant="h5"
              fontWeight="700"
            >
              {userBalance.tokenAllocation?.vested || 'n/a'} {currency?.major}
            </Typography>
          </Grid>
          <Grid item>
            <Box display="flex" alignItems="center" justifyContent="center" sx={{ mb: 1 }}>
              <Typography variant="subtitle2" sx={{ color: 'grey.500', mr: 0.5 }}>
                Transferable tokens
              </Typography>
              <InfoTooltip title="Unlocked tokens that are available to transfer to your balance" light />
            </Box>
            <Typography
              data-testid="refresh-success"
              sx={{ ml: 2, color: 'nym.background.dark' }}
              variant="h5"
              fontWeight="700"
            >
              {userBalance.tokenAllocation?.spendable || 'n/a'} {currency?.major}
            </Typography>
          </Grid>
        </Grid>
        <Grid item>
          <VestingTable />
        </Grid>
      </Grid>
      <Box display="flex" justifyContent="flex-end" alignItems="center" sx={{ mt: 2 }}>
        <Button
          size="large"
          variant="contained"
          onClick={async () => {
            setIsLoading(true)
            try {
              await withdrawVestedCoins(userBalance.tokenAllocation?.spendable!)
              await refreshBalances()
              enqueueSnackbar('Token release succeeded', {
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

const columnsHeaders = ['Locked', 'Period', 'Percentage Vested', 'Unlocked']
const VestingTable = () => {
  const { userBalance, currency } = useContext(ClientContext)
  const [vestedPercentage, setVestedPercentage] = useState(0)

  const calculatPercentage = () => {
    const { tokenAllocation, originalVesting } = userBalance
    if (tokenAllocation?.vesting && tokenAllocation.vested && tokenAllocation.vested !== '0' && originalVesting) {
      const percentage = Math.round((+tokenAllocation.vested / +originalVesting?.amount.amount) * 100)
      setVestedPercentage(percentage)
    } else {
      setVestedPercentage(0)
    }
  }

  useEffect(() => {
    calculatPercentage()
  }, [userBalance.tokenAllocation, calculatPercentage])

  return (
    <TableContainer>
      <Table>
        <TableHead>
          <TableRow>
            {columnsHeaders.map((header) => (
              <TableCell key={header} sx={{ color: 'grey.500' }}>
                {header}
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
                <Typography
                  variant="caption"
                  sx={{ color: 'nym.fee', fontWeight: 600 }}
                >{`${vestedPercentage}%`}</Typography>
                <LinearProgress
                  sx={{ flexBasis: '99%', color: 'nym.fee' }}
                  variant="determinate"
                  value={vestedPercentage}
                  color="inherit"
                />
              </Box>
            </TableCell>
            <TableCell sx={{ borderBottom: 'none' }}>
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

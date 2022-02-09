import React, { useEffect, useContext, useState } from 'react'
import {
  Alert,
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
import { InfoOutlined } from '@mui/icons-material'
import { NymCard } from '../../components'
import { ClientContext } from '../../context/main'
import { withdrawVestedCoins } from '../../requests'

export const VestingCard = () => {
  const { userBalance, currency } = useContext(ClientContext)
  const [isLoading, setIsLoading] = useState(false)
  return (
    <NymCard title="Unvested tokens" data-testid="check-unvested-tokens" Icon={InfoOutlined}>
      <Grid container direction="column" spacing={2}>
        <Grid item>
          {userBalance.error && (
            <Alert severity="error" data-testid="error-refresh" sx={{ p: 2 }}>
              {userBalance.error}
            </Alert>
          )}
          {!userBalance.error && (
            <>
              <Typography variant="subtitle2" sx={{ color: 'grey.500', ml: 2, mb: 1 }}>
                Amount of unvested tokens
              </Typography>
              <Typography
                data-testid="refresh-success"
                sx={{ ml: 2, color: 'nym.background.dark' }}
                variant="h5"
                fontWeight="700"
              >
                {userBalance.tokenAllocation?.vested || 'n/a'} {currency?.major}
              </Typography>
            </>
          )}
        </Grid>
        <Grid item>
          <VestingTable />
        </Grid>
      </Grid>
      <pre style={{ background: 'black', color: 'white', padding: 15 }}>
        {JSON.stringify(userBalance.tokenAllocation)}
      </pre>
      {userBalance.tokenAllocation?.spendable !== '0' && (
        <Box display="flex" justifyContent="flex-end" alignItems="center">
          <Button
            variant="contained"
            onClick={async () => {
              setIsLoading(true)
              try {
                await withdrawVestedCoins(userBalance.tokenAllocation?.spendable!)
                await userBalance.fetchBalance()
                await userBalance.fetchTokenAllocation()
              } catch (e) {
                console.log(e)
              } finally {
                setIsLoading(false)
              }
            }}
            endIcon={isLoading && <CircularProgress size={16} color="inherit" />}
            disabled={isLoading}
            disableElevation
          >
            Release Tokens
          </Button>
        </Box>
      )}
    </NymCard>
  )
}

const columnsHeaders = ['Vesting', 'Period', 'Amount', 'Vested']
const VestingTable = () => {
  const { userBalance, currency } = useContext(ClientContext)
  const [vestedPercentage, setVestedPercentage] = useState(0)

  const calculatPercentage = () => {
    const { tokenAllocation } = userBalance
    if (tokenAllocation?.vesting && tokenAllocation.vested && tokenAllocation.vested !== '0') {
      const percentage = Math.round((+tokenAllocation.vested / +tokenAllocation.original) * 100)
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
              {userBalance.tokenAllocation?.vesting || 'n/a'} {currency?.major}
            </TableCell>
            <TableCell sx={{ borderBottom: 'none' }}></TableCell>
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
              {userBalance.tokenAllocation?.vested || 'n/a'} {currency?.major}
            </TableCell>
          </TableRow>
        </TableHead>
      </Table>
    </TableContainer>
  )
}

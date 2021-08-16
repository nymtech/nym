import React, { useContext, useEffect } from 'react'
import { Grid, Button } from '@material-ui/core'
import RefreshIcon from '@material-ui/icons/Refresh'
import { useRouter } from 'next/router'
import { ValidatorClientContext } from '../contexts/ValidatorClient'
import MainNav from '../components/MainNav'
import Confirmation from '../components/Confirmation'
import NoClientError from '../components/NoClientError'
import { useGetBalance } from '../hooks/useGetBalance'
import { Layout, NymCard } from '../components'

export default function CheckBalance() {
  const router = useRouter()

  const { client } = useContext(ValidatorClientContext)
  const { getBalance, isBalanceLoading, balanceCheckError, printedBalance } =
    useGetBalance()

  useEffect(() => {
    const updateBalance = async () => {
      if (client === null) {
        await router.push('/')
      } else {
        await getBalance()
      }
    }
    updateBalance()
  }, [client])

  const balanceMessage = `Current account balance is ${printedBalance}`

  return (
    <>
      <MainNav />
      <Layout>
        <NymCard title="Check Balance">
          {client === null ? (
            <NoClientError />
          ) : (
            <Grid container direction="column" spacing={2}>
              <Grid item>
                <Confirmation
                  isLoading={isBalanceLoading}
                  error={balanceCheckError}
                  progressMessage="Checking balance..."
                  successMessage={balanceMessage}
                  failureMessage="Failed to check the account balance!"
                />
              </Grid>
              <Grid item>
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  <Button
                    variant="contained"
                    color="primary"
                    type="submit"
                    onClick={getBalance}
                    disabled={isBalanceLoading}
                    startIcon={<RefreshIcon />}
                  >
                    Refresh
                  </Button>
                </div>
              </Grid>
            </Grid>
          )}
        </NymCard>
      </Layout>
    </>
  )
}

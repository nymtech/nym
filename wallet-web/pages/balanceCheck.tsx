import React, { useContext, useEffect } from 'react'
import Button from '@material-ui/core/Button'
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
            <>
              <Confirmation
                isLoading={isBalanceLoading}
                error={balanceCheckError}
                progressMessage="Checking balance..."
                successMessage={balanceMessage}
                failureMessage="Failed to check the account balance!"
              />
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
            </>
          )}
        </NymCard>
      </Layout>
    </>
  )
}

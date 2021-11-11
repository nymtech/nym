import React, { useContext, useEffect, useState } from 'react'
import { NymCard } from '../../components'
import { UnbondForm } from './UnbondForm'
import { Layout } from '../../layouts'
import { useCheckOwnership } from '../../hooks/useCheckOwnership'
import { Alert } from '@material-ui/lab'
import { Box, Button, CircularProgress, Theme } from '@material-ui/core'
import { ClientContext } from '../../context/main'
import { unbond } from '../../requests'
import { useTheme } from '@material-ui/styles'

export const Unbond = () => {
  const [isLoading, setIsLoading] = useState(false)
  const { checkOwnership, ownership } = useCheckOwnership()
  const { getBalance } = useContext(ClientContext)

  const theme: Theme = useTheme()

  useEffect(() => {
    const initialiseForm = async () => {
      await checkOwnership()
    }
    initialiseForm()
  }, [ownership.hasOwnership, checkOwnership])

  return (
    <Layout>
      <NymCard title="Unbond" subheader="Unbond a mixnode or gateway" noPadding>
        {ownership?.hasOwnership && (
          <Alert
            severity="warning"
            data-testid="bond-noded"
            action={
              <Button
                data-testid="un-bond"
                disabled={isLoading}
                onClick={async () => {
                  setIsLoading(true)
                  await unbond(ownership.nodeType!)
                  getBalance.fetchBalance()
                  setIsLoading(false)
                }}
              >
                Unbond
              </Button>
            }
            style={{ margin: theme.spacing(2) }}
          >
            {`Looks like you already have a ${ownership.nodeType} bonded.`}
          </Alert>
        )}
        {!ownership.hasOwnership && (
          <Alert severity="info" style={{ margin: theme.spacing(3) }} data-testid="no-bond">
            You don't currently have a bonded node
          </Alert>
        )}
        {isLoading && (
          <Box
            style={{
              display: 'flex',
              justifyContent: 'center',
              padding: theme.spacing(3),
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
      </NymCard>
    </Layout>
  )
}

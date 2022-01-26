import React, { useContext, useEffect, useState } from 'react'
import { Alert, Box, Button, CircularProgress } from '@mui/material'
import { Fee, NymCard } from '../../components'
import { Layout } from '../../layouts'
import { useCheckOwnership } from '../../hooks/useCheckOwnership'
import { ClientContext } from '../../context/main'
import { unbond } from '../../requests'
import { Unbond as UnbondIcon } from '../../svg-icons'

export const Unbond = () => {
  const [isLoading, setIsLoading] = useState(false)
  const { checkOwnership, ownership } = useCheckOwnership()
  const { userBalance, getBondDetails } = useContext(ClientContext)

  useEffect(() => {
    const initialiseForm = async () => {
      await checkOwnership()
    }
    initialiseForm()
  }, [ownership.hasOwnership, checkOwnership])

  return (
    <Layout>
      <NymCard title="Unbond" subheader="Unbond a mixnode or gateway" noPadding Icon={UnbondIcon}>
        {ownership?.hasOwnership ? (
          <>
            <Alert
              severity="info"
              data-testid="bond-noded"
              action={
                <Button
                  data-testid="un-bond"
                  disabled={isLoading}
                  onClick={async () => {
                    setIsLoading(true)
                    await unbond(ownership.nodeType)
                    await userBalance.fetchBalance()
                    await getBondDetails()
                    await checkOwnership()
                    setIsLoading(false)
                  }}
                  color="inherit"
                >
                  Unbond
                </Button>
              }
              sx={{ m: 2 }}
            >
              {`Looks like you already have a ${ownership.nodeType} bonded.`}
            </Alert>

            <Box sx={{ p: 3 }}>
              <Fee feeType="UnbondMixnode" />
            </Box>
          </>
        ) : (
          <Alert severity="info" sx={{ m: 3 }} data-testid="no-bond">
            You don't currently have a bonded node
          </Alert>
        )}
        {isLoading && (
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'center',
              p: 3,
              pt: 0,
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
      </NymCard>
    </Layout>
  )
}

import React, { useContext, useEffect, useState } from 'react'
import { Alert, Box, Button, CircularProgress } from '@mui/material'
import { BondForm } from './BondForm'
import { SuccessView } from './SuccessView'
import { NymCard } from '../../components'
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus'
import { Layout } from '../../layouts'
import { unbond } from '../../requests'
import { useCheckOwnership } from '../../hooks/useCheckOwnership'
import { ClientContext } from '../../context/main'
import { Bond as BondIcon } from '../../svg-icons/bond'

export const Bond = () => {
  const [status, setStatus] = useState(EnumRequestStatus.initial)
  const [error, setError] = useState<string>()
  const [successDetails, setSuccessDetails] = useState<{ amount: string; address: string }>()

  const { checkOwnership, ownership } = useCheckOwnership()
  const { userBalance, getBondDetails } = useContext(ClientContext)

  useEffect(() => {
    if (status === EnumRequestStatus.initial) {
      const initialiseForm = async () => {
        await checkOwnership()
        setStatus(EnumRequestStatus.initial)
      }
      initialiseForm()
    }
  }, [status])

  return (
    <Layout>
      <NymCard title="Bond" subheader="Bond a node or gateway" noPadding Icon={BondIcon}>
        {status === EnumRequestStatus.initial && (
          <Box sx={{ px: 3, mb: 1 }}>
            <Alert severity="warning">Always ensure you leave yourself enough funds to UNBOND</Alert>
          </Box>
        )}
        {ownership?.hasOwnership && (
          <Box sx={{ px: 3 }}>
            <Alert
              severity="info"
              action={
                <Button
                  disabled={status === EnumRequestStatus.loading}
                  onClick={async () => {
                    setStatus(EnumRequestStatus.loading)
                    await unbond(ownership.nodeType!)
                    await getBondDetails()
                    await userBalance.fetchBalance()
                    setStatus(EnumRequestStatus.initial)
                  }}
                  data-testid="unBond"
                  color="inherit"
                >
                  Unbond
                </Button>
              }
            >
              {`Looks like you already have a ${ownership.nodeType} bonded.`}
            </Alert>
          </Box>
        )}
        {status === EnumRequestStatus.loading && (
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'center',
              padding: 3,
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
        {status === EnumRequestStatus.initial && (
          <BondForm
            onError={(e?: string) => {
              setError(e)
              setStatus(EnumRequestStatus.error)
            }}
            onSuccess={(details) => {
              setSuccessDetails(details)
              setStatus(EnumRequestStatus.success)
            }}
            disabled={ownership?.hasOwnership}
          />
        )}
        {(status === EnumRequestStatus.error || status === EnumRequestStatus.success) && (
          <>
            <RequestStatus
              status={status}
              Success={<SuccessView details={successDetails} />}
              Error={
                <Alert severity="error" data-testid="bond-error">
                  An error occurred with the request: {error}
                </Alert>
              }
            />
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'flex-end',
                padding: 3,
                pt: 0,
              }}
            >
              <Button
                onClick={() => {
                  setStatus(EnumRequestStatus.initial)
                  checkOwnership()
                }}
              >
                {status === EnumRequestStatus.error ? 'Again?' : 'Finish'}
              </Button>
            </Box>
          </>
        )}
      </NymCard>
    </Layout>
  )
}

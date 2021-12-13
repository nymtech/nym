import React, { useEffect, useState } from 'react'
import { Alert, AlertTitle, Box, Button, CircularProgress } from '@mui/material'
import { DelegateForm } from './DelegateForm'
import { Layout } from '../../layouts'
import { NymCard } from '../../components'
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus'
import { TFee } from '../../types'
import { getGasFee } from '../../requests'
import { SuccessView } from './SuccessView'

export const Delegate = () => {
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial)
  const [error, setError] = useState<string>()
  const [successDetails, setSuccessDetails] = useState<{ amount: string; address: string }>()
  const [isLoading, setIsLoading] = useState(true)
  const [fees, setFees] = useState<TFee>()

  useEffect(() => {
    const getFees = async () => {
      const mixnode = await getGasFee('DelegateToMixnode')
      setFees({
        mixnode: mixnode,
      })
      setIsLoading(false)
    }
    getFees()
  }, [])

  return (
    <Layout>
      <NymCard title="Delegate" subheader="Delegate to mixnode" noPadding data-testid="delegateCard">
        {isLoading && (
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'center',
              p: 3,
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
        <>
          {status === EnumRequestStatus.initial && fees && (
            <DelegateForm
              fees={fees}
              onError={(message?: string) => {
                setStatus(EnumRequestStatus.error)
                setError(message)
              }}
              onSuccess={(details) => {
                setStatus(EnumRequestStatus.success)
                setSuccessDetails(details)
              }}
            />
          )}
          {status !== EnumRequestStatus.initial && (
            <>
              <RequestStatus
                status={status}
                Error={
                  <Alert severity="error" data-testid="delegate-error">
                    <AlertTitle>Delegation failed</AlertTitle>
                    An error occurred with the request:
                    <Box sx={{ wordBreak: 'break-word' }}>{error}</Box>
                  </Alert>
                }
                Success={<SuccessView details={successDetails} />}
              />
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-end',
                  borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
                  bgcolor: 'grey.100',
                  p: 2,
                }}
              >
                <Button
                  data-testid="finish-button"
                  onClick={() => {
                    setStatus(EnumRequestStatus.initial)
                  }}
                >
                  Finish
                </Button>
              </Box>
            </>
          )}
        </>
      </NymCard>
    </Layout>
  )
}

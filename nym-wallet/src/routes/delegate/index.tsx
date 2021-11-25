import React, { useEffect, useState } from 'react'
import { Alert, AlertTitle, Box, Button, CircularProgress } from '@mui/material'
import { DelegateForm } from './DelegateForm'
import { Layout } from '../../layouts'
import { NymCard } from '../../components'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { TFee } from '../../types'
import { getGasFee } from '../../requests'

export const Delegate = () => {
  const [status, setStatus] = useState<EnumRequestStatus>(
    EnumRequestStatus.initial,
  )
  const [message, setMessage] = useState<string>()
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
      <NymCard
        title="Delegate"
        subheader="Delegate to mixnode"
        noPadding
        data-testid="delegateCard"
      >
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
                setMessage(message)
              }}
              onSuccess={(message?: string) => {
                setStatus(EnumRequestStatus.success)
                setMessage(message)
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
                    <Box sx={{ wordBreak: 'break-word' }}>{message}</Box>
                  </Alert>
                }
                Success={
                  <Alert severity="success" data-testid="delegate-success">
                    <AlertTitle>Delegation complete</AlertTitle>
                    <Box style={{ wordBreak: 'break-word' }}>{message}</Box>
                  </Alert>
                }
              />
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-end',
                  borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
                  background: (theme) => theme.palette.grey[100],
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

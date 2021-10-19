import React, { useEffect, useState } from 'react'
import { Box, Button, CircularProgress, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { Alert, AlertTitle } from '@material-ui/lab'
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
    EnumRequestStatus.initial
  )
  const [message, setMessage] = useState<string>()
  const [isLoading, setIsLoading] = useState(true)
  const [fees, setFees] = useState<TFee>()

  useEffect(() => {
    const getFees = async () => {
      const mixnode = await getGasFee('DelegateToMixnode')
      const gateway = await getGasFee('DelegateToGateway')
      setFees({
        mixnode: mixnode,
        gateway: gateway,
      })
      setIsLoading(false)
    }

    getFees()
  }, [])

  const theme: Theme = useTheme()
  return (
    <Layout>
      <NymCard
        title="Delegate"
        subheader="Delegate to mixnode or gateway"
        noPadding
      >
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
                  <Alert severity="error">
                    An error occurred with the request:
                    <Box style={{ wordBreak: 'break-word' }}>{message}</Box>
                  </Alert>
                }
                Success={
                  <Alert severity="success">
                    <AlertTitle>Delegation complete</AlertTitle>
                    {message}
                  </Alert>
                }
              />
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-end',
                  borderTop: `1px solid ${theme.palette.grey[200]}`,
                  background: theme.palette.grey[100],
                  padding: theme.spacing(2),
                }}
              >
                <Button
                  onClick={() => {
                    setStatus(EnumRequestStatus.initial)
                  }}
                >
                  Finish
                </Button>
              </div>
            </>
          )}
        </>
      </NymCard>
    </Layout>
  )
}

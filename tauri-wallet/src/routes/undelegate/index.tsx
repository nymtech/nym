import React, { useEffect, useState } from 'react'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { NymCard } from '../../components'
import { UndelegateForm } from './UndelegateForm'
import { Layout } from '../../layouts'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { Box, CircularProgress, Theme } from '@material-ui/core'
import { getGasFee } from '../../requests'
import { TFee } from '../../types'

export const Undelegate = () => {
  const [message, setMessage] = useState<string>()
  const [status, setStaus] = useState<EnumRequestStatus>(
    EnumRequestStatus.initial
  )
  const [isLoading, setIsLoading] = useState(true)
  const [fees, setFees] = useState<TFee>()

  useEffect(() => {
    const getFees = async () => {
      const mixnode = await getGasFee('UndelegateFromMixnode')
      const gateway = await getGasFee('UndelegateFromGateway')
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
        title="Undelegate"
        subheader="Undelegate from a mixnode or gateway"
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
            <UndelegateForm
              fees={fees}
              onError={(message) => {
                setMessage(message)
                setStaus(EnumRequestStatus.error)
              }}
              onSuccess={(message) => {
                setMessage(message)
                setStaus(EnumRequestStatus.success)
              }}
            />
          )}
          {status !== EnumRequestStatus.initial && (
            <RequestStatus
              status={status}
              Error={
                <Alert severity="error">
                  An error occurred with the request: {message}
                </Alert>
              }
              Success={<Alert severity="success">{message}</Alert>}
            />
          )}
        </>
      </NymCard>
    </Layout>
  )
}

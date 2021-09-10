import React, { useEffect, useState } from 'react'
import { Box, Button, CircularProgress, Theme } from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { BondForm } from './BondForm'
import { NymCard } from '../../components'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { Layout } from '../../layouts'
import { getGasFee } from '../../requests'
import { Coin, EnumNodeType } from '../../types'

export const Bond = () => {
  const [status, setStatus] = useState(EnumRequestStatus.loading)
  const [message, setMessage] = useState<string>()

  const theme: Theme = useTheme()

  const [fees, setFees] = useState<{ [key in EnumNodeType]: Coin }>()

  useEffect(() => {
    const getFees = async () => {
      const mixnode = await getGasFee('BondMixnode')
      const gateway = await getGasFee('BondGateway')
      setFees({
        mixnode: mixnode,
        gateway: gateway,
      })
      setStatus(EnumRequestStatus.initial)
    }

    getFees()
  }, [])

  console.log(fees, status, message)
  return (
    <Layout>
      <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
        {status === EnumRequestStatus.loading && (
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
        {status === EnumRequestStatus.initial && (
          <BondForm
            fees={fees!}
            onError={(e?: string) => {
              setMessage(e)
              setStatus(EnumRequestStatus.error)
            }}
            onSuccess={(message?: string) => {
              setMessage(message)
              setStatus(EnumRequestStatus.success)
            }}
          />
        )}
        {(status === EnumRequestStatus.error ||
          status === EnumRequestStatus.success) && (
          <>
            <RequestStatus
              status={status}
              Success={
                <Alert severity="success">Successfully bonded node</Alert>
              }
              Error={
                <Alert severity="error">
                  An error occurred with the request: {message}
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
                Resend?
              </Button>
            </div>
          </>
        )}
      </NymCard>
    </Layout>
  )
}

import React, { useState } from 'react'
import { Button, Theme } from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { NymCard } from '../../components'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { Layout } from '../../layouts'
import { BondForm } from './BondForm'

export const Bond = () => {
  const [status, setStatus] = useState(EnumRequestStatus.initial)
  const [message, setMessage] = useState<string>()

  const theme: Theme = useTheme()

  return (
    <Layout>
      <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
        <>
          {status === EnumRequestStatus.initial && (
            <BondForm
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
          {status !== EnumRequestStatus.initial && (
            <>
              <RequestStatus
                status={status}
                onSuccess={() => (
                  <Alert severity="success">Successfully bonded node</Alert>
                )}
                onError={() => (
                  <Alert severity="error">
                    An error occurred with the request: {message}
                  </Alert>
                )}
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
        </>
      </NymCard>
    </Layout>
  )
}

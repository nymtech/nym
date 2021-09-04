import React, { useState } from 'react'
import { Button, Theme } from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { DelegateForm } from './DelegateForm'
import { Layout } from '../../layouts'
import { NymCard } from '../../components'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { Alert } from '@material-ui/lab'

export const Delegate = () => {
  const [status, setStatus] = useState<EnumRequestStatus>(
    EnumRequestStatus.initial
  )
  const [message, setMessage] = useState<string>()

  const theme: Theme = useTheme()
  return (
    <Layout>
      <NymCard
        title="Delegate"
        subheader="Delegate to mixnode or gateway"
        noPadding
      >
        <>
          {status === EnumRequestStatus.initial && (
            <DelegateForm
              onError={(message?: string) => {
                setStatus(EnumRequestStatus.error)
                setMessage(message)
              }}
              onSuccess={() => {
                setStatus(EnumRequestStatus.success)
              }}
            />
          )}
          {status !== EnumRequestStatus.initial && (
            <>
              <RequestStatus
                status={status}
                onError={() => (
                  <Alert severity="error">
                    An error occurred with the request: {message}
                  </Alert>
                )}
                onSuccess={() => {}}
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

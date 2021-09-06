import React, { useState } from 'react'
import { NymCard } from '../../components'
import { UndelegateForm } from './UndelegateForm'
import { Layout } from '../../layouts'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { Alert } from '@material-ui/lab'

export const Undelegate = () => {
  const [message, setMessage] = useState<string>()
  const [status, setStaus] = useState<EnumRequestStatus>(
    EnumRequestStatus.initial
  )

  return (
    <Layout>
      <NymCard
        title="Undelegate"
        subheader="Undelegate from a mixnode or gateway"
        noPadding
      >
        <>
          {status === EnumRequestStatus.initial && (
            <UndelegateForm
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

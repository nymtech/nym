import React, { useEffect, useState } from 'react'
import { NymCard } from '../../components'
import { UndelegateForm } from './UndelegateForm'
import { Layout } from '../../layouts'
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus'
import { Alert, AlertTitle, Box, Button, CircularProgress } from '@mui/material'
import { getGasFee, getReverseMixDelegations } from '../../requests'
import { TFee, TDelegation } from '../../types'

export type TDelegations = {
  mixnodes: TDelegation
}

export const Undelegate = () => {
  const [message, setMessage] = useState<string>()
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial)
  const [isLoading, setIsLoading] = useState(true)
  const [fees, setFees] = useState<TFee>()
  const [delegations, setDelegations] = useState<TDelegations>()

  useEffect(() => {
    initialize()
  }, [])

  const initialize = async () => {
    setIsLoading(true)

    try {
      const [mixnodeFee, mixnodeDelegations] = await Promise.all([
        getGasFee('UndelegateFromMixnode'),
        getReverseMixDelegations(),
      ])

      setFees({
        mixnode: mixnodeFee,
      })

      setDelegations({
        mixnodes: mixnodeDelegations,
      })
    } catch {
      setStatus(EnumRequestStatus.error)
      setMessage('An error occured when initialising the page')
    }

    setIsLoading(false)
  }

  return (
    <Layout>
      <NymCard title="Undelegate" subheader="Undelegate from a mixnode" noPadding>
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
          {status === EnumRequestStatus.initial && fees && delegations && (
            <UndelegateForm
              fees={fees}
              delegations={delegations}
              onError={(message) => {
                setMessage(message)
                setStatus(EnumRequestStatus.error)
              }}
              onSuccess={(message) => {
                setMessage(message)
                setStatus(EnumRequestStatus.success)
              }}
            />
          )}
          {status !== EnumRequestStatus.initial && (
            <>
              <RequestStatus
                status={status}
                Error={
                  <Alert severity="error" data-testid="request-error">
                    An error occurred with the request: {message}
                  </Alert>
                }
                Success={
                  <Alert severity="success">
                    {' '}
                    <AlertTitle data-testid="undelegate-success">Undelegation complete</AlertTitle>
                    {message}
                  </Alert>
                }
              />
              <Box
                sx={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'flex-end',
                  borderTop: '1px solid grey[200]',
                  bgcolor: 'grey.100',
                  p: 2,
                }}
              >
                <Button
                  data-testid="finish-button"
                  variant="contained"
                  disableElevation
                  onClick={() => {
                    setStatus(EnumRequestStatus.initial)
                    initialize()
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

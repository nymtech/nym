import React, { useContext, useEffect, useState } from 'react'
import { Alert, AlertTitle, Box, Button, CircularProgress } from '@mui/material'
import { NymCard } from '../../components'
import { UndelegateForm } from './UndelegateForm'
import { Layout } from '../../layouts'
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus'
import { getGasFee, getReverseMixDelegations } from '../../requests'
import { TFee, TPagedDelegations } from '../../types'
import { Undelegate as UndelegateIcon } from '../../svg-icons'
import { ClientContext } from '../../context/main'

export const Undelegate = () => {
  const [message, setMessage] = useState<string>()
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial)
  const [isLoading, setIsLoading] = useState(true)
  const [fees, setFees] = useState<TFee>()
  const [pagedDelegations, setPagesDelegations] = useState<TPagedDelegations>()

  const { clientDetails } = useContext(ClientContext)

  useEffect(() => {
    initialize()
  }, [clientDetails])

  const initialize = async () => {
    setStatus(EnumRequestStatus.initial)
    setIsLoading(true)

    try {
      const [mixnodeFee, mixnodeDelegations] = await Promise.all([
        getGasFee('UndelegateFromMixnode'),
        getReverseMixDelegations(),
      ])

      setFees({
        mixnode: mixnodeFee,
      })

      setPagesDelegations(mixnodeDelegations)
    } catch (e) {
      setStatus(EnumRequestStatus.error)
      setMessage(e as string)
    }

    setIsLoading(false)
  }

  return (
    <Layout>
      <NymCard title="Undelegate" subheader="Undelegate from a mixnode" Icon={UndelegateIcon} noPadding>
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
          {status === EnumRequestStatus.initial && fees && pagedDelegations && (
            <UndelegateForm
              fees={fees}
              delegations={pagedDelegations?.delegations}
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
                  p: 3,
                  pt: 0,
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
                  size="large"
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

import React, { useContext, useState } from 'react'
import { Alert, AlertTitle, Box, Button, Link, Typography } from '@mui/material'
import { DelegateForm } from './DelegateForm'
import { Layout } from '../../layouts'
import { NymCard } from '../../components'
import { EnumRequestStatus, RequestStatus } from '../../components/RequestStatus'
import { SuccessView } from './SuccessView'
import { Delegate as DelegateIcon } from '../../svg-icons'
import { urls, ClientContext } from '../../context/main'

export const Delegate = () => {
  const [status, setStatus] = useState<EnumRequestStatus>(EnumRequestStatus.initial)
  const [error, setError] = useState<string>()
  const [successDetails, setSuccessDetails] = useState<{ amount: string; address: string }>()

  const {network} = useContext(ClientContext)
  
  return (
    <Layout>
      <>
        <NymCard
          title="Delegate"
          subheader="Delegate to mixnode"
          noPadding
          data-testid="delegateCard"
          Icon={DelegateIcon}
        >
          <>
            {status === EnumRequestStatus.initial && (
              <Box sx={{ px: 3, mb: 1 }}>
                <Alert severity="warning">Always ensure you leave yourself enough funds to UNDELEGATE</Alert>
              </Box>
            )}
            {status === EnumRequestStatus.initial && (
              <DelegateForm
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
                    p: 3,
                    pt: 0,
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
        <Typography sx={{ p: 3 }}>
          Checkout the{' '}
          <Link href={`${urls(network).networkExplorer}/network-components/mixnodes`} target="_blank">
            list of mixnodes
          </Link>{' '}
          for uptime and performances to help make delegation decisions
        </Typography>
      </>
    </Layout>
  )
}

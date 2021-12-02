import React, { useContext, useEffect, useState } from 'react'
import { Alert, Box, Button, CircularProgress } from '@mui/material'
import { BondForm } from './BondForm'
import { NymCard } from '../../components'
import {
  EnumRequestStatus,
  RequestStatus,
} from '../../components/RequestStatus'
import { Layout } from '../../layouts'
import { getGasFee, unbond } from '../../requests'
import { TFee } from '../../types'
import { useCheckOwnership } from '../../hooks/useCheckOwnership'
import { ClientContext } from '../../context/main'

export const Bond = () => {
  const [status, setStatus] = useState(EnumRequestStatus.initial)
  const [message, setMessage] = useState<string>()
  const [fees, setFees] = useState<TFee>()

  const { checkOwnership, ownership } = useCheckOwnership()
  const { userBalance } = useContext(ClientContext)

  useEffect(() => {
    if (status === EnumRequestStatus.initial) {
      const initialiseForm = async () => {
        await checkOwnership()
        setFees({
          mixnode: await getGasFee('BondMixnode'),
          gateway: await getGasFee('BondGateway'),
        })
        setStatus(EnumRequestStatus.initial)
      }
      initialiseForm()
    }
  }, [status])

  return (
    <Layout>
      <NymCard title="Bond" subheader="Bond a node or gateway" noPadding>
        {ownership?.hasOwnership && (
          <Alert
            severity="warning"
            action={
              <Button
                disabled={status === EnumRequestStatus.loading}
                onClick={async () => {
                  setStatus(EnumRequestStatus.loading)
                  await unbond(ownership.nodeType!)
                  userBalance.fetchBalance()
                  setStatus(EnumRequestStatus.initial)
                }}
                data-testid="unBond"
              >
                Unbond
              </Button>
            }
            style={{ margin: 2 }}
          >
            {`Looks like you already have a ${ownership.nodeType} bonded.`}
          </Alert>
        )}
        {status === EnumRequestStatus.loading && (
          <Box
            sx={{
              display: 'flex',
              justifyContent: 'center',
              padding: 3,
            }}
          >
            <CircularProgress size={48} />
          </Box>
        )}
        {status === EnumRequestStatus.initial && (
          <BondForm
            fees={!ownership.hasOwnership ? fees : undefined}
            onError={(e?: string) => {
              setMessage(e)
              setStatus(EnumRequestStatus.error)
            }}
            onSuccess={(message?: string) => {
              setMessage(message)
              setStatus(EnumRequestStatus.success)
            }}
            disabled={ownership?.hasOwnership}
          />
        )}
        {(status === EnumRequestStatus.error ||
          status === EnumRequestStatus.success) && (
          <>
            <RequestStatus
              status={status}
              Success={
                <Alert severity="success" data-testid="bond-success">
                  Successfully bonded node
                </Alert>
              }
              Error={
                <Alert severity="error" data-testid="bond-error">
                  An error occurred with the request: {message}
                </Alert>
              }
            />
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'flex-end',
                borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
                bgcolor: (theme) => theme.palette.grey[50],
                padding: 2,
              }}
            >
              <Button
                onClick={() => {
                  setStatus(EnumRequestStatus.initial)
                  checkOwnership()
                }}
              >
                Again?
              </Button>
            </Box>
          </>
        )}
      </NymCard>
    </Layout>
  )
}

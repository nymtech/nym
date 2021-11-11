import React, { useContext, useEffect, useState } from 'react'
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
import { getGasFee, unbond } from '../../requests'
import { TFee } from '../../types'
import { useCheckOwnership } from '../../hooks/useCheckOwnership'
import { ClientContext } from '../../context/main'

export const Bond = () => {
  const [status, setStatus] = useState(EnumRequestStatus.initial)
  const [message, setMessage] = useState<string>()
  const [fees, setFees] = useState<TFee>()

  const { checkOwnership, ownership } = useCheckOwnership()
  const { getBalance } = useContext(ClientContext)

  const theme: Theme = useTheme()

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
                  getBalance.fetchBalance()
                  setStatus(EnumRequestStatus.initial)
                }}
                data-testid="unBond"
              >
                Unbond
              </Button>
            }
            style={{ margin: theme.spacing(2) }}
          >
            {`Looks like you already have a ${ownership.nodeType} bonded.`}
          </Alert>
        )}
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
                <Alert severity="success" data-testid="bond-success">Successfully bonded node</Alert>
              }
              Error={
                <Alert severity="error" data-testid="bond-error">
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
                  checkOwnership()
                }}
              >
                Again?
              </Button>
            </div>
          </>
        )}
      </NymCard>
    </Layout>
  )
}

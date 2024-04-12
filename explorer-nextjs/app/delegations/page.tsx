'use client'

import React, { useEffect, useMemo } from 'react'
import {
  Alert,
  AlertTitle,
  Box,
  Button,
  Card,
  Chip,
  IconButton,
  Tooltip,
  Typography,
} from '@mui/material'
import { DelegationModal, DelegationModalProps, Title } from '@/app/components'
import { useWalletContext } from '@/app/context/wallet'
import { unymToNym } from '@/app/utils/currency'
import {
  DelegationWithRewards,
  DelegationsProvider,
  PendingEvent,
  useDelegationsContext,
} from '@/app/context/delegations'
import { urls } from '@/app/utils'
import { useClipboard } from 'use-clipboard-copy'
import { Close } from '@mui/icons-material'
import { useRouter } from 'next/navigation'
import {
  MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from 'material-react-table'

const mapToDelegationsRow = (
  delegation: DelegationWithRewards,
  index: number
) => ({
  identity: delegation.identityKey,
  mix_id: delegation.mix_id,
  amount: `${unymToNym(delegation.amount.amount)} NYM`,
  rewards: `${unymToNym(delegation.rewards)} NYM`,
  id: index,
  pending: delegation.pending,
})

const Banner = ({ onClose }: { onClose: () => void }) => {
  const { copy } = useClipboard()

  return (
    <Alert
      severity="info"
      sx={{ mb: 3, fontSize: 'medium', width: '100%' }}
      action={
        <IconButton size="small" onClick={onClose}>
          <Close fontSize="small" />
        </IconButton>
      }
    >
      <AlertTitle> Mobile Delegations Beta</AlertTitle>
      <Box>
        <Typography>
          This is a beta release for mobile delegations If you have any feedback
          or feature suggestions contact us at support@nymte.ch
          <Button
            size="small"
            onClick={() => copy('support@nymte.ch')}
            sx={{ display: 'inline-block' }}
          >
            Copy
          </Button>
        </Typography>
      </Box>
    </Alert>
  )
}

const DelegationsPage = () => {
  const [confirmationModalProps, setConfirmationModalProps] = React.useState<
    DelegationModalProps | undefined
  >()
  const [isLoading, setIsLoading] = React.useState(false)
  const [showBanner, setShowBanner] = React.useState(true)

  const { isWalletConnected } = useWalletContext()
  const { handleGetDelegations, handleUndelegate, delegations } =
    useDelegationsContext()

  const router = useRouter()

  useEffect(() => {
    let timeoutId: NodeJS.Timeout

    const fetchDelegations = async () => {
      setIsLoading(true)
      try {
        await handleGetDelegations()
      } catch (error) {
        setConfirmationModalProps({
          status: 'error',
          message: "Couldn't fetch delegations. Please try again later.",
        })
      } finally {
        setIsLoading(false)

        timeoutId = setTimeout(() => {
          fetchDelegations()
        }, 60_000)
      }
    }

    fetchDelegations()

    return () => {
      clearTimeout(timeoutId)
    }
  }, [handleGetDelegations])

  const getTooltipTitle = (pending: PendingEvent) => {
    if (pending?.kind === 'undelegate') {
      return 'You have an undelegation pending'
    }

    if (pending?.kind === 'delegate') {
      return `You have a delegation pending worth ${unymToNym(
        pending.amount.amount
      )} NYM`
    }

    return undefined
  }

  const onUndelegate = async (mixId: number) => {
    setConfirmationModalProps({ status: 'loading' })

    try {
      const tx = await handleUndelegate(mixId)

      if (tx) {
        setConfirmationModalProps({
          status: 'success',
          message: 'Undelegation can take up to one hour to process',
          transactions: [
            {
              url: `${urls('MAINNET').blockExplorer}/transaction/${
                tx.transactionHash
              }`,
              hash: tx.transactionHash,
            },
          ],
        })
      }
    } catch (error) {
      if (error instanceof Error) {
        setConfirmationModalProps({ status: 'error', message: error.message })
      }
    }
  }

  const columns = useMemo<
    MRT_ColumnDef<ReturnType<typeof mapToDelegationsRow>>[]
  >(() => {
    return [
      {
        id: 'delegations-data',
        header: 'Delegations Data',
        columns: [
          {
            id: 'identity',
            accessorKey: 'identity',
            header: 'Identity Key',
            width: 400,
          },
          {
            id: 'mix_id',
            accessorKey: 'mix_id',
            header: 'Mix ID',
            size: 150,
          },
          {
            id: 'amount',
            accessorKey: 'amount',
            header: 'Amount',
            width: 150,
          },
          {
            id: 'rewards',
            accessorKey: 'rewards',
            header: 'Rewards',
            width: 150,
            enableColumnFilters: false,
          },
          {
            id: 'undelegate',
            accessorKey: 'undelegate',
            header: '',
            enableSorting: false,
            enableColumnActions: false,
            Filter: () => null,
            Cell: ({ row }) => {
              return (
                <Box
                  sx={{ width: '100%', display: 'flex', justifyContent: 'end' }}
                >
                  {row.original.pending ? (
                    <Tooltip
                      placement="left"
                      title={getTooltipTitle(row.original.pending)}
                      onClick={(e) => e.stopPropagation()}
                      PopperProps={{}}
                    >
                      <Chip size="small" label="Pending events" />
                    </Tooltip>
                  ) : (
                    <Button
                      size="small"
                      variant="outlined"
                      onClick={(e) => {
                        e.stopPropagation()
                        onUndelegate(row.original.mix_id)
                      }}
                    >
                      Undelegate
                    </Button>
                  )}
                </Box>
              )
            },
          },
        ],
      },
    ]
  }, [])

  const data = useMemo(() => {
    return (delegations || []).map(mapToDelegationsRow)
  }, [delegations])

  const table = useMaterialReactTable({
    columns,
    data,
    enableFullScreenToggle: false,
    state: {
      isLoading,
    },
    initialState: {
      columnPinning: { right: ['undelegate'] },
    },
  })

  return (
    <Box>
      {confirmationModalProps && (
        <DelegationModal
          {...confirmationModalProps}
          open={Boolean(confirmationModalProps)}
          onClose={async () => {
            if (confirmationModalProps.status === 'success') {
              await handleGetDelegations()
            }
            setConfirmationModalProps(undefined)
          }}
          sx={{
            width: {
              xs: '90%',
              sm: 600,
            },
          }}
        />
      )}
      {showBanner && <Banner onClose={() => setShowBanner(false)} />}
      <Box display="flex" justifyContent="space-between" alignItems="center">
        <Title text="Your Delegations" />
        <Button
          variant="contained"
          color="primary"
          onClick={() => router.push('/network-components/mixnodes')}
        >
          Delegate
        </Button>
      </Box>
      {!isWalletConnected ? (
        <Box>
          <Typography mb={2} variant="h6">
            Connect your wallet to view your delegations.
          </Typography>
        </Box>
      ) : null}

      <Card
        sx={{
          mt: 2,
          padding: 2,
          height: '100%',
        }}
      >
        <MaterialReactTable table={table} />
      </Card>
    </Box>
  )
}

const Delegations = () => (
  <DelegationsProvider>
    <DelegationsPage />
  </DelegationsProvider>
)

export default Delegations

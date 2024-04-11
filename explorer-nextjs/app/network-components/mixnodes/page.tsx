'use client'

import React, { useMemo } from 'react'
import router from 'next/router'
import {
  MaterialReactTable,
  useMaterialReactTable,
  type MRT_ColumnDef,
} from 'material-react-table'
import { Grid, Card, Button, Box, Stack } from '@mui/material'
import {
  CustomColumnHeading,
  DelegateIconButton,
  DelegationModalProps,
  MixNodeStatusDropdown,
  MixnodeRowType,
  StyledLink,
  TableToolbar,
  Title,
  Tooltip,
  mixnodeToGridRow,
} from '@/app/components'
import { DelegationsProvider } from '@/app/context/delegations'
import { useWalletContext } from '@/app/context/wallet'
import { useGetMixNodeStatusColor, useIsMobile } from '@/app/hooks'
import { useMainContext } from '@/app/context/main'
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard.js'
import { splice } from '@/app/utils'
import { currencyToString } from '@/app/utils/currency'
import { NYM_BIG_DIPPER } from '@/app/api/constants'

export default function MixnodesPage() {
  const isMobile = useIsMobile()
  const { isWalletConnected } = useWalletContext()
  const { mixnodes, fetchMixnodes } = useMainContext()

  const [itemSelectedForDelegation, setItemSelectedForDelegation] =
    React.useState<{
      mixId: number
      identityKey: string
    }>()
  const [confirmationModalProps, setConfirmationModalProps] = React.useState<
    DelegationModalProps | undefined
  >()

  console.log(mixnodeToGridRow(mixnodes?.data))

  React.useEffect(() => {
    // when the status changes, get the mixnodes
    fetchMixnodes()
  }, [])

  const handleOnDelegate = ({
    identityKey,
    mixId,
  }: {
    identityKey: string
    mixId: number
  }) => {
    if (!isWalletConnected) {
      setConfirmationModalProps({
        status: 'info',
        message: 'Please connect your wallet to delegate',
      })
    } else {
      setItemSelectedForDelegation({ identityKey, mixId })
    }
  }

  const columns = useMemo<MRT_ColumnDef<MixnodeRowType>[]>(() => {
    return [
      {
        id: 'mixnode-data',
        header: 'Mixnode Data',
        columns: [
          {
            id: 'delegate',
            header: 'Delegate',
            size: isMobile ? 25 : 150,
            grow: false,
            accessorFn: (row) => (
              <DelegateIconButton
                size="small"
                onDelegate={() =>
                  handleOnDelegate({
                    identityKey: row.identity_key,
                    mixId: row.mix_id,
                  })
                }
              />
            ),
          },
          {
            id: 'identity_key',
            header: 'Identity Key',
            accessorKey: 'identity_key',
            size: 250,
            Cell: ({ row }) => {
              return (
                <Stack direction="row" alignItems="center" gap={1}>
                  <CopyToClipboard
                    sx={{ mr: 0.5, color: 'grey.400' }}
                    smallIcons
                    value={row.original.identity_key}
                    tooltip={`Copy identity key ${row.original.identity_key} to clipboard`}
                  />
                  <StyledLink
                    to={`/network-components/mixnode/${row.original.mix_id}`}
                    color={useGetMixNodeStatusColor(row.original.status)}
                    dataTestId="identity-link"
                  >
                    {splice(7, 29, row.original.identity_key)}
                  </StyledLink>
                </Stack>
              )
            },
          },
          {
            id: 'bond',
            header: 'Stake',
            accessorKey: 'bond',
            Cell: ({ row }) => (
              <StyledLink
                to={`/network-components/mixnode/${row.original.mix_id}`}
                color={useGetMixNodeStatusColor(row.original.status)}
              >
                {currencyToString({ amount: row.original.bond.toString() })}
              </StyledLink>
            ),
          },
          {
            id: 'stake_saturation',
            header: 'Stake Saturation',
            accessorKey: 'stake_saturation',
            size: 225,
            Header() {
              return (
                <CustomColumnHeading
                  headingTitle="Stake Saturation"
                  tooltipInfo="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is 940k NYMs, computed as S/K where S is target amount of tokens staked in the network and K is the number of nodes in the reward set."
                />
              )
            },
            Cell: ({ row }) => (
              <StyledLink
                to={`/network-components/mixnode/${row.original.mix_id}`}
                color={useGetMixNodeStatusColor(row.original.status)}
              >{`${row.original.stake_saturation} %`}</StyledLink>
            ),
          },
          {
            id: 'pledge_amount',
            header: 'Bond',
            accessorKey: 'pledge_amount',
            size: 185,
            Header: () => (
              <CustomColumnHeading
                headingTitle="Bond"
                tooltipInfo="Node operator's share of stake."
              />
            ),
            Cell: ({ row }) => (
              <StyledLink
                to={`/network-components/mixnode/${row.original.mix_id}`}
                color={useGetMixNodeStatusColor(row.original.status)}
              >
                {currencyToString({
                  amount: row.original.pledge_amount.toString(),
                })}
              </StyledLink>
            ),
          },
        ],
      },
      {
        id: 'profit_percentage',
        accessorKey: 'profit_percentage',
        header: 'Profit Margin',
        size: 145,
        Header: () => (
          <CustomColumnHeading
            headingTitle="Profit Margin"
            tooltipInfo="Percentage of the delegators rewards that the operator takes as fee before rewards are distributed to the delegators."
          />
        ),
        Cell: ({ row }) => (
          <StyledLink
            to={`/network-components/mixnode/${row.original.mix_id}`}
            color={useGetMixNodeStatusColor(row.original.status)}
          >{`${row.original.profit_percentage}%`}</StyledLink>
        ),
      },
      {
        id: 'operating_cost',
        accessorKey: 'operating_cost',
        size: 220,
        header: 'Operating Cost',
        disableColumnMenu: true,
        Header: () => (
          <CustomColumnHeading
            headingTitle="Operating Cost"
            tooltipInfo="Monthly operational cost of running this node. This cost is set by the operator and it influences how the rewards are split between the operator and delegators."
          />
        ),
        Cell: ({ row }) => (
          <StyledLink
            to={`/network-components/mixnode/${row.original.mix_id}`}
            color={useGetMixNodeStatusColor(row.original.status)}
          >{`${row.original.operating_cost} NYM`}</StyledLink>
        ),
      },
      {
        id: 'node_performance',
        accessorKey: 'node_performance',
        size: 200,
        header: 'Routing Score',
        Header: () => (
          <CustomColumnHeading
            headingTitle="Routing Score"
            tooltipInfo="Mixnode's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test."
          />
        ),
        Cell: ({ row }) => (
          <StyledLink
            to={`/network-components/mixnode/${row.original.mix_id}`}
            color={useGetMixNodeStatusColor(row.original.status)}
          >{`${row.original.node_performance}%`}</StyledLink>
        ),
      },
      {
        id: 'owner',
        accessorKey: 'owner',
        size: 150,
        header: 'Owner',
        Header: () => <CustomColumnHeading headingTitle="Owner" />,
        Cell: ({ row }) => (
          <StyledLink
            to={`${NYM_BIG_DIPPER}/account/${row.original.owner}`}
            color={useGetMixNodeStatusColor(row.original.status)}
            target="_blank"
            data-testid="big-dipper-link"
          >
            {splice(7, 29, row.original.owner)}
          </StyledLink>
        ),
      },
      {
        id: 'location',
        accessorKey: 'location',
        header: 'Location',
        maxSize: 150,
        Header: () => <CustomColumnHeading headingTitle="Location" />,
        Cell: ({ row }) => (
          <Tooltip text={row.original.location} id="mixnode-location-text">
            <Box
              sx={{
                overflow: 'hidden',
                whiteSpace: 'nowrap',
                textOverflow: 'ellipsis',
                cursor: 'pointer',
                color: useGetMixNodeStatusColor(row.original.status),
              }}
            >
              {row.original.location}
            </Box>
          </Tooltip>
        ),
      },
      {
        id: 'host',
        accessorKey: 'host',
        header: 'Host',
        size: 130,
        Header: () => <CustomColumnHeading headingTitle="Host" />,
        Cell: ({ row }) => (
          <StyledLink
            color={useGetMixNodeStatusColor(row.original.status)}
            to={`/network-components/mixnode/${row.original.mix_id}`}
          >
            {row.original.host}
          </StyledLink>
        ),
      },
    ]
  }, [])

  const data = useMemo(() => {
    return mixnodeToGridRow(mixnodes?.data)
  }, [mixnodes?.data])

  const table = useMaterialReactTable({
    columns,
    data,
    enableFullScreenToggle: false,
    layoutMode: 'grid-no-grow',
  })

  return (
    <DelegationsProvider>
      <Box mb={2}>
        <Title text="Mixnodes" />
      </Box>
      <Grid container>
        <Grid item xs={12}>
          <Card
            sx={{
              padding: 2,
              height: '100%',
            }}
          >
            <TableToolbar
              childrenBefore={
                <MixNodeStatusDropdown
                  sx={{ mr: 2 }}
                  status={'active'}
                  onSelectionChanged={() => {}}
                />
              }
              childrenAfter={
                isWalletConnected && (
                  <Button
                    fullWidth
                    size="large"
                    variant="outlined"
                    onClick={() => router.push('/delegations')}
                  >
                    Delegations
                  </Button>
                )
              }
              onChangeSearch={() => {}}
              onChangePageSize={() => {}}
              pageSize={''}
              searchTerm={''}
              withFilters
            />
            <MaterialReactTable table={table} />
          </Card>
        </Grid>
      </Grid>
    </DelegationsProvider>
  )
}

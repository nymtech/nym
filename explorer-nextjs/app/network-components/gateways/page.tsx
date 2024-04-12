'use client'

import React, { useMemo } from 'react'
import { Box, Card, Grid, Stack } from '@mui/material'
import { useTheme } from '@mui/material/styles'
import {
  MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from 'material-react-table'
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid'
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard'
import { Tooltip as InfoTooltip } from '@nymproject/react/tooltip/Tooltip'
import { diff, gte, rcompare } from 'semver'
import { useMainContext } from '@/app/context/main'
import { TableToolbar } from '@/app/components/TableToolbar'
import { CustomColumnHeading } from '@/app/components/CustomColumnHeading'
import { Title } from '@/app/components/Title'
import { unymToNym } from '@/app/utils/currency'
import { Tooltip } from '@/app/components/Tooltip'
import { NYM_BIG_DIPPER } from '@/app/api/constants'
import { splice } from '@/app/utils'
import {
  VersionDisplaySelector,
  VersionSelectOptions,
} from '@/app/components/Gateways/VersionDisplaySelector'
import StyledLink from '@/app/components/StyledLink'
import {
  GatewayRowType,
  gatewayToGridRow,
} from '@/app/components/Gateways/Gateways'

export const PageGateways: FCWithChildren = () => {
  const { gateways } = useMainContext()
  const [versionFilter, setVersionFilter] =
    React.useState<VersionSelectOptions>(VersionSelectOptions.all)

  const theme = useTheme()

  const highestVersion = React.useMemo(() => {
    if (gateways?.data) {
      const versions = gateways.data.reduce(
        (a: string[], b) => [...a, b.gateway.version],
        []
      )
      const [lastestVersion] = versions.sort(rcompare)
      return lastestVersion
    }
    // fallback value
    return '2.0.0'
  }, [gateways])

  const filterByLatestVersions = React.useMemo(() => {
    const filtered = gateways?.data?.filter((gw) => {
      const versionDiff = diff(highestVersion, gw.gateway.version)
      return versionDiff === 'patch' || versionDiff === null
    })
    if (filtered) return filtered
    return []
  }, [gateways])

  const filterByOlderVersions = React.useMemo(() => {
    const filtered = gateways?.data?.filter((gw) => {
      const versionDiff = diff(highestVersion, gw.gateway.version)
      return versionDiff === 'major' || versionDiff === 'minor'
    })
    if (filtered) return filtered
    return []
  }, [gateways])

  const filteredByVersion = React.useMemo(() => {
    switch (versionFilter) {
      case VersionSelectOptions.latestVersion:
        return filterByLatestVersions
      case VersionSelectOptions.olderVersions:
        return filterByOlderVersions
      case VersionSelectOptions.all:
        return gateways?.data || []
      default:
        return []
    }
  }, [versionFilter, gateways])

  const data = useMemo(() => {
    return gatewayToGridRow(filteredByVersion || [])
  }, [filteredByVersion])

  const columns = useMemo<MRT_ColumnDef<GatewayRowType>[]>(() => {
    return [
      {
        id: 'gateway-data',
        header: 'Gatewsay Data',
        columns: [
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
                    to={`/network-components/gateway/${row.original.identity_key}`}
                    dataTestId="identity-link"
                    color="text.primary"
                  >
                    {splice(7, 29, row.original.identity_key)}
                  </StyledLink>
                </Stack>
              )
            },
          },
          {
            id: 'node_performance',
            header: 'Node Performance',
            accessorKey: 'node_performance',
            size: 200,
            Header: () => {
              return (
                <Box display="flex">
                  <InfoTooltip
                    id="gateways-list-routing-score"
                    title="Gateway's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test"
                    placement="top-start"
                    textColor={theme.palette.nym.networkExplorer.tooltip.color}
                    bgColor={
                      theme.palette.nym.networkExplorer.tooltip.background
                    }
                    maxWidth={230}
                    arrow
                  />
                  <CustomColumnHeading headingTitle="Routing Score" />
                </Box>
              )
            },
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/gateway/${row.original.identity_key}`}
                  data-testid="node-performance"
                  color="text.primary"
                >
                  {`${row.original.node_performance}%`}
                </StyledLink>
              )
            },
          },
          {
            id: 'version',
            header: 'Version',
            accessorKey: 'version',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/gateway/${row.original.identity_key}`}
                  data-testid="version"
                  color="text.primary"
                >
                  {row.original.version}
                </StyledLink>
              )
            },
          },
          {
            id: 'location',
            header: 'Location',
            accessorKey: 'location',
            size: 150,
            Cell: ({ row }) => {
              return (
                <Box
                  sx={{ justifyContent: 'flex-start', cursor: 'pointer' }}
                  data-testid="location-button"
                >
                  <Tooltip
                    text={row.original.location}
                    id="gateway-location-text"
                  >
                    <Box
                      sx={{
                        overflow: 'hidden',
                        whiteSpace: 'nowrap',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      {row.original.location}
                    </Box>
                  </Tooltip>
                </Box>
              )
            },
          },
          {
            id: 'host',
            header: 'IP:Port',
            accessorKey: 'host',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/gateway/${row.original.identity_key}`}
                  data-testid="host"
                  color="text.primary"
                >
                  {row.original.host}
                </StyledLink>
              )
            },
          },
          {
            id: 'owner',
            header: 'Owner',
            accessorKey: 'owner',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`${NYM_BIG_DIPPER}/account/${row.original.owner}`}
                  target="_blank"
                  data-testid="owner"
                  color="text.primary"
                >
                  {splice(7, 29, row.original.owner)}
                </StyledLink>
              )
            },
          },
        ],
      },
    ]
  }, [])

  const _columns: GridColDef[] = [
    {
      field: 'node_performance',
      align: 'center',
      renderHeader: () => (
        <>
          <InfoTooltip
            id="gateways-list-routing-score"
            title="Gateway's most recent score (measured in the last 15 minutes). Routing score is relative to that of the network. Each time a gateway is tested, the test packets have to go through the full path of the network (gateway + 3 nodes). If a node in the path drop packets it will affect the score of the gateway and other nodes in the test"
            placement="top-start"
            textColor={theme.palette.nym.networkExplorer.tooltip.color}
            bgColor={theme.palette.nym.networkExplorer.tooltip.background}
            maxWidth={230}
            arrow
          />
          <CustomColumnHeading headingTitle="Routing Score" />
        </>
      ),
      width: 120,
      disableColumnMenu: true,
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="pledge-amount"
        >
          {`${params.value}%`}
        </StyledLink>
      ),
    },
    {
      field: 'version',
      align: 'center',
      renderHeader: () => <CustomColumnHeading headingTitle="Version" />,
      width: 150,
      disableColumnMenu: true,
      headerAlign: 'center',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="version"
        >
          {params.value}
        </StyledLink>
      ),
      sortComparator: (a, b) => {
        if (gte(a, b)) return 1
        return -1
      },
    },
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      width: 180,
      disableColumnMenu: true,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Box
          onClick={() => handleSearch(params.value as string)}
          sx={{ justifyContent: 'flex-start', cursor: 'pointer' }}
          data-testid="location-button"
        >
          <Tooltip text={params.value} id="gateway-location-text">
            <Box
              sx={{
                overflow: 'hidden',
                whiteSpace: 'nowrap',
                textOverflow: 'ellipsis',
              }}
            >
              {params.value}
            </Box>
          </Tooltip>
        </Box>
      ),
    },
    {
      field: 'host',
      renderHeader: () => <CustomColumnHeading headingTitle="IP:Port" />,
      width: 180,
      disableColumnMenu: true,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="host"
        >
          {params.value}
        </StyledLink>
      ),
    },
    {
      field: 'owner',
      headerName: 'Owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 180,
      disableColumnMenu: true,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`${NYM_BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          data-testid="owner"
        >
          {splice(7, 29, params.value)}
        </StyledLink>
      ),
    },
    {
      field: 'bond',
      width: 150,
      disableColumnMenu: true,
      type: 'number',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      headerClassName: 'MuiDataGrid-header-override',
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <StyledLink
          to={`/network-components/gateway/${params.row.identity_key}`}
          data-testid="pledge-amount"
        >
          {`${unymToNym(params.value, 6)}`}
        </StyledLink>
      ),
    },
  ]

  const table = useMaterialReactTable({
    columns,
    data,
  })

  return (
    <>
      <Box mb={2}>
        <Title text="Gateways" />
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
                <VersionDisplaySelector
                  handleChange={(option) => setVersionFilter(option)}
                  selected={versionFilter}
                />
              }
            />
            <MaterialReactTable table={table} />
          </Card>
        </Grid>
      </Grid>
    </>
  )
}

export default PageGateways

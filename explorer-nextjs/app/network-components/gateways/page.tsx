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
import { EXPLORER_FOR_ACCOUNTS } from '@/app/api/constants'
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
import {LocatedGateway} from "@/app/typeDefs/explorer-api";

const gatewaySanitize = (g?: LocatedGateway): boolean => {
  if(!g) {
    return false;
  }

  if(!g.gateway.version || !g.gateway.version.trim().length) {
    return false;
  }

  if(g.gateway.version === "null") {
    return false;
  }

  return true;
}

const PageGateways = () => {
  const { gateways } = useMainContext()
  const [versionFilter, setVersionFilter] =
    React.useState<VersionSelectOptions>(VersionSelectOptions.all)

  const theme = useTheme()

  const highestVersion = React.useMemo(() => {
    if (gateways?.data) {
      const versions = gateways.data.filter(gatewaySanitize).reduce(
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
    const filtered = gateways?.data?.filter(gatewaySanitize).filter((gw) => {
      const versionDiff = diff(highestVersion, gw.gateway.version)
      return versionDiff === 'patch' || versionDiff === null
    })
    if (filtered) return filtered
    return []
  }, [gateways])

  const filterByOlderVersions = React.useMemo(() => {
    const filtered = gateways?.data?.filter(gatewaySanitize).filter((gw) => {
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
        header: 'Gateways Data',
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
                    to={`/network-components/gateways/${row.original.identity_key}`}
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
            id: 'version',
            header: 'Version',
            accessorKey: 'version',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/gateways/${row.original.identity_key}`}
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
                  to={`/network-components/gateways/${row.original.identity_key}`}
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
                  to={`${EXPLORER_FOR_ACCOUNTS}/account/${row.original.owner}`}
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

  const table = useMaterialReactTable({
    columns,
    data,
  })

  return (
    <>
      <Box mb={2}>
        <Title text="Legacy Gateways" />
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

'use client'

import React, { useMemo } from 'react'
import { Box, Card, Grid, Stack, Chip } from '@mui/material'
import { useTheme } from '@mui/material/styles'
import {
  MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from 'material-react-table'
import { diff, gte, rcompare } from 'semver'
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid'
import { CopyToClipboard } from '@nymproject/react/clipboard/CopyToClipboard'
import { Tooltip as InfoTooltip } from '@nymproject/react/tooltip/Tooltip'
import { useMainContext } from '@/app/context/main'
import { CustomColumnHeading } from '@/app/components/CustomColumnHeading'
import { Title } from '@/app/components/Title'
import { unymToNym } from '@/app/utils/currency'
import { Tooltip } from '@/app/components/Tooltip'
import { EXPLORER_FOR_ACCOUNTS } from '@/app/api/constants'
import { splice } from '@/app/utils'

import StyledLink from '@/app/components/StyledLink'
import {DeclaredRole} from "@/app/network-components/nodes/DeclaredRole";

function getFlagEmoji(countryCode: string) {
  const codePoints = countryCode
    .toUpperCase()
    .split('')
    .map(char =>  127397 + char.charCodeAt(0));
  return String.fromCodePoint(...codePoints);
}

const PageNodes = () => {
  const [isLoading, setLoading] = React.useState(true);
  const { nodes, fetchNodes } = useMainContext()

  React.useEffect(() => {
    (async () => {
      try {
        await fetchNodes();
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const columns = useMemo<MRT_ColumnDef<any>[]>(() => {
    return [
      {
        id: 'nym-node-data',
        header: 'Nym Node Data',
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
                    value={row.original.bond_information.node.identity_key}
                    tooltip={`Copy identity key ${row.original.bond_information.node.identity_key} to clipboard`}
                  />
                  <StyledLink
                    to={`/network-components/nodes/${row.original.node_id}`}
                    dataTestId="identity-link"
                    color="text.primary"
                  >
                    {splice(7, 29, row.original.bond_information.node.identity_key)}
                  </StyledLink>
                </Stack>
              )
            },
          },
          {
            id: 'version',
            header: 'Version',
            accessorKey: 'description.build_information.build_version',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/nodes/${row.original.node_id}`}
                  data-testid="version"
                  color="text.primary"
                >
                  {row.original.description?.build_information?.build_version || "-"}
                </StyledLink>
              )
            },
          },
          {
            id: 'contract_node_type',
            header: 'Kind',
            accessorKey: 'contract_node_type',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/nodes/${row.original.node_id}`}
                  data-testid="version"
                  color="text.primary"
                >
                  <code>{row.original.contract_node_type || "-"}</code>
                </StyledLink>
              )
            },
          },
          {
            id: 'declared_role',
            header: 'Declare Role',
            accessorKey: 'description.declared_role',
            size: 250,
            Cell: ({ row }) => {
              return (
                <Box
                  sx={{ justifyContent: 'flex-start', cursor: 'pointer' }}
                  data-testid="location-button"
                >
                  <DeclaredRole declared_role={row.original.description?.declared_role}/>
                </Box>
              )
            },
          },
          {
            id: 'location',
            header: 'Location',
            accessorKey: 'location.country_name',
            size: 150,
            Cell: ({ row }) => {
              return (
                <Box
                  sx={{ justifyContent: 'flex-start', cursor: 'pointer' }}
                  data-testid="location-button"
                >
                  <Tooltip
                    text={row.original.location?.country_name || "-"}
                    id="nym-node-location-text"
                  >
                    <Box
                      sx={{
                        overflow: 'hidden',
                        whiteSpace: 'nowrap',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      {row.original.location?.country_name ? <>{getFlagEmoji(row.original.location.two_letter_iso_country_code.toUpperCase())}&nbsp;{row.original.location.two_letter_iso_country_code}</> : <>-</> }
                    </Box>
                  </Tooltip>
                </Box>
              )
            },
          },
          {
            id: 'host',
            header: 'IP',
            accessorKey: 'bond_information.node.host',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`/network-components/nodes/${row.original.node_id}`}
                  data-testid="host"
                  color="text.primary"
                >
                  {row.original.bond_information?.node?.host || "-"}
                </StyledLink>
              )
            },
          },
          {
            id: 'owner',
            header: 'Owner',
            accessorKey: 'bond_information.owner',
            size: 150,
            Cell: ({ row }) => {
              return (
                <StyledLink
                  to={`${EXPLORER_FOR_ACCOUNTS}/account/${row.original.bond_information?.owner || "-"}`}
                  target="_blank"
                  data-testid="bond_information.node.owner"
                  color="text.primary"
                >
                  {splice(7, 29, row.original.bond_information?.owner)}
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
    data: nodes?.data || [],
    state: {
      isLoading,
      showLoadingOverlay: isLoading,
    },
    initialState: {
      isLoading: true,
      showLoadingOverlay: true,
    }
  })

  return (
    <>
      <Box mb={2}>
        <Title text="Nym Nodes" />
      </Box>
      <Grid container>
        <Grid item xs={12}>
          <Card
            sx={{
              padding: 2,
              height: '100%',
            }}
          >
            <MaterialReactTable table={table} />
          </Card>
        </Grid>
      </Grid>
    </>
  )
}

export default PageNodes

'use client'

import React, { useMemo } from 'react'
import {
  Box,
  Button,
  Card,
  FormControl,
  Grid,
  ListItem,
  Menu,
  Typography,
} from '@mui/material'
import { TableToolbar } from '@/app/components/TableToolbar'
import { Title } from '@/app/components/Title'
import { useMainContext } from '@/app/context/main'
import { CustomColumnHeading } from '@/app/components/CustomColumnHeading'
import {
  MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from 'material-react-table'
import { DirectoryServiceProvider } from '@/app/typeDefs/explorer-api'

const SupportedApps = () => {
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)
  const open = Boolean(anchorEl)
  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setAnchorEl(event.currentTarget)
  }
  const handleClose = () => {
    setAnchorEl(null)
  }
  const anchorRef = React.useRef<HTMLButtonElement>(null)

  return (
    <FormControl size="small">
      <Button
        ref={anchorRef}
        onClick={handleClick}
        size="large"
        variant="outlined"
        color="inherit"
        sx={{ mr: 2, textTransform: 'capitalize' }}
      >
        Supported Apps
      </Button>
      <Menu anchorEl={anchorEl} open={open} onClose={handleClose}>
        <ListItem>Keybase</ListItem>
        <ListItem>Telegram</ListItem>
        <ListItem>Electrum</ListItem>
        <ListItem>Blockstream Green</ListItem>
      </Menu>
    </FormControl>
  )
}

const ServiceProviders = () => {
  const { serviceProviders } = useMainContext()

  const columns = useMemo<MRT_ColumnDef<DirectoryServiceProvider>[]>(() => {
    return [
      {
        id: 'service-providers-data',
        header: 'Service Providers Data',
        columns: [
          {
            id: 'address',
            accessorKey: 'address',
            header: 'Client ID',
            size: 450,
          },
          {
            id: 'service_type-type',
            accessorKey: 'service_type',
            header: 'Type',
            size: 100,
          },
          {
            id: 'routing_score-score',
            accessorKey: 'routing_score',
            header: 'Routing score',
            Header() {
              return (
                <CustomColumnHeading
                  headingTitle="Routing score"
                  tooltipInfo="Routing score is only displayed for the service providers that had a successful ping within the last two hours"
                />
              )
            },
            Cell({ row }) {
              return row.original.routing_score || '-'
            },
          },
        ],
      },
    ]
  }, [])

  const table = useMaterialReactTable({
    columns,
    data: serviceProviders?.data || [],
    layoutMode: 'grid',
    state: {
      isLoading: serviceProviders?.isLoading,
    },
    initialState: {
      sorting: [
        {
          id: 'routing_score',
          desc: true,
        },
      ],
    },
  })

  return (
    <>
      <Box mb={2}>
        <Title text="Service Providers" />
      </Box>
      <Grid container>
        <Grid item xs={12}>
          <Card
            sx={{
              padding: 2,
            }}
          >
            <>
              <TableToolbar childrenBefore={<SupportedApps />} />
              <MaterialReactTable table={table} />
            </>
          </Card>
        </Grid>
      </Grid>
    </>
  )
}

export default ServiceProviders

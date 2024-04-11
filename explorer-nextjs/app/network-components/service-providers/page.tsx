'use client'

import React from 'react'
import {
  Box,
  Button,
  Card,
  FormControl,
  Grid,
  ListItem,
  Menu,
  SelectChangeEvent,
  Typography,
} from '@mui/material'
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid'
import { TableToolbar } from '@/app/components/TableToolbar'
import { Title } from '@/app/components/Title'
import { UniversalDataGrid } from '@/app/components/Universal-DataGrid'
import { useMainContext } from '@/app/context/main'
import { CustomColumnHeading } from '@/app/components/CustomColumnHeading'

const columns: GridColDef[] = [
  {
    headerName: 'Client ID',
    field: 'address',
    disableColumnMenu: true,
    flex: 3,
  },
  {
    headerName: 'Type',
    field: 'service_type',
    disableColumnMenu: true,
    flex: 1,
  },
  {
    headerName: 'Routing score',
    field: 'routing_score',
    disableColumnMenu: true,
    flex: 2,
    sortingOrder: ['asc', 'desc'],
    sortComparator: (a?: string, b?: string) => {
      if (!a) return -1 // Place undefined values at the end
      if (!b) return 1 // Place undefined values at the end

      const aToNum = parseInt(a, 10)
      const bToNum = parseInt(b, 10)

      if (aToNum > bToNum) return 1

      return -1 // Sort numbers in ascending order
    },
    renderCell: (params: GridRenderCellParams) =>
      !params.value ? '-' : params.value,
    renderHeader: () => (
      <CustomColumnHeading
        headingTitle="Routing score"
        tooltipInfo="Routing score is only displayed for the service providers that had a successful ping within the last two hours"
      />
    ),
  },
]

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
  const [pageSize, setPageSize] = React.useState('10')

  const handleOnPageSizeChange = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value)
  }

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
            {serviceProviders?.data ? (
              <>
                <TableToolbar
                  onChangePageSize={handleOnPageSizeChange}
                  pageSize={pageSize}
                  childrenBefore={<SupportedApps />}
                />
                <UniversalDataGrid
                  pagination
                  rows={serviceProviders.data}
                  columns={columns}
                  pageSize={pageSize}
                  initialState={{
                    sorting: {
                      sortModel: [
                        {
                          field: 'routing_score',
                          sort: 'desc',
                        },
                      ],
                    },
                  }}
                />
              </>
            ) : (
              <Typography>No service providers to display</Typography>
            )}
          </Card>
        </Grid>
      </Grid>
    </>
  )
}

export default ServiceProviders

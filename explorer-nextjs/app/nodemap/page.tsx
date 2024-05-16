'use client'

import React, { useMemo } from 'react'
import {
  Alert,
  Box,
  CircularProgress,
  Grid,
  SelectChangeEvent,
  Typography,
} from '@mui/material'
import { ContentCard } from '@/app/components/ContentCard'
import { TableToolbar } from '@/app/components/TableToolbar'
import { Title } from '@/app/components/Title'
import { WorldMap } from '@/app/components/WorldMap'
import { useMainContext } from '@/app/context/main'
import { CountryDataRowType, countryDataToGridRow } from '@/app/utils'
import {
  MRT_ColumnDef,
  MaterialReactTable,
  useMaterialReactTable,
} from 'material-react-table'

const PageMixnodesMap = () => {
  const { countryData } = useMainContext()

  const data = useMemo(() => {
    return countryDataToGridRow(Object.values(countryData?.data || {}))
  }, [countryData])

  const columns = useMemo<MRT_ColumnDef<CountryDataRowType>[]>(() => {
    return [
      {
        id: 'delegations-data',
        header: 'Global Mixnodes Data',
        columns: [
          {
            id: 'country-name',
            header: 'Location',
            accessorKey: 'countryName',
          },
          {
            id: 'nodes',
            header: 'Nodes',
            accessorKey: 'nodes',
          },
          {
            id: 'percentage',
            header: 'Percentage',
            accessorKey: 'percentage',
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
    <Box component="main" sx={{ flexGrow: 1 }}>
      <Grid>
        <Grid item data-testid="mixnodes-globe">
          <Title text="Mixnodes Around the Globe" />
        </Grid>
        <Grid item>
          <Grid container spacing={2}>
            <Grid item xs={12}>
              <ContentCard title="Distribution of nodes">
                <WorldMap loading={false} countryData={countryData} />
                <Box sx={{ marginTop: 2 }} />
                <TableToolbar />
                <MaterialReactTable table={table} />
              </ContentCard>
            </Grid>
          </Grid>
        </Grid>
      </Grid>
    </Box>
  )
}

export default PageMixnodesMap

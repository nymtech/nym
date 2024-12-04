'use client'

import * as React from 'react'
import { makeStyles } from '@mui/styles'
import {
  DataGrid,
  GridColDef,
  GridEventListener,
  useGridApiContext,
} from '@mui/x-data-grid'
import Pagination from '@mui/material/Pagination'
import { LinearProgress } from '@mui/material'
import { GridInitialStateCommunity } from '@mui/x-data-grid/models/gridStateCommunity'

const useStyles = makeStyles({
  root: {
    display: 'flex',
  },
})

const CustomPagination = () => {
  const apiRef = useGridApiContext()
  const classes = useStyles()
  console.log(apiRef.current.state)

  return (
    <Pagination
      className={classes.root}
      sx={{ mt: 2 }}
      color="primary"
      count={apiRef.current.state.pagination.paginationModel.pageSize}
      page={apiRef.current.state.pagination.paginationModel.page + 1}
      onChange={(_, value) => apiRef.current.setPage(value - 1)}
    />
  )
}

type DataGridProps = {
  columns: GridColDef[]
  pagination?: true | undefined
  pageSize?: string | undefined
  rows: any
  loading?: boolean
  initialState?: GridInitialStateCommunity
  onRowClick?: GridEventListener<'rowClick'> | undefined
}
export const UniversalDataGrid: FCWithChildren<DataGridProps> = ({
  rows,
  columns,
  loading,
  pagination,
  pageSize,
  initialState,
  onRowClick,
}) => {
  if (loading) return <LinearProgress />

  return (
    <DataGrid
      onRowClick={onRowClick}
      pagination={pagination}
      rows={rows}
      slots={{
        pagination: CustomPagination,
      }}
      columns={columns}
      autoHeight
      hideFooter={!pagination}
      initialState={initialState}
      style={{
        width: '100%',
        border: 'none',
      }}
      sx={{
        '*::-webkit-scrollbar': {
          width: '1em',
        },
        '*::-webkit-scrollbar-track': {
          background: (t) => t.palette.nym.networkExplorer.scroll.backgroud,
          outline: (t) =>
            `1px solid ${t.palette.nym.networkExplorer.scroll.border}`,
          boxShadow: 'auto',
          borderRadius: 'auto',
        },
        '*::-webkit-scrollbar-thumb': {
          backgroundColor: (t) => t.palette.nym.networkExplorer.scroll.color,
          borderRadius: '20px',
          width: '.4em',
          border: (t) =>
            `3px solid ${t.palette.nym.networkExplorer.scroll.backgroud}`,
          shadow: 'auto',
        },
      }}
    />
  )
}

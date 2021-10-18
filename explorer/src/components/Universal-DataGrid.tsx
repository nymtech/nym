import * as React from 'react';
import { DataGrid, GridColumns, GridRowData } from '@mui/x-data-grid';

type DataGridProps = {
  height: number,
  loading?: boolean,
  rows: GridRowData[],
  columnsData: GridColumns,
  pageSize?: string,
  pagination?: boolean,
}

export const cellStyles = {
  color: 'inherit',
  textDecoration: 'none',
  marginLeft: 2,
  fontWeight: 400,
  fontSize: 12,
}

export const UniversalDataGrid = ({
  loading,
  rows,
  columnsData,
  pageSize,
  pagination,
}: DataGridProps) => {

  if (columnsData && rows) {
    return (
      <DataGrid
        loading={loading}
        columns={columnsData}
        rows={rows}
        pageSize={Number(pageSize)}
        rowsPerPageOptions={[5]}
        hideFooterPagination={!pagination}
        disableColumnMenu
        autoHeight
      />
    );
  }
  return null;
};

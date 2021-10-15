import * as React from 'react';
import { DataGrid, GridColumns, GridRowData } from '@mui/x-data-grid';
import { Box } from '@mui/system';

type DataGridProps = {
  height: number,
  loading?: boolean,
  rows: GridRowData[],
  columnsData: GridColumns,
  pageSize?: string,
  pagination?: boolean,
}

export const UniversalDataGrid = ({
  height,
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

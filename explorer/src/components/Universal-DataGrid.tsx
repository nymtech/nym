import * as React from 'react';
import { DataGrid, GridColumns, GridRowData } from '@mui/x-data-grid';
import { Box } from '@mui/system';

type DataGridProps = {
  height: number,
  loading?: boolean,
  rows: GridRowData[],
  columnsData: GridColumns,
  pageSize?: string
}

export const UniversalDataGrid = ({
  height,
  loading,
  rows,
  columnsData,
  pageSize,
}: DataGridProps) => {

  if (columnsData && rows) {
    return (
      <Box sx={{ height, width: '100%' }}>
        <DataGrid
          loading={loading}
          columns={columnsData}
          rows={rows}
          pageSize={Number(pageSize)}
          rowsPerPageOptions={[5]}
          disableSelectionOnClick
        />
      </Box>
    );
  }
  return null;
};

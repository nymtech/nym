import * as React from 'react';
import { DataGrid, GridColumns, GridRenderCellParams, GridRowData } from '@mui/x-data-grid';
import { Box } from '@mui/system';

type DataGridProps = {
  height: number,
  loading?: boolean,
  rows?: GridRowData[],
  columnsData?: GridColumns
}

export const UniversalDataGrid = ({ height, loading, rows, columnsData }: DataGridProps) => {
  if (columnsData && rows) {
    return (
      <Box sx={{ height, width: '100%' }}>
        <DataGrid
          loading={loading}
          columns={columnsData}
          rows={rows}
          pageSize={50}
          rowsPerPageOptions={[5]}
          disableSelectionOnClick
        />
      </Box>
    );
  }
  return null;
};

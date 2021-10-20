import * as React from 'react';
import { DataGrid, GridColumns, GridRowData } from '@mui/x-data-grid';

type DataGridProps = {
  loading?: boolean;
  rows: GridRowData[];
  columnsData: GridColumns;
  pageSize?: string;
  pagination?: boolean;
  hideFooter?: boolean;
};

export const cellStyles = {
  width: 'auto',
  maxWidth: 170,
  maxHeight: 100,
  color: 'inherit',
  textDecoration: 'none',
  fontWeight: 400,
  fontSize: 12,
  lineHeight: 2,
  'word-break': 'break-word',
  'white-space': 'break-spaces',
};

export const UniversalDataGrid = ({
  loading,
  rows,
  columnsData,
  pageSize,
  pagination,
  hideFooter,
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
        disableColumnFilter
        disableColumnMenu
        disableSelectionOnClick
        columnBuffer={0}
        autoHeight
        hideFooter={hideFooter}
        style={{
          maxWidth: 980,
          width: '100%',
        }}
      />
    );
  }
  return null;
};

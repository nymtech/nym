import * as React from 'react';
import { makeStyles } from '@mui/styles';
import {
  DataGrid,
  GridColumns,
  GridRowData,
  useGridSlotComponentProps,
} from '@mui/x-data-grid';
import Pagination from '@mui/material/Pagination';

const useStyles = makeStyles({
  root: {
    display: 'flex',
  },
});

type DataGridProps = {
  loading?: boolean;
  rows: GridRowData[];
  columnsData: GridColumns;
  pageSize?: string;
  pagination?: boolean;
  hideFooter?: boolean;
};

export const cellStyles = {
  width: '100%',
  padding: 0,
  'text-align': 'start',
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

function CustomPagination() {
  const { state, apiRef } = useGridSlotComponentProps();
  const classes = useStyles();

  return (
    <Pagination
      className={classes.root}
      color="primary"
      count={state.pagination.pageCount}
      page={state.pagination.page + 1}
      onChange={(event, value) => apiRef.current.setPage(value - 1)}
    />
  );
}

export const UniversalDataGrid: React.FC<DataGridProps> = ({
  loading,
  rows,
  columnsData,
  pageSize,
  pagination,
  hideFooter,
}) => {
  if (columnsData && rows) {
    return (
      <DataGrid
        pagination
        components={{
          Pagination: CustomPagination,
        }}
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
          width: '100%',
          border: 'none',
        }}
      />
    );
  }
  return null;
};

UniversalDataGrid.defaultProps = {
  loading: false,
  pageSize: undefined,
  pagination: false,
  hideFooter: true,
};

import * as React from 'react';
import { makeStyles } from '@mui/styles';
import {
  DataGrid,
  GridColumns,
  GridRowModel,
  GridSortModel,
  useGridApiContext,
  useGridState,
} from '@mui/x-data-grid';
import Pagination from '@mui/material/Pagination';
import { SxProps } from '@mui/system';

const useStyles = makeStyles({
  root: {
    display: 'flex',
  },
});

type DataGridProps = {
  loading?: boolean;
  rows: GridRowModel[];
  columnsData: GridColumns;
  pageSize?: string;
  pagination?: boolean;
  hideFooter?: boolean;
  sortModel?: GridSortModel;
};

export const cellStyles: SxProps = {
  width: '100%',
  padding: 0,
  maxHeight: 100,
  color: 'inherit',
  textDecoration: 'none',
  fontWeight: 400,
  fontSize: 12,
  lineHeight: 2,
  textAlign: 'start',
  wordBreak: 'break-word',
  whiteSpace: 'break-spaces',
};

function CustomPagination() {
  const apiRef = useGridApiContext();
  const [state] = useGridState(apiRef);

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
  sortModel,
}) => {
  const [sortModelState, setSortModelState] = React.useState<
    GridSortModel | undefined
  >(sortModel);
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
        sortModel={sortModelState}
        onSortModelChange={setSortModelState}
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
  sortModel: undefined,
};

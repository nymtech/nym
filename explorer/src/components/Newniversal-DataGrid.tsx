import * as React from 'react';
import { makeStyles } from '@mui/styles';
import {
  DataGrid,
  GridColDef,
  GridColumns,
  GridRowModel,
  GridSortModel,
  useGridApiContext,
  useGridState,
} from '@mui/x-data-grid';
import Pagination from '@mui/material/Pagination';
import { SxProps } from '@mui/system';
import { LinearProgress } from '@mui/material';

const useStyles = makeStyles({
  root: {
    display: 'flex',
  },
});

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
      sx={{ mt: 2 }}
      color="primary"
      count={state.pagination.pageCount}
      page={state.pagination.page + 1}
      onChange={(event, value) => apiRef.current.setPage(value - 1)}
    />
  );
}

type DataGridProps = {
  columns: GridColDef[];
  pagination?: boolean;
  hideFooter?: boolean | undefined;
  pageSize?: string | undefined;
  rows: {
    id: number | null;
    lastName: string | null;
    firstName: string | null;
    age: number | null;
  }[];
  loading?: boolean;
};
export const NewniversalDataGrid: React.FC<DataGridProps> = ({
  rows,
  columns,
  loading,
  pagination,
  hideFooter,
  pageSize,
}) => {
  if (loading) return <LinearProgress />;
  if (!loading)
    return (
      <DataGrid
        pagination={pagination ? true : undefined}
        rows={rows}
        components={{
          Pagination: CustomPagination,
        }}
        columns={columns}
        pageSize={Number(pageSize)}
        rowsPerPageOptions={[5]}
        disableSelectionOnClick
        autoHeight
        hideFooter={hideFooter}
        style={{
          width: '100%',
          border: 'none',
        }}
      />
    );
  return null;
};

NewniversalDataGrid.defaultProps = {
  loading: false,
  pagination: false,
  hideFooter: true,
  pageSize: '10',
  //   sortModel: undefined,
};

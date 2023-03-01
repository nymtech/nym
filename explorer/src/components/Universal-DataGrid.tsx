import * as React from 'react';
import { makeStyles } from '@mui/styles';
import { DataGrid, GridColDef, useGridApiContext, useGridState } from '@mui/x-data-grid';
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

const CustomPagination = () => {
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
      onChange={(_, value) => apiRef.current.setPage(value - 1)}
    />
  );
};

type DataGridProps = {
  columns: GridColDef[];
  pagination?: true | undefined;
  pageSize?: string | undefined;
  rows: any;
  loading?: boolean;
};
export const UniversalDataGrid: FCWithChildren<DataGridProps> = ({ rows, columns, loading, pagination, pageSize }) => {
  if (loading) return <LinearProgress />;
  if (!loading)
    return (
      <DataGrid
        pagination={pagination}
        rows={rows}
        components={{
          Pagination: CustomPagination,
        }}
        columns={columns}
        pageSize={Number(pageSize)}
        disableSelectionOnClick
        autoHeight
        hideFooter={!pagination}
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
            outline: (t) => `1px solid ${t.palette.nym.networkExplorer.scroll.border}`,
            boxShadow: 'auto',
            borderRadius: 'auto',
          },
          '*::-webkit-scrollbar-thumb': {
            backgroundColor: (t) => t.palette.nym.networkExplorer.scroll.color,
            borderRadius: '20px',
            width: '.4em',
            border: (t) => `3px solid ${t.palette.nym.networkExplorer.scroll.backgroud}`,
            shadow: 'auto',
          },
        }}
      />
    );
  return null;
};

UniversalDataGrid.defaultProps = {
  loading: false,
  pagination: undefined,
  pageSize: '10',
};

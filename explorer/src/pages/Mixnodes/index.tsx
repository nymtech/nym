import React from 'react';
import { GridRenderCellParams } from '@mui/x-data-grid';
import { Link } from 'react-router-dom';
import { Typography } from '@mui/material';
import { useContext } from 'react';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';

const columns = [
  { field: 'owner', headerName: 'Owner', width: 380, },
  {
    field: 'identity_key',
    headerName: 'Identity Key',
    width: 420,
    renderCell: (params: GridRenderCellParams) => {
      return (
        <Link to={`/network-components/mixnodes/${params.value}`} style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
          {params.value}
        </Link>
      )
    }
  },
  {
    field: 'bond',
    headerName: 'Bond',
    width: 130,
  },
  {
    field: 'host',
    headerName: 'IP:Port',
    width: 130,
  },
  {
    field: 'location',
    headerName: 'Location',
    width: 120,
  },
  {
    field: 'layer',
    headerName: 'Layer',
    width: 100,
    type: 'number',
  },
];

export const PageMixnodes: React.FC = () => {
  const { mixnodes } = useContext(MainContext);
  const [rows, setRows] = React.useState();

  React.useEffect(() => {
    if (mixnodes?.data !== undefined) {
      const formattedRows = mixnodeToGridRow(mixnodes.data);
      setRows(formattedRows);
    };
  }, [mixnodes])
  return (
    <>
      <Typography sx={{ marginBottom: 1 }} variant="h5">
        Mixnodes
      </Typography>
      <UniversalDataGrid
        loading={mixnodes?.isLoading}
        columnsData={columns}
        rows={rows}
        height={1080}
      />
    </>
  );
};

import { DataGrid } from '@mui/x-data-grid';
import { Box } from '@mui/system';
import { useContext } from 'react';
import { MainContext } from 'src/context/main';

const columns = [
  { field: 'owner', headerName: 'Owner', width: 500 },
  {
    field: 'identity_key',
    headerName: 'Identity Key',
    width: 500,
  },
  {
    field: 'bond',
    headerName: 'Bond',
    width: 500,
  },
  {
    field: 'host',
    headerName: 'IP:Port',
    width: 300,
  },
  {
    field: 'location',
    headerName: 'Location',
    width: 150,
  },
  {
    field: 'layer',
    headerName: 'Layer',
    width: 100,
    type: 'number',
  },
];

export const MixnodesDataGrid = () => {
  const { mixnodes } = useContext(MainContext);
  return (
    <Box sx={{ height: 1080, width: '100%' }}>
      <DataGrid
        loading={mixnodes?.isLoading}
        columns={columns}
        rows={
          mixnodes?.data?.map((m) => ({
            id: m.owner,
            owner: m.owner,
            location: m.location?.country_name || '',
            identity_key: m.mix_node.identity_key || '',
            bond: m.bond_amount.amount || '',
            host: m.mix_node.host || '',
            layer: m.layer || '',
          })) || []
        }
        pageSize={50}
        rowsPerPageOptions={[5]}
        disableSelectionOnClick
      />
    </Box>
  );
  return null;
};

import React from 'react';
import { GridRenderCellParams } from '@mui/x-data-grid';
import { Link } from 'react-router-dom';
import { Typography } from '@mui/material';
import { SelectChangeEvent } from '@mui/material/Select';
import { useContext } from 'react';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';
import { TableToolbar } from 'src/components/TableToolbar';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';

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
  const [filteredMixnodes, setFilteredMixnodes] = React.useState<MixNodeResponse>([])
  const [pageSize, setPageSize] = React.useState<string>("50");

  const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
    const st = event.target.value.toLowerCase();
    if (st === '' && mixnodes?.data) {
      setFilteredMixnodes(mixnodes?.data)
    } else {
      const filtered = mixnodes?.data?.filter((m) => {
        if (
          m.location?.country_name.toLowerCase().includes(st) ||
          m.mix_node.identity_key.toLocaleLowerCase().includes(st) ||
          m.owner.toLowerCase().includes(st)
        ) {
          return m;
        }
      })
      if (filtered) {
        setFilteredMixnodes(filtered)
      }
    }
  }

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  React.useEffect(() => {
    if (mixnodes?.data) {
      setFilteredMixnodes(mixnodes?.data)
    }
  }, [mixnodes]);

  return (
    <>
      <Typography sx={{ marginBottom: 1 }} variant="h5">
        Mixnodes
      </Typography>
      <TableToolbar
        onChangeSearch={handleSearch}
        onChangePageSize={handlePageSize}
        pageSize={pageSize}
      />
      <UniversalDataGrid
        loading={mixnodes?.isLoading}
        columnsData={columns}
        rows={mixnodeToGridRow(filteredMixnodes)}
        height={1080}
        pageSize={pageSize}
      />
    </>
  );
};

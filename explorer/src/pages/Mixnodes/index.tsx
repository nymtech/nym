import React from 'react';
import { GridRenderCellParams, GridRowData } from '@mui/x-data-grid';
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
  const { mixnodes, filterMixnodes, fetchMixnodes } = useContext(MainContext);
  const [rows, setRows] = React.useState<GridRowData[]>();
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const [pageSize, setPageSize] = React.useState<string>("50");

  const resetGrid = async () => {
    if (mixnodes?.data) {
      const formattedRows = mixnodeToGridRow(mixnodes?.data);
      setRows(formattedRows);
    }
  }
  const filterBySearch = () => {
    if (mixnodes?.data && searchTerm !== '') {
      let results: MixNodeResponse = [];
      mixnodes?.data.forEach((mn, i) => {
        if (mn.location && mn.location.country_name) {
          const cn = mn.location.country_name.toLowerCase();
          const idKey = mn.mix_node.identity_key.toLowerCase();
          const ownr = mn.owner.toLowerCase();
          if (
            idKey.includes(searchTerm) ||
            cn.includes(searchTerm) ||
            ownr.includes(searchTerm)
          ) {

            results.push(mn);
          }
        }
      });
      filterMixnodes(results)
    }
  }

  const resetAndRefetchData = () => {
    resetGrid();
    fetchMixnodes()
  }

  const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
    const st = event.target.value;
    if (mixnodes?.data && st === '') {
      resetAndRefetchData();
    } else if (mixnodes?.data && st !== '') {
      setSearchTerm(st.toLowerCase());
    }
  }

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  React.useEffect(() => {
    resetGrid();
  }, [mixnodes]);

  React.useEffect(() => {
    filterBySearch();
    if (searchTerm === '') {
      resetAndRefetchData();
    }
  }, [searchTerm]);

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
        rows={rows}
        height={1080}
        pageSize={pageSize}
      />
    </>
  );
};

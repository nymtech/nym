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

export const PageMixnodes: React.FC = () => {
  const { mixnodes } = useContext(MainContext);
  const [filteredMixnodes, setFilteredMixnodes] = React.useState<MixNodeResponse>([])
  const [pageSize, setPageSize] = React.useState<string>("50");
  const [searchTerm, setSearchTerm] = React.useState<string>('');

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase())
  }

  React.useEffect(() => {
    if (searchTerm === '' && mixnodes?.data) {
      setFilteredMixnodes(mixnodes?.data)
    } else {
      const filtered = mixnodes?.data?.filter((m) => {
        if (
          m.location?.country_name.toLowerCase().includes(searchTerm) ||
          m.mix_node.identity_key.toLocaleLowerCase().includes(searchTerm) ||
          m.owner.toLowerCase().includes(searchTerm)
        ) {
          return m;
        }
      })
      if (filtered) {
        setFilteredMixnodes(filtered)
      }
    }
  }, [searchTerm, mixnodes?.data])

  const columns = [
    {
      field: 'owner',
      headerName: 'Owner',
      width: 380,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <a href={`https://testnet-milhon-blocks.nymtech.net/account/${params.value}`} target='_blank' style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
            {params.value}
          </a>
        )
      }
    },
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
      renderCell: (params: GridRenderCellParams) => {
        return (
          <Link to={`/network-components/mixnodes/${params.row.identity_key}`} style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
            {params.value}
          </Link>
        )
      }
    },
    {
      field: 'host',
      headerName: 'IP:Port',
      width: 130,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <Link to={`/network-components/mixnodes/${params.row.identity_key}`} style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
            {params.value}
          </Link>
        )
      }
    },
    {
      field: 'location',
      headerName: 'Location',
      width: 120,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <div onClick={() => handleSearch(params.value as string)} style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
            {params.value}
          </div>
        )
      }
    },
    {
      field: 'layer',
      headerName: 'Layer',
      width: 100,
      type: 'number',
      renderCell: (params: GridRenderCellParams) => {
        return (
          <Link to={`/network-components/mixnodes/${params.row.identity_key}`} style={{ textDecoration: 'none', color: 'white', marginLeft: 16 }}>
            {params.value}
          </Link>
        )
      }
    },
  ];

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
        searchTerm={searchTerm}
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

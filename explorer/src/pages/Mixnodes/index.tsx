import React from 'react';
import { printableCoin } from '@nymproject/nym-validator-client';
import { GridRenderCellParams } from '@mui/x-data-grid';
import { Link as RRDLink } from 'react-router-dom';
import { Link as MuiLink } from '@mui/material';
import { Typography } from '@mui/material';
import { SelectChangeEvent } from '@mui/material/Select';
import { UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';
import { TableToolbar } from 'src/components/TableToolbar';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';
import { BIG_DIPPER } from 'src/api/constants';

export const PageMixnodes: React.FC = () => {
  const { mixnodes } = React.useContext(MainContext);
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

  const linkStyles = {
    color: 'inherit',
    textDecoration: 'none',
    marginLeft: 2,
  }

  const columns = [
    {
      field: 'owner',
      headerName: 'Owner',
      width: 380,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink
            href={`${BIG_DIPPER}/account/${params.value}`}
            target='_blank'
            sx={linkStyles}
          >
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'identity_key',
      headerName: 'Identity Key',
      width: 420,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink sx={linkStyles} component={RRDLink} to={`/network-components/mixnodes/${params.value}`}>
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'bond',
      headerName: 'Bond',
      width: 180,
      renderCell: (params: GridRenderCellParams) => {
        const bondAsPunk = printableCoin({ amount: params.value as string, denom: 'upunk' })
        return (
          <MuiLink sx={linkStyles} component={RRDLink} to={`/network-components/mixnodes/${params.row.identity_key}`}>
            {bondAsPunk}
          </MuiLink>
        )
      }
    },
    {
      field: 'host',
      headerName: 'IP:Port',
      width: 130,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink sx={linkStyles} component={RRDLink} to={`/network-components/mixnodes/${params.row.identity_key}`}>
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'location',
      headerName: 'Location',
      width: 120,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <div
            onClick={() => handleSearch(params.value as string)}
            style={linkStyles}
          >
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
          <MuiLink sx={linkStyles} component={RRDLink} to={`/network-components/mixnodes/${params.row.identity_key}`}>
            {params.value}
          </MuiLink>
        )
      }
    },
  ];

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

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

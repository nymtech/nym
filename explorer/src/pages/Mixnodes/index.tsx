import React from 'react';
import { GridRenderCellParams, GridColumnHeaderParams, GridColDef } from '@mui/x-data-grid';
import { Link as RRDLink } from 'react-router-dom';
import { Grid, Link as MuiLink } from '@mui/material';
import { Typography } from '@mui/material';
import { SelectChangeEvent } from '@mui/material/Select';
import { useContext } from 'react';
import { cellStyles, UniversalDataGrid } from 'src/components/Universal-DataGrid';
import { MainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';
import { TableToolbar } from 'src/components/TableToolbar';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';
import { BIG_DIPPER } from 'src/api/constants';
import { ContentCard } from 'src/components/ContentCard';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';

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

  const columns: GridColDef[] = [
    {
      field: 'owner',
      renderHeader: () => <CustomColumnHeading headingTitle='Owner' />,
      width: 200,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink
            href={`${BIG_DIPPER}/account/${params.value}`}
            target='_blank'
            sx={cellStyles}
          >
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'identity_key',
      renderHeader: () => <CustomColumnHeading headingTitle='Identity Key' />,
      width: 200,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink sx={cellStyles} component={RRDLink} to={`/network-components/mixnodes/${params.value}`}>
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'bond',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle='Bond' />,
      width: 120,
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink sx={cellStyles} component={RRDLink} to={`/network-components/mixnodes/${params.row.identity_key}`}>
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'host',
      renderHeader: () => <CustomColumnHeading headingTitle='IP:Port' />,
      width: 130,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink sx={cellStyles} component={RRDLink} to={`/network-components/mixnodes/${params.row.identity_key}`}>
            {params.value}
          </MuiLink>
        )
      }
    },
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle='Location' />,
      width: 120,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => {
        return (
          <div
            onClick={() => handleSearch(params.value as string)}
            style={cellStyles}
          >
            {params.value}
          </div>
        )
      }
    },
    {
      field: 'layer',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle='Layer' />,
      width: 100,
      type: 'number',
      renderCell: (params: GridRenderCellParams) => {
        return (
          <MuiLink sx={{ ...cellStyles, textAlign: 'left' }} component={RRDLink} to={`/network-components/mixnodes/${params.row.identity_key}`}>
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
      <Typography sx={{ marginBottom: 3 }} variant="h5">
        Mixnodes
      </Typography>

      <Grid container>
        <Grid item xs={12} md={12} lg={12} xl={8}>
          <ContentCard>
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
              pageSize={pageSize}
              pagination
            />
          </ContentCard>

        </Grid>
      </Grid>
    </>
  );
};

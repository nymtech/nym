import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { printableCoin } from '@nymproject/nym-validator-client';
import { Link as RRDLink } from 'react-router-dom';
import { Button, Grid, Link as MuiLink } from '@mui/material';
import { SelectChangeEvent } from '@mui/material/Select';
import {
  cellStyles,
  UniversalDataGrid,
} from 'src/components/Universal-DataGrid';
import { useMainContext } from 'src/context/main';
import { mixnodeToGridRow } from 'src/utils';
import { TableToolbar } from 'src/components/TableToolbar';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';
import { BIG_DIPPER } from 'src/api/constants';
import { ContentCard } from 'src/components/ContentCard';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { Title } from 'src/components/Title';

export const PageMixnodes: React.FC = () => {
  const { mixnodes } = useMainContext();
  const [filteredMixnodes, setFilteredMixnodes] =
    React.useState<MixNodeResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [searchTerm, setSearchTerm] = React.useState<string>('');

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase());
  };

  React.useEffect(() => {
    if (searchTerm === '' && mixnodes?.data) {
      setFilteredMixnodes(mixnodes?.data);
    } else {
      const filtered = mixnodes?.data?.filter((m) => {
        if (
          m.location?.country_name.toLowerCase().includes(searchTerm) ||
          m.mix_node.identity_key.toLocaleLowerCase().includes(searchTerm) ||
          m.owner.toLowerCase().includes(searchTerm)
        ) {
          return m;
        }
        return null;
      });
      if (filtered) {
        setFilteredMixnodes(filtered);
      }
    }
  }, [searchTerm, mixnodes?.data]);

  const columns: GridColDef[] = [
    {
      field: 'owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      flex: 3,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          href={`${BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          sx={cellStyles}
          data-testid="big-dipper-link"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'identity_key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      flex: 3,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={cellStyles}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.value}`}
          data-testid="identity-link"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'bond',
      headerName: 'Bond',
      type: 'number',
      headerAlign: 'left',
      flex: 1,
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      renderCell: (params: GridRenderCellParams) => {
        const bondAsPunk = printableCoin({
          amount: params.value as string,
          denom: 'upunk',
        });
        return (
          <MuiLink
            sx={cellStyles}
            component={RRDLink}
            to={`/network-components/mixnodes/${params.row.identity_key}`}
          >
            {bondAsPunk}
          </MuiLink>
        );
      },
    },
    {
      field: 'self_percentage',
      headerName: 'Self %',
      headerAlign: 'left',
      type: 'number',
      width: 99,
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Self %" />,
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={cellStyles}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}%
        </MuiLink>
      ),
    },
    {
      field: 'host',
      renderHeader: () => <CustomColumnHeading headingTitle="Host" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={cellStyles}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Button
          onClick={() => handleSearch(params.value as string)}
          sx={{ ...cellStyles, justifyContent: 'flex-start' }}
        >
          {params.value}
        </Button>
      ),
    },
    {
      field: 'layer',
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Layer" />,
      flex: 1,
      type: 'number',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...cellStyles, textAlign: 'left' }}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
  ];

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  return (
    <>
      <Title text="Mixnodes" />
      <Grid>
        <Grid item>
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
              hideFooter={false}
              sortModel={[
                {
                  field: 'bond',
                  sort: 'desc',
                },
              ]}
            />
          </ContentCard>
        </Grid>
      </Grid>
    </>
  );
};

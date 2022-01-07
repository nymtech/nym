import * as React from 'react';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { Button, Card, Grid, Link as MuiLink } from '@mui/material';
import { Link as RRDLink } from 'react-router-dom';
import { SelectChangeEvent } from '@mui/material/Select';
import { useMainContext } from 'src/context/main';
import { MixnodeRowType, mixnodeToGridRow } from 'src/components/MixNodes';
import { TableToolbar } from 'src/components/TableToolbar';
import { MixNodeResponse } from 'src/typeDefs/explorer-api';
import { BIG_DIPPER } from 'src/api/constants';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { Title } from 'src/components/Title';
import {
  cellStyles,
  UniversalDataGrid,
} from 'src/components/Universal-DataGrid';
import { SxProps } from '@mui/system';
import { Theme, useTheme } from '@mui/material/styles';
import { currencyToString } from '../../utils/currency';
import { getMixNodeStatusColor } from '../../components/MixNodes/Status';

export const PageMixnodes: React.FC = () => {
  const { mixnodes } = useMainContext();
  const [filteredMixnodes, setFilteredMixnodes] =
    React.useState<MixNodeResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [searchTerm, setSearchTerm] = React.useState<string>('');
  const theme = useTheme();

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
      headerName: 'Owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 380,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          href={`${BIG_DIPPER}/account/${params.value}`}
          target="_blank"
          sx={getCellStyles(theme, params.row)}
          data-testid="big-dipper-link"
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'identity_key',
      headerName: 'Identity Key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 380,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
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
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      type: 'number',
      headerClassName: 'MuiDataGrid-header-override',
      width: 150,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {currencyToString(params.value)}
        </MuiLink>
      ),
    },
    {
      field: 'location',
      headerName: 'Location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      width: 150,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Button
          onClick={() => handleSearch(params.value as string)}
          sx={{
            ...getCellStyles(theme, params.row),
            justifyContent: 'flex-start',
          }}
        >
          {params.value}
        </Button>
      ),
    },
    {
      field: 'self_percentage',
      headerName: 'Self %',
      width: 110,
      headerClassName: 'MuiDataGrid-header-override',
      renderHeader: () => <CustomColumnHeading headingTitle="Self %" />,
      type: 'number',
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}%
        </MuiLink>
      ),
    },
    {
      field: 'host',
      headerName: 'Host',
      renderHeader: () => <CustomColumnHeading headingTitle="Host" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 130,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={getCellStyles(theme, params.row)}
          component={RRDLink}
          to={`/network-components/mixnodes/${params.row.identity_key}`}
        >
          {params.value}
        </MuiLink>
      ),
    },
    {
      field: 'layer',
      headerName: 'Layer',
      renderHeader: () => <CustomColumnHeading headingTitle="Layer" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 110,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <MuiLink
          sx={{ ...getCellStyles(theme, params.row), textAlign: 'left' }}
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
      <Grid container>
        <Grid item xs={12}>
          <Card
            sx={{
              padding: 2,
              height: '100%',
            }}
          >
            <TableToolbar
              onChangeSearch={handleSearch}
              onChangePageSize={handlePageSize}
              pageSize={pageSize}
              searchTerm={searchTerm}
            />
            <UniversalDataGrid
              pagination
              rows={mixnodeToGridRow(filteredMixnodes)}
              columns={columns}
              pageSize={pageSize}
            />
          </Card>
        </Grid>
      </Grid>
    </>
  );
};

const getCellStyles = (theme: Theme, row: MixnodeRowType): SxProps => {
  const color = getMixNodeStatusColor(theme, row);
  return {
    ...cellStyles,
    // TODO: should these be here, or change in `cellStyles`??
    fontWeight: 700,
    fontSize: 14,
    color,
  };
};

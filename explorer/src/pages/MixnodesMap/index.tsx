import * as React from 'react';
import {
  Box,
  CircularProgress,
  Grid,
  SelectChangeEvent,
  Typography,
  Alert,
} from '@mui/material';
import { WorldMap } from 'src/components/WorldMap';
import { MainContext } from 'src/context/main';
import {
  cellStyles,
  UniversalDataGrid,
} from 'src/components/Universal-DataGrid';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
// import { CountryData } from 'src/typeDefs/explorer-api';
import { TableToolbar } from 'src/components/TableToolbar';
import { countryDataToGridRow } from 'src/utils';
import { ContentCard } from '../../components/ContentCard';

export const PageMixnodesMap: React.FC = () => {
  const { countryData } = React.useContext(MainContext);
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [formattedCountries, setFormattedCountries] = React.useState<any>([]);
  const [searchTerm, setSearchTerm] = React.useState<string>('');

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase());
  };

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  const columns: GridColDef[] = [
    {
      field: 'countryName',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles}>{params.value}</Typography>
      ),
    },
    {
      field: 'nodes',
      renderHeader: () => (
        <CustomColumnHeading headingTitle="Number of Nodes" />
      ),
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles}>{params.value}</Typography>
      ),
    },
    {
      field: 'percentage',
      renderHeader: () => <CustomColumnHeading headingTitle="Percentage %" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles}>{params.value}</Typography>
      ),
    },
  ];

  React.useEffect(() => {
    if (countryData?.data !== undefined) {
      setFormattedCountries(countryDataToGridRow(countryData.data));
    }
  }, [countryData?.data]);

  if (countryData?.isLoading) {
    return <CircularProgress />;
  }
  if (countryData?.data && !countryData.isLoading) {
    return (
      <Box component="main" sx={{ flexGrow: 1 }}>
        <Grid container spacing={1} sx={{ mb: 4 }}>
          <Grid item xs={4}>
            <Typography sx={{ marginLeft: 3, fontSize: '24px' }}>
              Mixnodes Around the Globe
            </Typography>
          </Grid>
        </Grid>
        <Grid container spacing={2}>
          <Grid item xs={12} xl={9} sx={{ maxHeight: 500 }}>
            <ContentCard title="Distribution of nodes">
              <WorldMap loading={false} countryData={countryData} />
            </ContentCard>

            <ContentCard>
              <TableToolbar
                onChangeSearch={handleSearch}
                onChangePageSize={handlePageSize}
                pageSize={pageSize}
                searchTerm={searchTerm}
              />
              <UniversalDataGrid
                loading={countryData?.isLoading}
                columnsData={columns}
                rows={formattedCountries}
                pageSize={pageSize}
                pagination
              />
            </ContentCard>
          </Grid>
        </Grid>
      </Box>
    );
  }
  return <Alert severity="error">{countryData?.error}</Alert>;
};

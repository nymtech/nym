import * as React from 'react';
import {
  Alert,
  Box,
  CircularProgress,
  Grid,
  SelectChangeEvent,
  Typography,
} from '@mui/material';
import { WorldMap } from 'src/components/WorldMap';
import { useMainContext } from 'src/context/main';
import {
  cellStyles,
  UniversalDataGrid,
} from 'src/components/Universal-DataGrid';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { TableToolbar } from 'src/components/TableToolbar';
import { CountryDataRowType, countryDataToGridRow } from 'src/utils';
import { Title } from 'src/components/Title';
import { ContentCard } from '../../components/ContentCard';

export const PageMixnodesMap: React.FC = () => {
  const { countryData } = useMainContext();
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [formattedCountries, setFormattedCountries] = React.useState<
    CountryDataRowType[]
  >([]);
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
        <Typography sx={cellStyles} data-testid="country-name">
          {params.value}
        </Typography>
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
        <Typography sx={cellStyles} data-testid="number-of-nodes">
          {params.value}
        </Typography>
      ),
    },
    {
      field: 'percentage',
      renderHeader: () => <CustomColumnHeading headingTitle="Percentage %" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles} data-testid="percentage">
          {params.value}
        </Typography>
      ),
    },
  ];

  React.useEffect(() => {
    if (countryData?.data && searchTerm === '') {
      setFormattedCountries(
        countryDataToGridRow(Object.values(countryData.data)),
      );
    } else if (countryData?.data !== undefined && searchTerm !== '') {
      const formatted = countryDataToGridRow(Object.values(countryData?.data));
      const filtered = formatted.filter((m) => {
        if (
          m.countryName.toLowerCase().includes(searchTerm) ||
          m.ISO3.toLowerCase().includes(searchTerm)
        ) {
          return m;
        }
        return null;
      });
      if (filtered) {
        setFormattedCountries(filtered);
      }
    }
  }, [searchTerm, countryData?.data]);

  if (countryData?.isLoading) {
    return <CircularProgress />;
  }

  if (countryData?.data && !countryData.isLoading) {
    return (
      <Box component="main" sx={{ flexGrow: 1 }}>
        <Grid container spacing={1} sx={{ mb: 4 }}>
          <Grid item xs={12} data-testid="mixnodes-globe">
            <Title text="Mixnodes Around the Globe" />
          </Grid>
          <Grid item xs={12} lg={9}>
            <Grid container spacing={2}>
              <Grid item xs={12}>
                <ContentCard title="Distribution of nodes">
                  <WorldMap loading={false} countryData={countryData} />
                  <Box sx={{ marginTop: 2 }} />
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
          </Grid>
        </Grid>
      </Box>
    );
  }
  return <Alert severity="error">{countryData?.error}</Alert>;
};

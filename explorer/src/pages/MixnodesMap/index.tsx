import * as React from 'react';
import { Alert, Box, CircularProgress, Grid, SelectChangeEvent, Typography } from '@mui/material';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { ContentCard } from '../../components/ContentCard';
import { CustomColumnHeading } from '../../components/CustomColumnHeading';
import { TableToolbar } from '../../components/TableToolbar';
import { Title } from '../../components/Title';
import { UniversalDataGrid } from '../../components/Universal-DataGrid';
import { WorldMap } from '../../components/WorldMap';
import { useMainContext } from '../../context/main';
import { CountryDataRowType, countryDataToGridRow } from '../../utils';

export const PageMixnodesMap: FCWithChildren = () => {
  const { countryData } = useMainContext();
  const [pageSize, setPageSize] = React.useState<string>('10');
  const [formattedCountries, setFormattedCountries] = React.useState<CountryDataRowType[]>([]);
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
      renderCell: (params: GridRenderCellParams) => <Typography data-testid="country-name">{params.value}</Typography>,
    },
    {
      field: 'nodes',
      renderHeader: () => <CustomColumnHeading headingTitle="Number of Nodes" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography data-testid="number-of-nodes">{params.value}</Typography>
      ),
    },
    {
      field: 'percentage',
      renderHeader: () => <CustomColumnHeading headingTitle="Percentage %" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => <Typography data-testid="percentage">{params.value}</Typography>,
    },
  ];

  React.useEffect(() => {
    if (countryData?.data && searchTerm === '') {
      setFormattedCountries(countryDataToGridRow(Object.values(countryData.data)));
    } else if (countryData?.data !== undefined && searchTerm !== '') {
      const formatted = countryDataToGridRow(Object.values(countryData?.data));
      const filtered = formatted.filter(
        (m) => m?.countryName?.toLowerCase().includes(searchTerm) || m?.ISO3?.toLowerCase().includes(searchTerm),
      );
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
        <Grid>
          <Grid item data-testid="mixnodes-globe">
            <Title text="Mixnodes Around the Globe" />
          </Grid>
          <Grid item>
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
                    pagination
                    loading={countryData?.isLoading}
                    columns={columns}
                    rows={formattedCountries}
                    pageSize={pageSize}
                  />
                </ContentCard>
              </Grid>
            </Grid>
          </Grid>
        </Grid>
      </Box>
    );
  }

  if (countryData?.error) {
    return <Alert severity="error">{countryData.error.message}</Alert>;
  }

  return null;
};

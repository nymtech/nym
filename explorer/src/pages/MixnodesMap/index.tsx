import * as React from 'react';
import { Box, Grid, Typography } from '@mui/material';
import { WorldMap } from 'src/components/WorldMap';
import { MainContext } from 'src/context/main';
import {
  cellStyles,
  UniversalDataGrid,
} from 'src/components/Universal-DataGrid';
import { CustomColumnHeading } from 'src/components/CustomColumnHeading';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { ContentCard } from '../../components/ContentCard';

export const PageMixnodesMap: React.FC = () => {
  const { countryData, mode } = React.useContext(MainContext);

  const columns: GridColDef[] = [
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      flex: 1,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles}>{params.value}</Typography>
      ),
    },
  ];
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
        </Grid>
      </Grid>
      <Grid container spacing={2}>
        <Grid item xs={12} xl={9} sx={{ maxHeight: 500 }}>
          <UniversalDataGrid loading={false} columnsData={columns} rows={} />
        </Grid>
      </Grid>
    </Box>
  );
};

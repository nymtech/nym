import React from 'react';
import { styled } from '@mui/material/styles';
import { Box, Grid, Typography, Link } from '@mui/material';
import { SettingsAccessibility as ConnectIcon } from '@mui/icons-material';
import { WorldMap } from 'src/components/WorldMap';
import { Title } from '../../components/Title';
import { IconWithLink } from '../../components/IconWithLink';

// MUI Icons

const DrawerHeader = styled('div')(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'flex-end',
  padding: theme.spacing(0, 1),
  // necessary for content to be below app bar
  ...theme.mixins.toolbar,
}));

export const PageOverview: React.FC = () => (
  <>
    <Box component="main" sx={{ flexGrow: 1 }}>
      <DrawerHeader />
      <Grid
        container
        spacing={2}
        style={{ border: '1px solid red' }}
        sx={{
          height: 'auto',
          padding: (theme) => theme.spacing(4),
          background: (theme) => theme.palette.primary.dark,
        }}
      >
        <Grid
          item
          xs={12}
          sx={{
            justifyContent: 'flex-start',
            padding: (theme) => theme.spacing(2),
          }}
        >
          <Typography>Overview</Typography>
        </Grid>

        <IconWithLink text="5134 Mixnodes →" url="/foo" SVGIcon={ConnectIcon} />
        <IconWithLink text="5134 Mixnodes →" url="/foo" SVGIcon={ConnectIcon} />
        <IconWithLink text="5134 Mixnodes →" url="/foo" SVGIcon={ConnectIcon} />
        <Title text="Current block height is 647,059" />
        <WorldMap />
      </Grid>
    </Box>
  </>
);

import React from 'react';
import { styled } from '@mui/material/styles';
import { Box, Grid, Typography } from '@mui/material';
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
          <Typography
            sx={{ color: (theme) => theme.palette.primary.main, fontSize: 24 }}
          >
            Overview
          </Typography>
        </Grid>

        <IconWithLink
          id="mixn"
          text="Mixnodes →"
          apiUrl="https://testnet-milhon-explorer.nymtech.net/api/mix-node"
          linkUrl="/network-components/mixnodes"
          SVGIcon={ConnectIcon}
          errorMsg="Oh no! Mixnodes info not available right now."
        />

        <IconWithLink
          id="gate"
          text="Gateways →"
          apiUrl="https://testnet-milhon-validator1.nymtech.net/api/v1/gateways"
          linkUrl="/network-components/gateways"
          SVGIcon={ConnectIcon}
          errorMsg="Oh no! Gateways info is undergoing maintenance. Please try later."
        />

        <IconWithLink
          id="val"
          text="Validators →"
          apiUrl="https://testnet-milhon-validator1.nymtech.net/validators"
          linkUrl="https://testnet-milhon-blocks.nymtech.net/validators"
          SVGIcon={ConnectIcon}
          errorMsg="Oh no! Gateways info is undergoing maintenance. Please try later."
        />

        <Title text="Current block height is 647,059" />
        <WorldMap />
      </Grid>
    </Box>
  </>
);

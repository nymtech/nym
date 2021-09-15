import React from 'react';
import { styled } from '@mui/material/styles';
import { Box, Grid, Paper, Typography, Link } from '@mui/material';

// MUI Icons
import { SettingsAccessibility as ConnectIcon } from '@mui/icons-material';

const DrawerHeader = styled('div')(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'flex-end',
  padding: theme.spacing(0, 1),
  // necessary for content to be below app bar
  ...theme.mixins.toolbar,
}));

interface IconAndLinkProps {
  text: string;
  SVGIcon: React.FunctionComponent<any>;
  url: string;
}

interface TitleProps {
  text: string;
}

const Title = ({ text }: TitleProps) => (
  <Grid
    item
    xs={12}
    sx={{
      justifyContent: 'flex-start',
      padding: (theme) => theme.spacing(2),
      backgroundColor: (theme) => theme.palette.primary.dark,
    }}
  >
    <Box
      sx={{
        padding: (theme) => theme.spacing(3),
        backgroundColor: (theme) => theme.palette.primary.light,
      }}
    >
      <Typography
        sx={{
          color: (theme) => theme.palette.primary.main,
        }}
      >
        {text}
      </Typography>
    </Box>
  </Grid>
);

const IconAndLink = ({ text, SVGIcon, url }: IconAndLinkProps) => (
  <Grid
    item
    xs={12}
    sm={6}
    md={4}
    sx={{
      justifyContent: 'flex-start',
      padding: (theme) => theme.spacing(2),
      backgroundColor: (theme) => theme.palette.primary.dark,
    }}
  >
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'row',

        padding: (theme) => theme.spacing(3),
        backgroundColor: (theme) => theme.palette.primary.light,
      }}
    >
      <SVGIcon />
      <Link href={url}>
        <Typography sx={{ marginLeft: (theme) => theme.spacing(2) }}>
          {text}
        </Typography>
      </Link>
    </Box>
  </Grid>
);

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

        <IconAndLink text="5134 Mixnodes →" url="/foo" SVGIcon={ConnectIcon} />
        <IconAndLink text="5134 Mixnodes →" url="/foo" SVGIcon={ConnectIcon} />
        <IconAndLink text="5134 Mixnodes →" url="/foo" SVGIcon={ConnectIcon} />
        <Title text="Current block height is 647,059" />
      </Grid>
    </Box>
  </>
);

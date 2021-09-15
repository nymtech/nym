import React from 'react';
import { Box, Grid, Typography, Link } from '@mui/material';

export type IconWithLinkProps = {
  text: string;
  SVGIcon: React.FunctionComponent<any>;
  url: string;
};

export const IconWithLink: React.FC<IconWithLinkProps> = ({
  text,
  SVGIcon,
  url,
}) => (
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

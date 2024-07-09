import React from 'react';
import { Theme } from '@mui/material/styles';
import { Box, Grid, Typography } from '@mui/material';
import flatten from 'flat';

const SWATCH_SIZE = '40px';

const PaletteSwatch = ({
  theme,
  path,
  value,
  width,
}: {
  theme: Theme;
  path: string;
  value: string;
  width?: string;
  chidlren: React.ReactNode;
}) => (
  <>
    <Box
      sx={{
        mr: 2,
        height: SWATCH_SIZE,
        width: SWATCH_SIZE,
        background: value,
        border: `1px solid ${theme.palette.text.primary}`,
      }}
    />
    <Box>
      <Typography minWidth={width} maxWidth={width} fontFamily="monospace" overflow="scroll" fontSize="12px">
        {path}
      </Typography>
    </Box>
  </>
);

export const PaletteSwatches: FCWithChildren<{
  theme: Theme;
}> = ({ theme }) => {
  const swatches = React.useMemo<any>(() => flatten(theme.palette), [theme.palette]);
  return (
    <Grid container spacing={2}>
      {Object.keys(swatches)
        .filter((key) => typeof swatches[key] === 'string' && key !== 'mode')
        .map((key) => (
          <Grid item key={key}>
            <PaletteSwatch theme={theme} path={key} value={swatches[key]} width="150px" />
          </Grid>
        ))}
    </Grid>
  );
};

export const PaletteSwatchesList: FCWithChildren<{
  theme: Theme;
}> = ({ theme }) => {
  const swatches = React.useMemo<any>(() => flatten(theme.palette), [theme.palette]);
  return (
    <>
      {Object.keys(swatches)
        .filter((key) => typeof swatches[key] === 'string' && key !== 'mode')
        .map((key) => (
          <Box display="flex" alignItems="center" p={1}>
            <PaletteSwatch theme={theme} path={key} value={swatches[key]} />
          </Box>
        ))}
    </>
  );
};

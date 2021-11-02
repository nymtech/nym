import * as React from 'react';
import { Box, Typography } from '@mui/material';
import { ExpandLess, ExpandMore } from '@mui/icons-material';
import { MainContext } from 'src/context/main';

export const CustomColumnHeading: React.FC<{ headingTitle: string }> = ({
  headingTitle,
}) => {
  const { mode } = React.useContext(MainContext);
  const [filter, toggleFilter] = React.useState<boolean>(false);

  const handleClick = () => {
    toggleFilter(!filter);
  };
  return (
    <Box alignItems="center" display="flex" onClick={handleClick}>
      <Typography
        sx={{
          fontWeight: 'bold',
          color: (theme) =>
            mode === 'dark'
              ? theme.palette.primary.main
              : theme.palette.secondary.main,
          fontSize: 14,
          padding: 0,
        }}
        data-testid={headingTitle}
      >
        {headingTitle}&nbsp;
      </Typography>
      {filter ? <ExpandMore color="primary" /> : <ExpandLess color="primary" />}
    </Box>
  );
};

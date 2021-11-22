import * as React from 'react';
import { Box, Typography } from '@mui/material';
import { ExpandLess, ExpandMore } from '@mui/icons-material';

export const CustomColumnHeading: React.FC<{ headingTitle: string }> = ({
  headingTitle,
}) => {
  const [filter, toggleFilter] = React.useState<boolean>(false);

  const handleClick = () => {
    toggleFilter(!filter);
  };
  return (
    <Box alignItems="center" display="flex" onClick={handleClick}>
      <Typography
        sx={{
          fontWeight: 'bold',
          fontSize: 14,
          padding: 0,
        }}
        data-testid={headingTitle}
      >
        {headingTitle}&nbsp;
      </Typography>
      {filter ? <ExpandMore /> : <ExpandLess />}
    </Box>
  );
};

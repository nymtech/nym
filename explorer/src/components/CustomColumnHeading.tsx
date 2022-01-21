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
          fontWeight: 600,
          fontSize: 14,
          padding: 0,
          // border: '1px solid red',
          // minWidth: 300,
        }}
        data-testid={headingTitle}
      >
        {headingTitle}&nbsp;
      </Typography>
      {filter ? <ExpandMore /> : <ExpandLess />}
    </Box>
  );
};

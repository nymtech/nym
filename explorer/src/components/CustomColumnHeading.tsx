import * as React from 'react';
import { Box, Typography } from '@mui/material';
import { ExpandLess, ExpandMore } from '@mui/icons-material';
import { useTheme } from '@mui/material/styles';
import { Tooltip } from '@nymproject/react/tooltip/Tooltip';

export const CustomColumnHeading: React.FC<{ headingTitle: string; tooltipInfo?: string }> = ({
  headingTitle,
  tooltipInfo,
}) => {
  const [filter, toggleFilter] = React.useState<boolean>(false);
  const theme = useTheme();

  const handleClick = () => {
    toggleFilter(!filter);
  };
  return (
    <Box alignItems="center" display="flex" onClick={handleClick}>
      {tooltipInfo && (
        <Tooltip
          title={tooltipInfo}
          id={headingTitle}
          placement="top-start"
          textColor={theme.palette.nym.networkExplorer.tooltip.color}
          bgColor={theme.palette.nym.networkExplorer.tooltip.background}
          maxWidth={230}
          arrow
        />
      )}
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

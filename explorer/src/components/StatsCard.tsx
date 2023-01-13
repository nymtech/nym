import * as React from 'react';
import { Box, Card, CardContent, IconButton, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import EastIcon from '@mui/icons-material/East';

interface StatsCardProps {
  icon: React.ReactNode;
  title: string;
  count?: string | number;
  errorMsg?: Error | string;
  onClick?: () => void;
  color?: string;
}
export const StatsCard: FCWithChildren<StatsCardProps> = ({
  icon,
  title,
  count,
  onClick,
  errorMsg,
  color: colorProp,
}) => {
  const theme = useTheme();
  const color = colorProp || theme.palette.text.primary;
  return (
    <Card onClick={onClick} sx={{ height: '100%' }}>
      <CardContent
        sx={{
          padding: 1.5,
          paddingLeft: 3,
          '&:last-child': {
            paddingBottom: 1.5,
          },
          cursor: 'pointer',
          fontSize: 14,
          fontWeight: 600,
        }}
      >
        <Box display="flex" alignItems="center" color={color}>
          <Box display="flex">
            {icon}
            <Typography ml={3} mr={0.75} fontSize="inherit" fontWeight="inherit" data-testid={`${title}-amount`}>
              {count === undefined || count === null ? '' : count}
            </Typography>
            <Typography mr={1} fontSize="inherit" fontWeight="inherit" data-testid={title}>
              {title}
            </Typography>
          </Box>
          <IconButton color="inherit" sx={{ fontSize: '16px' }}>
            <EastIcon fontSize="inherit" />
          </IconButton>
        </Box>
        {errorMsg && (
          <Typography variant="body2" sx={{ color: 'danger', padding: 2 }}>
            {typeof errorMsg === 'string' ? errorMsg : errorMsg.message || 'Oh no! An error occurred'}
          </Typography>
        )}
      </CardContent>
    </Card>
  );
};

StatsCard.defaultProps = {
  onClick: undefined,
  errorMsg: undefined,
  color: undefined,
  count: undefined,
};

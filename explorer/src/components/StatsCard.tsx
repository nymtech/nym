import * as React from 'react';
import { Box, Card, CardContent, IconButton, Typography } from '@mui/material';
import ArrowRightAltIcon from '@mui/icons-material/ArrowRightAlt';

interface StatsCardProps {
  icon: React.ReactNode;
  title: string;
  count: string | number;
  errorMsg?: Error | string;
  onClick?: () => void;
}
export const StatsCard: React.FC<StatsCardProps> = ({
  icon,
  title,
  count,
  onClick,
  errorMsg,
}) => (
  <Card onClick={onClick} sx={{ height: '100%' }}>
    <CardContent
      sx={{
        padding: 2,
        '&:last-child': {
          paddingBottom: 2,
        },
        cursor: 'pointer',
      }}
    >
      <Box
        display="flex"
        alignItems="center"
        sx={{ color: (theme) => theme.palette.text.primary }}
      >
        {icon}
        <Typography ml={3} mr={0.75}>
          {count}
        </Typography>
        <Typography mr={1}>{title}</Typography>
        <IconButton color="inherit">
          <ArrowRightAltIcon />
        </IconButton>
      </Box>
      {errorMsg && (
        <Typography variant="body2" sx={{ color: 'danger', padding: 2 }}>
          {errorMsg}
        </Typography>
      )}
    </CardContent>
  </Card>
);

StatsCard.defaultProps = {
  onClick: undefined,
  errorMsg: undefined,
};

import * as React from 'react';
import {
  Box,
  Card,
  CardContent,
  IconButton,
  Typography,
  useMediaQuery,
} from '@mui/material';
import { useTheme } from '@mui/material/styles';
import ArrowRightAltIcon from '@mui/icons-material/ArrowRightAlt';

interface StatsCardProps {
  icon: React.ReactNode;
  title: string;
  count?: string | number;
  errorMsg?: Error | string;
  onClick?: () => void;
  color?: string;
}
export const StatsCard: React.FC<StatsCardProps> = ({
  icon,
  title,
  count,
  onClick,
  errorMsg,
  color: colorProp,
}) => {
  const theme = useTheme();
  const matches = useMediaQuery(theme.breakpoints.down('sm'));
  const color = colorProp || theme.palette.text.primary;
  return (
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
          sx={{
            color,
            fontSize: 18,
            justifyContent: 'space-between',
          }}
        >
          <Box display="flex">
            {icon}
            <Typography
              ml={3}
              mr={0.75}
              fontSize="inherit"
              data-testid={`${title}-amount`}
            >
              {count === undefined || count === null ? '' : count}
            </Typography>
            <Typography mr={1} fontSize="inherit" data-testid={title}>
              {title}
            </Typography>
          </Box>
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
};

StatsCard.defaultProps = {
  onClick: undefined,
  errorMsg: undefined,
  color: undefined,
};

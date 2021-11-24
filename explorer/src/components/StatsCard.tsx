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
}) => {
  const theme = useTheme();
  const matches = useMediaQuery(theme.breakpoints.down('sm'));
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
            color: theme.palette.text.primary,
            fontSize: 18,
            justifyContent: matches ? 'space-between' : 'flex-start',
            maxWidth: matches ? 230 : null,
          }}
        >
          {icon}

          <Box
            sx={{
              color: theme.palette.text.primary,
              display: 'flex',
              flexDirection: 'row',
              fontSize: 18,
              justifyContent: matches ? 'space-between' : 'flex-start',
              alignItems: 'center',
              width: '100%',
              maxWidth: matches ? 230 : null,
            }}
          >
            <Typography
              ml={3}
              mr={0.75}
              fontSize="inherit"
              data-testid={`${title}-amount`}
            >
              {count}
            </Typography>
            <Typography mr={1} fontSize="inherit" data-testid={title}>
              {title}
            </Typography>
            <IconButton color="inherit">
              <ArrowRightAltIcon />
            </IconButton>
          </Box>
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
};

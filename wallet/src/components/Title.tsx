import { Box, SxProps, Typography } from '@mui/material';

export const Title: FCWithChildren<{ title: string | React.ReactNode; Icon?: React.ReactNode; sx?: SxProps }> = ({
  title,
  Icon,
  sx,
}) => (
  <Box width="100%" display="flex" alignItems="center">
    {Icon}
    <Typography width="100%" variant="h5" sx={{ fontWeight: 600, ...sx }}>
      {title}
    </Typography>
  </Box>
);

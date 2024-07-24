import { Typography } from '@mui/material';

export const Title = ({ title }: { title: string }) => (
  <Typography sx={{ color: 'common.white', fontWeight: 600, fontSize: 20 }}>{title}</Typography>
);

export const Subtitle = ({ subtitle }: { subtitle: string }) => (
  <Typography sx={{ color: 'common.white', textAlign: 'center', maxWidth: 450 }}>{subtitle}</Typography>
);

export const SubtitleSlick = ({ subtitle }: { subtitle: string }) => (
  <Typography
    variant="caption"
    sx={{
      color: (theme) => theme.palette.text.disabled,
      textTransform: 'uppercase',
      letterSpacing: 4,
      fontWeight: 400,
      fontSize: 14,
    }}
  >
    {subtitle}
  </Typography>
);

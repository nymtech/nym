import type { SxProps, Theme } from '@mui/material/styles';
import { alpha } from '@mui/material/styles';

/**
 * Shared visual style for account and network controls in the AppBar pill.
 */
export const headerControlPillSx: SxProps<Theme> = {
  color: 'text.primary',
  fontSize: 14,
  textTransform: 'none',
  borderRadius: 999,
  px: 2.25,
  py: 1.25,
  minHeight: 44,
  lineHeight: 1.2,
  border: (theme: Theme) =>
    theme.palette.mode === 'light'
      ? `1px solid ${alpha(theme.palette.common.black, 0.08)}`
      : `1px solid ${theme.palette.divider}`,
  bgcolor: (theme: Theme) =>
    theme.palette.mode === 'dark' ? 'rgba(255,255,255,0.06)' : alpha(theme.palette.background.paper, 0.85),
  '&:hover': {
    bgcolor: (theme: Theme) =>
      theme.palette.mode === 'dark' ? 'rgba(255,255,255,0.1)' : alpha(theme.palette.common.black, 0.04),
  },
};

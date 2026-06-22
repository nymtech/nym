import { Badge, styled } from '@mui/material';

/**
 * Mint notification badge for pending family invites that need addressing. Shows
 * the count inside the dot and hides itself when the count is 0 (MUI's default
 * `showZero={false}`), so callers can pass the raw count without guarding.
 */
export const InviteNotificationBadge = styled(Badge)(({ theme }) => ({
  '& .MuiBadge-badge': {
    backgroundColor: theme.palette.primary.main,
    color: theme.palette.primary.contrastText,
    fontWeight: 700,
    fontSize: 10,
    lineHeight: 1,
    minWidth: 16,
    height: 16,
    padding: '0 4px',
  },
}));

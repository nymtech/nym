import React from 'react';
import { Chip, ChipProps } from '@mui/material';

export type FamilyStatus = 'active' | 'joined' | 'pending' | 'rejected' | 'removed' | 'expired';

const STATUS_STYLES: Record<FamilyStatus, { color: ChipProps['color']; variant: ChipProps['variant']; label: string }> =
  {
    active: { color: 'primary', variant: 'outlined', label: 'Active' },
    joined: { color: 'success', variant: 'filled', label: 'Joined' },
    pending: { color: 'warning', variant: 'outlined', label: 'Pending' },
    rejected: { color: 'error', variant: 'outlined', label: 'Rejected' },
    removed: { color: 'default', variant: 'outlined', label: 'Removed' },
    expired: { color: 'default', variant: 'outlined', label: 'Expired' },
  };

export interface StatusChipProps extends Pick<ChipProps, 'sx'> {
  status: FamilyStatus;
  /** Overrides the default label (e.g. a live "in 3h" expiry on an active invite). */
  label?: string;
  'data-testid'?: string;
}

/** Single source of truth for family status colours/labels, shared by every list and card. */
export const StatusChip = ({ status, label, sx, ...rest }: StatusChipProps) => {
  const style = STATUS_STYLES[status];
  return (
    <Chip size="small" color={style.color} variant={style.variant} label={label ?? style.label} sx={sx} {...rest} />
  );
};

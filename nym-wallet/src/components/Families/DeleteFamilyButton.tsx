import React from 'react';
import { Alert, Box, Stack, Typography } from '@mui/material';
import { ConfirmActionButton } from './ConfirmActionButton';

export interface DeleteFamilyButtonProps {
  /** Current member count; deletion is only allowed at zero. */
  memberCount: number;
  isBusy?: boolean;
  /** `FamilyNotEmpty` (or other) error surfaced from a failed attempt. */
  errorMessage?: string;
  onDelete: () => void;
}

export const DeleteFamilyButton = ({ memberCount, isBusy, errorMessage, onDelete }: DeleteFamilyButtonProps) => {
  const blocked = memberCount > 0;
  return (
    <Stack spacing={2} data-testid="delete-family">
      <Typography variant="body2" color="text.secondary">
        A family must be empty before it can be dissolved. Dissolving returns your creation fee.
      </Typography>
      {blocked && (
        <Alert severity="info" data-testid="delete-family-blocked">
          This family still has {memberCount} member{memberCount === 1 ? '' : 's'}. Remove or wait for members to leave
          before dissolving.
        </Alert>
      )}
      {errorMessage && (
        <Alert severity="error" data-testid="delete-family-error">
          {errorMessage}
        </Alert>
      )}
      <Box sx={{ alignSelf: 'flex-start' }}>
        <ConfirmActionButton
          label="Dissolve family"
          color="error"
          title="Dissolve this family?"
          body="This permanently removes the family and refunds your creation fee. This cannot be undone."
          confirmLabel="Dissolve family"
          disabled={blocked || isBusy}
          onConfirm={onDelete}
          dataTestid="delete-family-button"
        />
      </Box>
    </Stack>
  );
};

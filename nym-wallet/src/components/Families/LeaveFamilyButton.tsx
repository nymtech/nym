import React from 'react';
import { ConfirmActionButton } from './ConfirmActionButton';

export interface LeaveFamilyButtonProps {
  familyName: string;
  isBusy?: boolean;
  onLeave: () => void;
}

export const LeaveFamilyButton = ({ familyName, isBusy, onLeave }: LeaveFamilyButtonProps) => (
  <ConfirmActionButton
    label="Leave family"
    color="error"
    title="Leave family?"
    body={`Leave "${familyName}"? Your node will be removed from the family. You can be invited again afterwards.`}
    confirmLabel="Leave family"
    disabled={isBusy}
    onConfirm={onLeave}
    dataTestid="leave-family-button"
  />
);

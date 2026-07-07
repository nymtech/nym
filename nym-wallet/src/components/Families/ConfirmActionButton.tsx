import React, { useState } from 'react';
import { Button, ButtonProps, Stack } from '@mui/material';
import { ConfirmationModal } from '../Modals/ConfirmationModal';

export interface ConfirmActionButtonProps {
  label: React.ReactNode;
  title: string;
  body?: React.ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  disabled?: boolean;
  color?: ButtonProps['color'];
  variant?: ButtonProps['variant'];
  size?: ButtonProps['size'];
  dataTestid?: string;
}

/** A button that gates its action behind a confirmation prompt (used across all gated family actions). */
export const ConfirmActionButton = ({
  label,
  title,
  body,
  confirmLabel,
  onConfirm,
  disabled,
  color = 'primary',
  variant = 'outlined',
  size = 'small',
  dataTestid,
}: ConfirmActionButtonProps) => {
  const [open, setOpen] = useState(false);

  const handleConfirm = () => {
    setOpen(false);
    onConfirm();
  };

  return (
    <>
      <Button
        variant={variant}
        color={color}
        size={size}
        disabled={disabled}
        onClick={() => setOpen(true)}
        data-testid={dataTestid}
      >
        {label}
      </Button>
      <ConfirmationModal
        open={open}
        title={title}
        confirmButton={
          <Stack direction="row" spacing={2} width="100%">
            <Button fullWidth variant="outlined" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button
              fullWidth
              variant="contained"
              color={color}
              onClick={handleConfirm}
              data-testid={dataTestid ? `${dataTestid}-confirm` : undefined}
            >
              {confirmLabel}
            </Button>
          </Stack>
        }
        onConfirm={handleConfirm}
        onClose={() => setOpen(false)}
      >
        {body}
      </ConfirmationModal>
    </>
  );
};

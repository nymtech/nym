import React from 'react';
import { FeeDetails } from '@nymproject/types';
import { Box, Button } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
import { SimpleModal } from './Modals/SimpleModal';
import { ModalFee } from './Modals/ModalFee';
import { backDropStyles, modalStyles } from '../../.storybook/storiesStyles';

const storybookStyles = (theme: Theme, isStorybook?: boolean, backdropProps?: object) =>
  !!isStorybook
    ? {
        backdropProps: { ...backDropStyles(theme), ...backdropProps },
        sx: modalStyles(theme),
      }
    : {};

export const ConfirmTx: React.FC<{
  open: boolean;
  header: string;
  subheader?: string;
  fee: FeeDetails;
  onConfirm: () => Promise<void>;
  onClose?: () => void;
  onPrev: () => void;
  isStorybook?: boolean;
}> = ({ open, fee, onConfirm, onClose, header, subheader, onPrev, children, isStorybook }) => {
  const theme = useTheme();
  return (
    <SimpleModal
      open={open}
      header={header}
      subHeader={subheader}
      okLabel="Confirm"
      onOk={onConfirm}
      onClose={onClose}
      SecondaryAction={
        <Button fullWidth sx={{ mt: 1 }} size="large" onClick={onPrev}>
          Cancel
        </Button>
      }
      {...storybookStyles(theme, isStorybook)}
    >
      <Box sx={{ mt: 3 }}>
        {children}
        <ModalFee fee={fee} isLoading={false} />
      </Box>
    </SimpleModal>
  );
};

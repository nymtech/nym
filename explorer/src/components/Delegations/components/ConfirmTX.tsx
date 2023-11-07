import React from 'react';
import { FeeDetails } from '@nymproject/types';
import { Box } from '@mui/material';
import { useTheme, Theme } from '@mui/material/styles';
import { SimpleModal } from './SimpleModal';
import { ModalFee } from './ModalFee';
import { ModalDivider } from './ModalDivider';
import { backDropStyles, modalStyles } from './styles';

const storybookStyles = (theme: Theme, isStorybook?: boolean, backdropProps?: object) =>
  isStorybook
    ? {
        backdropProps: { ...backDropStyles(theme), ...backdropProps },
        sx: modalStyles(theme),
      }
    : {};

export const ConfirmTx: FCWithChildren<{
  open: boolean;
  header: string;
  subheader?: string;
  fee: FeeDetails;
  onConfirm: () => Promise<void>;
  onClose?: () => void;
  onPrev: () => void;
  isStorybook?: boolean;
  children?: React.ReactNode;
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
      onBack={onPrev}
      {...storybookStyles(theme, isStorybook)}
    >
      <Box sx={{ mt: 3 }}>
        {children}
        <ModalFee fee={fee} isLoading={false} />
        <ModalDivider />
      </Box>
    </SimpleModal>
  );
};

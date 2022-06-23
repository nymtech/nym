import React from 'react';
import { Box, Button, Modal, Stack, SxProps, Typography } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import ErrorOutline from '@mui/icons-material/ErrorOutline';
import { modalStyle } from './styles';

export const SimpleModal: React.FC<{
  open: boolean;
  hideCloseIcon?: boolean;
  displayErrorIcon?: boolean;
  headerStyles?: SxProps;
  subHeaderStyles?: SxProps;
  onClose?: () => void;
  onOk?: () => void;
  header: string;
  subHeader?: string;
  okLabel: string;
  okDisabled?: boolean;
  sx?: SxProps;
  SecondaryAction?: React.ReactNode;
}> = ({
  open,
  hideCloseIcon,
  displayErrorIcon,
  headerStyles,
  subHeaderStyles,
  onClose,
  okDisabled,
  onOk,
  header,
  subHeader,
  okLabel,
  sx,
  SecondaryAction,
  children,
}) => (
  <Modal open={open} onClose={onClose}>
    <Box sx={{ ...modalStyle, ...sx }}>
      {displayErrorIcon && <ErrorOutline color="error" sx={{ mb: 3 }} />}
      <Stack direction="row" justifyContent="space-between" alignItems="center">
        <Typography fontSize={22} fontWeight={600} sx={{ ...headerStyles }}>
          {header}
        </Typography>
        {!hideCloseIcon && <CloseIcon onClick={onClose} cursor="pointer" />}
      </Stack>
      {subHeader && (
        <Typography
          mt={0.5}
          mb={3}
          fontSize="small"
          color={(theme) => theme.palette.text.secondary}
          sx={{ ...subHeaderStyles }}
        >
          {subHeader}
        </Typography>
      )}

      {children}

      <Button variant="contained" fullWidth size="large" onClick={onOk} disabled={okDisabled} sx={{ mt: 2 }}>
        {okLabel}
      </Button>

      {SecondaryAction}
    </Box>
  </Modal>
);

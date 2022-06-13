import React from 'react';
import { Box, Button, Modal, Stack, SxProps, Typography } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import ErrorOutline from '@mui/icons-material/ErrorOutline';
import { modalStyle } from './styles';

export const SimpleModal: React.FC<{
  open: boolean;
  displayCloseIcon?: boolean;
  displayErrorIcon?: boolean;
  titleStyles?: SxProps;
  descriptionStyles?: SxProps;
  onClose?: () => void;
  onOk?: () => void;
  header: string;
  subHeader?: string;
  okLabel: string;
  okDisabled?: boolean;
  sx?: SxProps;
}> = ({
  open,
  displayCloseIcon,
  displayErrorIcon,
  titleStyles,
  descriptionStyles,
  onClose,
  okDisabled,
  onOk,
  header,
  subHeader,
  okLabel,
  sx,
  children,
}) => (
  <Modal open={open} onClose={onClose}>
    <Box sx={{ ...modalStyle, ...sx }}>
      {!!displayErrorIcon && <ErrorOutline color="error" sx={{ mb: 3 }} />}
      <Stack direction="row" justifyContent="space-between" alignItems="center">
        <Typography fontSize={22} fontWeight={600} sx={{ ...titleStyles }}>
          {header}
        </Typography>
        {!!displayCloseIcon && <CloseIcon onClick={onClose} cursor="pointer" />}
      </Stack>
      {subHeader && (
        <Typography
          mt={0.5}
          mb={3}
          fontSize="small"
          color={(theme) => theme.palette.text.secondary}
          sx={{ ...descriptionStyles }}
        >
          {subHeader}
        </Typography>
      )}

      {children}

      <Button variant="contained" fullWidth sx={{ mt: 3 }} size="large" onClick={onOk} disabled={okDisabled}>
        {okLabel}
      </Button>
    </Box>
  </Modal>
);

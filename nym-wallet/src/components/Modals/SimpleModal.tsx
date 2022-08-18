import React from 'react';
import { Box, Button, Modal, Stack, SxProps, Typography } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import ErrorOutline from '@mui/icons-material/ErrorOutline';
import { StyledBackButton } from 'src/components/StyledBackButton';
import { modalStyle } from './styles';

export const SimpleModal: React.FC<{
  open: boolean;
  hideCloseIcon?: boolean;
  displayErrorIcon?: boolean;
  headerStyles?: SxProps;
  subHeaderStyles?: SxProps;
  onClose?: () => void;
  onOk?: () => Promise<void>;
  onBack?: () => void;
  header: string | React.ReactNode;
  subHeader?: string;
  okLabel: string;
  okDisabled?: boolean;
  sx?: SxProps;
  backdropProps?: object;
}> = ({
  open,
  hideCloseIcon,
  displayErrorIcon,
  headerStyles,
  subHeaderStyles,
  onClose,
  okDisabled,
  onOk,
  onBack,
  header,
  subHeader,
  okLabel,
  sx,
  children,
  backdropProps,
}) => (
  <Modal open={open} onClose={onClose} BackdropProps={backdropProps}>
    <Box sx={{ ...modalStyle, ...sx }}>
      {displayErrorIcon && <ErrorOutline color="error" sx={{ mb: 3 }} />}
      <Stack direction="row" justifyContent="space-between" alignItems="center">
        {typeof header === 'string' ? (
          <Typography fontSize={20} fontWeight={600} sx={{ color: 'text.primary', ...headerStyles }}>
            {header}
          </Typography>
        ) : (
          header
        )}
        {!hideCloseIcon && <CloseIcon onClick={onClose} cursor="pointer" />}
      </Stack>

      <Typography
        mt={0.5}
        mb={3}
        fontSize={12}
        color={(theme) => theme.palette.text.secondary}
        sx={{ color: (theme) => theme.palette.nym.nymWallet.text.muted, ...subHeaderStyles }}
      >
        {subHeader}
      </Typography>

      {children}

      {(onOk || onBack) && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mt: 2 }}>
          {onBack && <StyledBackButton onBack={onBack} />}
          {onOk && (
            <Button variant="contained" fullWidth size="large" onClick={onOk} disabled={okDisabled}>
              {okLabel}
            </Button>
          )}
        </Box>
      )}
    </Box>
  </Modal>
);

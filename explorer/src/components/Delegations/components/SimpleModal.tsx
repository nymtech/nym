import React from 'react';
import { Box, Button, Modal, Stack, SxProps, Typography } from '@mui/material';
import CloseIcon from '@mui/icons-material/Close';
import ErrorOutline from '@mui/icons-material/ErrorOutline';
import InfoOutlinedIcon from '@mui/icons-material/InfoOutlined';
import { modalStyle } from '../../../../../nym-wallet/src/components/Modals/styles';

import ArrowBackIosNewIcon from '@mui/icons-material/ArrowBackIosNew';

export const StyledBackButton = ({
  onBack,
  label,
  fullWidth,
  sx,
}: {
  onBack: () => void;
  label?: string;
  fullWidth?: boolean;
  sx?: SxProps;
}) => (
  <Button disableFocusRipple size="large" fullWidth={fullWidth} variant="outlined" onClick={onBack} sx={sx}>
    {label || <ArrowBackIosNewIcon fontSize="small" />}
  </Button>
);

export const SimpleModal: FCWithChildren<{
  open: boolean;
  hideCloseIcon?: boolean;
  displayErrorIcon?: boolean;
  displayInfoIcon?: boolean;
  headerStyles?: SxProps;
  subHeaderStyles?: SxProps;
  buttonFullWidth?: boolean;
  onClose?: () => void;
  onOk?: () => Promise<void>;
  onBack?: () => void;
  header: string | React.ReactNode;
  subHeader?: string;
  okLabel: string;
  backLabel?: string;
  backButtonFullWidth?: boolean;
  okDisabled?: boolean;
  sx?: SxProps;
  backdropProps?: object;
  children?: React.ReactNode;
}> = ({
  open,
  hideCloseIcon,
  displayErrorIcon,
  displayInfoIcon,
  headerStyles,
  subHeaderStyles,
  buttonFullWidth,
  onClose,
  okDisabled,
  onOk,
  onBack,
  header,
  subHeader,
  okLabel,
  backLabel,
  backButtonFullWidth,
  sx,
  children,
  backdropProps,
}) => (
  <Modal open={open} onClose={onClose} BackdropProps={backdropProps}>
    <Box sx={{ border: (t) => `1px solid #fff`, ...modalStyle, ...sx }}>
      {displayErrorIcon && <ErrorOutline color="error" sx={{ mb: 3 }} />}
      {displayInfoIcon && <InfoOutlinedIcon sx={{ mb: 2, color: 'blue' }} />}
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
        mt={subHeader ? 0.5 : 0}
        mb={3}
        fontSize={12}
        color={(theme) => theme.palette.text.secondary}
        // sx={{ color: (theme) => theme.palette.nym.nymWallet.text.muted, ...subHeaderStyles }}
      >
        {subHeader}
      </Typography>

      {children}

      {(onOk || onBack) && (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mt: 2, width: buttonFullWidth ? '100%' : null }}>
          {onBack && <StyledBackButton onBack={onBack} label={backLabel} fullWidth={backButtonFullWidth} />}
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

import React from 'react';
import { Box, Button, Modal, Stack, SxProps, Typography } from '@mui/material';
import { alpha } from '@mui/material/styles';
import CloseIcon from '@mui/icons-material/Close';
import ErrorOutline from '@mui/icons-material/ErrorOutline';
import InfoOutlinedIcon from '@mui/icons-material/InfoOutlined';
import { StyledBackButton } from 'src/components/StyledBackButton';
import { modalStyle } from './styles';

export const SimpleModal: FCWithChildren<{
  open: boolean;
  hideCloseIcon?: boolean;
  displayErrorIcon?: boolean;
  displayInfoIcon?: boolean;
  /** Center the header title; close control stays top-right. */
  headerCentered?: boolean;
  headerStyles?: SxProps;
  subHeaderStyles?: SxProps;
  buttonFullWidth?: boolean;
  /** Tighter padding and typography */
  dense?: boolean;
  /** Primary left accent bar */
  accent?: 'none' | 'primary';
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
  headerCentered,
  headerStyles,
  subHeaderStyles,
  buttonFullWidth,
  dense,
  accent = 'none',
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
    <Box
      sx={{
        border: (t) => `1px solid ${t.palette.nym.nymWallet.modal.border}`,
        ...modalStyle,
        ...(dense ? { p: 3, borderRadius: '12px' } : {}),
        ...(accent === 'primary'
          ? {
              borderLeft: (t) => `4px solid ${t.palette.primary.main}`,
            }
          : {}),
        ...sx,
      }}
    >
      {displayErrorIcon && <ErrorOutline color="error" sx={{ mb: dense ? 2 : 3 }} />}
      {displayInfoIcon && <InfoOutlinedIcon sx={{ mb: 2, color: (theme) => theme.palette.nym.nymWallet.text.blue }} />}
      <Stack
        direction="row"
        justifyContent={headerCentered ? 'center' : 'space-between'}
        alignItems="center"
        sx={headerCentered ? { position: 'relative', width: '100%' } : undefined}
      >
        {typeof header === 'string' ? (
          <Typography
            fontSize={dense ? 18 : 20}
            fontWeight={600}
            sx={{
              color: 'text.primary',
              ...(headerCentered ? { flex: 1, textAlign: 'center', pr: 4 } : {}),
              ...headerStyles,
            }}
          >
            {header}
          </Typography>
        ) : (
          header
        )}
        {!hideCloseIcon && (
          <CloseIcon
            onClick={onClose}
            cursor="pointer"
            sx={
              headerCentered
                ? {
                    position: 'absolute',
                    right: 0,
                    top: '50%',
                    transform: 'translateY(-50%)',
                  }
                : undefined
            }
          />
        )}
      </Stack>

      {subHeader ? (
        <Typography
          mt={0.5}
          mb={dense ? 2 : 3}
          fontSize={14}
          sx={{
            color: 'text.secondary',
            lineHeight: 1.45,
            ...(headerCentered ? { textAlign: 'center' } : {}),
            ...subHeaderStyles,
          }}
        >
          {subHeader}
        </Typography>
      ) : null}

      {children}

      {(onOk || onBack) && (
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 2,
            mt: dense ? 1.5 : 2,
            width: buttonFullWidth ? '100%' : null,
          }}
        >
          {onBack && <StyledBackButton onBack={onBack} label={backLabel} fullWidth={backButtonFullWidth} />}
          {onOk && (
            <Button
              variant="contained"
              fullWidth
              size="large"
              onClick={onOk}
              disabled={okDisabled}
              sx={(theme) => ({
                color: theme.palette.primary.contrastText,
                '&.Mui-disabled': {
                  color: alpha(theme.palette.primary.contrastText, 0.55),
                },
              })}
            >
              {okLabel}
            </Button>
          )}
        </Box>
      )}
    </Box>
  </Modal>
);

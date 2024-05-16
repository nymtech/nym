import React from 'react'
import { Box, Button, Modal, Stack, SxProps, Typography } from '@mui/material'
import CloseIcon from '@mui/icons-material/Close'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import InfoOutlinedIcon from '@mui/icons-material/InfoOutlined'
import ArrowBackIosNewIcon from '@mui/icons-material/ArrowBackIosNew'
import { useIsMobile } from '@/app/hooks/useIsMobile'

export const modalStyle = (width: number | string = 600) => ({
  position: 'absolute' as 'absolute',
  top: '50%',
  left: '50%',
  width,
  transform: 'translate(-50%, -50%)',
  bgcolor: 'background.paper',
  boxShadow: 24,
  borderRadius: '16px',
  p: 4,
})

export const StyledBackButton = ({
  onBack,
  label,
  fullWidth,
  sx,
}: {
  onBack: () => void
  label?: string
  fullWidth?: boolean
  sx?: SxProps
}) => (
  <Button
    disableFocusRipple
    size="large"
    fullWidth={fullWidth}
    variant="outlined"
    onClick={onBack}
    sx={sx}
  >
    {label || <ArrowBackIosNewIcon fontSize="small" />}
  </Button>
)

export const SimpleModal: FCWithChildren<{
  open: boolean
  hideCloseIcon?: boolean
  displayErrorIcon?: boolean
  displayInfoIcon?: boolean
  headerStyles?: SxProps
  subHeaderStyles?: SxProps
  buttonFullWidth?: boolean
  onClose?: () => void
  onOk?: () => Promise<void>
  onBack?: () => void
  header: string | React.ReactNode
  subHeader?: string
  okLabel: string
  backLabel?: string
  backButtonFullWidth?: boolean
  okDisabled?: boolean
  sx?: SxProps
  children?: React.ReactNode
}> = ({
  open,
  hideCloseIcon,
  displayErrorIcon,
  displayInfoIcon,
  headerStyles,
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
}) => {
  const isMobile = useIsMobile()

  return (
    <Modal open={open} onClose={onClose}>
      <Box sx={{ ...modalStyle(isMobile ? '90%' : 600), ...sx }}>
        {displayErrorIcon && <ErrorOutline color="error" sx={{ mb: 3 }} />}
        {displayInfoIcon && <InfoOutlinedIcon sx={{ mb: 2, color: 'blue' }} />}
        <Stack
          direction="row"
          justifyContent="space-between"
          alignItems="center"
        >
          {typeof header === 'string' ? (
            <Typography
              fontSize={20}
              fontWeight={600}
              sx={{ color: 'text.primary', ...headerStyles }}
            >
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
        >
          {subHeader}
        </Typography>

        {children}

        {(onOk || onBack) && (
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 2,
              mt: 2,
              width: buttonFullWidth ? '100%' : null,
            }}
          >
            {onBack && (
              <StyledBackButton
                onBack={onBack}
                label={backLabel}
                fullWidth={backButtonFullWidth}
              />
            )}
            {onOk && (
              <Button
                variant="contained"
                fullWidth
                size="large"
                onClick={onOk}
                disabled={okDisabled}
              >
                {okLabel}
              </Button>
            )}
          </Box>
        )}
      </Box>
    </Modal>
  )
}

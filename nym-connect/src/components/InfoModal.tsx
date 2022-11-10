import { Close, ErrorOutline } from '@mui/icons-material';
import { Box, IconButton, Modal, Theme, Typography } from '@mui/material';
import React from 'react';

const styles = {
  position: 'absolute',
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  width: 200,
  bgcolor: '#292E34',
  p: 1.5,
  borderRadius: 0.5,
  height: 'fit-content',
  border: (theme: Theme) => `1px solid ${theme.palette.grey[700]}`,
};

const ModalTitle = ({ title, withCloseIcon }: { title: string; withCloseIcon: boolean }) => {
  return (
    <Box textAlign="center" mt={withCloseIcon ? -2 : 0}>
      <ErrorOutline sx={{ color: 'warning.main' }} />
      <Typography variant="body2" textAlign="center" sx={{ color: 'warning.main' }}>
        {title}
      </Typography>
    </Box>
  );
};

const ModalBody = ({ description, children }: { description: string; children?: React.ReactElement }) => (
  <Box textAlign="center" mt={1}>
    {children}
    <Typography fontSize="small" sx={{ mt: 1 }}>
      {description}
    </Typography>
  </Box>
);

export const InfoModal = ({
  title,
  description,
  show,
  children,
  Action,
  onClose,
}: {
  title: string;
  description: string;
  show: boolean;
  children?: React.ReactElement;
  Action?: React.ReactNode;
  onClose?: () => void;
}) => (
  <Modal open={show} onClose={onClose}>
    <Box sx={styles}>
      {onClose && (
        <Box display="flex" justifyContent="flex-end">
          <IconButton size="small" onClick={onClose}>
            <Close sx={{ fontSize: 14 }} />
          </IconButton>
        </Box>
      )}
      <ModalTitle title={title} withCloseIcon={Boolean(onClose)} />
      <ModalBody description={description}>{children}</ModalBody>
      {Action && (
        <Box mt={1} textAlign="center">
          {Action}
        </Box>
      )}
    </Box>
  </Modal>
);

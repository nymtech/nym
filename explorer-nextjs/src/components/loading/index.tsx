import {
  Box,
  CircularProgress,
  Modal,
  Stack,
  type SxProps,
  Typography,
} from "@mui/material";

const modalStyle: SxProps = {
  position: "absolute",
  top: "50%",
  left: "50%",
  transform: "translate(-50%, -50%)",
  width: 300,
  bgcolor: "background.paper",
  boxShadow: 24,
  borderRadius: "16px",
  p: 4,
};

const LoadingModal: FCWithChildren<{
  text?: string;
  sx?: SxProps;
  backdropProps?: object;
}> = ({ sx, backdropProps, text = "Please wait..." }) => (
  <Modal open slotProps={{ backdrop: backdropProps }}>
    <Box sx={{ ...modalStyle, ...sx }} textAlign="center">
      <Stack spacing={4} direction="row" alignItems="center">
        <CircularProgress size={25} />
        <Typography variant="h4">{text}</Typography>
      </Stack>
    </Box>
  </Modal>
);

export default LoadingModal;

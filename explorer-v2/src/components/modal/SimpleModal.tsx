"use client";

import {
  Dialog,
  DialogActions,
  DialogContent,
  IconButton,
  Stack,
  Typography,
  useTheme,
} from "@mui/material";
import Cross from "../../components/icons/Cross";
import CrossDark from "../../components/icons/CrossDark";

type SimpleModalPropsClosed = {
  open: false;
};

type SimpleModalPropsOpen = {
  open: true;
  title: string;
  children?: React.ReactNode;
  Actions?: React.ReactNode;
  onClose: () => void;
};

export type SimpleModalProps = SimpleModalPropsClosed | SimpleModalPropsOpen;

const SimpleModal = (props: SimpleModalProps) => {
  const theme = useTheme();
  if (!props.open) {
    return null;
  }

  const { title, children, Actions, onClose } = props;

  return (
    <Dialog open maxWidth="sm" fullWidth onClose={onClose}>
      <Stack
        direction="row"
        justifyContent="space-between"
        alignItems="center"
        sx={{ p: 4 }}
      >
        <Typography variant="body1" sx={{ textTransform: "uppercase" }}>
          {title}
        </Typography>
        <IconButton aria-label="close" size="large" onClick={onClose}>
          {theme.palette.mode === "dark" ? <CrossDark /> : <Cross />}
        </IconButton>
      </Stack>
      <DialogContent sx={{ p: 4, pt: 0 }}>{children}</DialogContent>
      <DialogActions sx={{ px: 4, pb: 4 }}>{Actions}</DialogActions>
    </Dialog>
  );
};

export default SimpleModal;

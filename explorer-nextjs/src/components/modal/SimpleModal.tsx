import Cross from "@/components/icons/Cross";
import {
  Dialog,
  DialogActions,
  DialogContent,
  IconButton,
  Stack,
  Typography,
} from "@mui/material";

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

const SimpleModal = ({
  open,
  title,
  children,
  Actions,
  onClose,
}: SimpleModalProps) => {
  return (
    <Dialog open={open} maxWidth="sm" fullWidth onClose={onClose}>
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
          <Cross />
        </IconButton>
      </Stack>
      <DialogContent sx={{ p: 4, pt: 0 }}>{children}</DialogContent>
      <DialogActions sx={{ px: 4, pb: 4 }}>{Actions}</DialogActions>
    </Dialog>
  );
};

export default SimpleModal;

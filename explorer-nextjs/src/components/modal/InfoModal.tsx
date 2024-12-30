import { Button, Link, Stack, Typography } from "@mui/material";
import SimpleModal from "./SimpleModal";

type InfoModalPropsClosed = {
  open: false;
};

type InfoModalPropsOpen = {
  open: true;
  title: string;
  message: string;
  Action?: React.ReactNode;
  tx?: string;
  onClose: () => void;
};

export type InfoModalProps = InfoModalPropsClosed | InfoModalPropsOpen;

const InfoModal = (props: InfoModalProps) => {
  if (!props.open) {
    return null;
  }

  const { open, onClose, title, message, tx, Action } = props;

  return (
    <SimpleModal
      onClose={onClose}
      open={open}
      title=""
      Actions={
        Action || (
          <Button variant="contained" onClick={onClose}>
            Close
          </Button>
        )
      }
    >
      <Stack spacing={2} alignItems="center">
        <Typography variant="h3">{title}</Typography>
        <Typography variant="body3">{message}</Typography>
        {tx && (
          <Link href="/">
            <Typography variant="h5">Block explorer link</Typography>
          </Link>
        )}
      </Stack>
    </SimpleModal>
  );
};

export default InfoModal;

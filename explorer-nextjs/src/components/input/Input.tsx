import { TextField } from "@mui/material";

const Input = ({
  placeholder,
  fullWidth,
}: {
  placeholder?: string;
  fullWidth?: boolean;
}) => {
  return <TextField placeholder={placeholder} fullWidth={fullWidth} />;
};

export default Input;

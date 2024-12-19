import { TextField } from "@mui/material";

const Input = ({
  placeholder,
  fullWidth,
  value,
  onChange,
}: {
  placeholder?: string;
  fullWidth?: boolean;
  value: string;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}) => {
  return (
    <TextField
      placeholder={placeholder}
      fullWidth={fullWidth}
      value={value}
      onChange={onChange}
    />
  );
};

export default Input;

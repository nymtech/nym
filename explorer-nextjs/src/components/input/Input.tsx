import { type SxProps, TextField } from "@mui/material";

const Input = ({
  placeholder,
  fullWidth,
  value,
  rounded = false,
  onChange,
}: {
  placeholder?: string;
  fullWidth?: boolean;
  rounded?: boolean;
  sx?: SxProps;
  value: string;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}) => {
  return (
    <TextField
      placeholder={placeholder}
      fullWidth={fullWidth}
      value={value}
      onChange={onChange}
      slotProps={{
        input: {
          sx: {
            borderRadius: rounded ? 10 : 2,
          },
        },
      }}
    />
  );
};

export default Input;

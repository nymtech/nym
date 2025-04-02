import { type SxProps, TextField } from "@mui/material";
import type { KeyboardEventHandler } from "react";

const Input = ({
  placeholder,
  fullWidth,
  value,
  rounded = false,
  onChange,
  onKeyDown,
}: {
  placeholder?: string;
  fullWidth?: boolean;
  rounded?: boolean;
  sx?: SxProps;
  value: string;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
  onKeyDown?: KeyboardEventHandler<HTMLDivElement> | undefined;
}) => {
  return (
    <TextField
      placeholder={placeholder}
      fullWidth={fullWidth}
      value={value}
      onChange={onChange}
      onKeyDown={onKeyDown}
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

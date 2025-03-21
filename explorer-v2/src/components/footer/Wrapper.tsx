import { Box } from "@mui/material";
import type { BoxProps } from "@mui/system";

export type WrapperProps = BoxProps;

export function Wrapper({ children, sx, ...props }: WrapperProps) {
  return (
    <Box
      sx={{
        mx: "auto",
        maxWidth: "1378px",
        width: "100%",
        px: "30px",
        ...sx,
      }}
      {...props}
    >
      {children}
    </Box>
  );
}

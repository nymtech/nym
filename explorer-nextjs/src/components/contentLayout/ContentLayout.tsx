import { Box, type BoxProps } from "@mui/material";

export const ContentLayout = ({
  children,
  component = "div",
  className,
  sx,
  ...rest
}: BoxProps) => {
  return (
    <Box
      component={component}
      sx={{
        display: "flex",
        flexDirection: "column",
        gap: { xs: 3, md: 10 },
        py: { xs: 3, md: 10 },
        ...sx,
      }}
      className={className}
      {...rest}
    >
      {children}
    </Box>
  );
};
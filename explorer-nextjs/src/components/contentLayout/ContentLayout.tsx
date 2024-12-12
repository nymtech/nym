import { Box, type BoxProps } from "@mui/material";

export const ContentLayout = ({
  children,
  component: Component = "div",
  className,
  sx,
  ...rest
}: BoxProps) => {
  return (
    <Box
      component={Component}
      sx={{
        display: "flex",
        flexDirection: "column",
        gap: { xs: "30px", md: "200px" },
        py: { xs: "30px", md: "100px" },
        ...sx,
      }}
      className={className}
      {...rest}
    >
      {children}
    </Box>
  );
};

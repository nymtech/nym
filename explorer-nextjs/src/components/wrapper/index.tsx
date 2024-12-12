import { Container, type SxProps } from "@mui/material";

export function Wrapper({
  children,
  sx,
}: {
  children: React.ReactNode;
  sx?: SxProps;
}) {
  return (
    <Container
      maxWidth="xl"
      sx={{
        ...sx,
      }}
    >
      {children}
    </Container>
  );
}

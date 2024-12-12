import { Divider, Stack, type SxProps, Typography } from "@mui/material";

const listItemLabelStyle: SxProps = {
  color: "pine.600",
  letterSpacing: 0.7,
  flexGrow: 1,
};

const listItemValueStyle: SxProps = {
  fontSize: "small",
  fontWeight: 300,
  color: "pine.950",
};

const ExplorerListItem = ({
  label,
  value,
  row,
  divider,
}: {
  label: string;
  value: string | React.ReactNode;
  row?: boolean;
  divider: boolean;
}) => {
  return (
    <>
      <Stack
        direction={row ? "row" : "column"}
        alignItems={row ? "center" : "flex-start"}
        justifyContent="space-between"
        gap={1}
      >
        <Typography variant="h6" sx={listItemLabelStyle}>
          {label}
        </Typography>
        {typeof value === "string" ? (
          <Typography variant="body3" sx={listItemValueStyle}>
            {value}
          </Typography>
        ) : (
          value
        )}
      </Stack>
      {divider && <Divider variant="fullWidth" sx={{ my: 2 }} />}
    </>
  );
};

export default ExplorerListItem;

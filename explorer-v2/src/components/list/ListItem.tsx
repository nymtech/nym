import {
  Divider,
  Stack,
  type SxProps,
  Typography,
  useTheme,
} from "@mui/material";

const ExplorerListItem = ({
  label,
  value,
  row,
  divider,
}: {
  label: string;
  value: string | React.ReactNode;
  row?: boolean;
  divider?: boolean;
}) => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  const listItemLabelStyle: SxProps = {
    color: isDarkMode ? "pine.300" : "pine.600",
    letterSpacing: 0.7,
    flexGrow: 1,
  };

  const listItemValueStyle: SxProps = {
    fontSize: "small",
    fontWeight: 300,
    color: isDarkMode ? "base.white" : "pine.950",
  };

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

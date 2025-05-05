import { Stack, Typography, useTheme } from "@mui/material";
import Flag from "react-world-flags";

interface ICountryFlag {
  countryCode: string;
  countryName?: string | JSX.Element;
}

const CountryFlag = ({ countryCode, countryName }: ICountryFlag) => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  return (
    <Stack direction="row" gap={1}>
      <Flag code={countryCode} width="19" />

      <Typography
        variant="h6"
        sx={{ color: isDarkMode ? "white" : "pine.900" }}
      >
        {countryName}
      </Typography>
    </Stack>
  );
};

export default CountryFlag;
export type { ICountryFlag };

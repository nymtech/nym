import { Stack, Typography } from "@mui/material";
import Flag from "react-world-flags";

interface ICountryFlag {
  countryCode: string;
  countryName?: string;
}

const CountryFlag = ({ countryCode, countryName }: ICountryFlag) => {
  return (
    <Stack direction="row" gap={1}>
      <Flag code={countryCode} width="19" />
      {countryName && (
        <Typography variant="subtitle2" sx={{ color: "pine.950" }}>
          {countryName}
        </Typography>
      )}
    </Stack>
  );
};

export default CountryFlag;
export type { ICountryFlag };

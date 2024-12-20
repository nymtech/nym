"use client";
import type { BondInformation, NodeDescription } from "@/app/api/types";
import { Box, Button, Stack, Typography } from "@mui/material";
import { RandomAvatar } from "react-random-avatars";
import ExplorerCard from "../cards/ExplorerCard";
import CountryFlag from "../countryFlag/CountryFlag";

interface INodeProfileCardProps {
  bondInfo: BondInformation;
  nodeDescription: NodeDescription;
}

export const NodeProfileCard = (props: INodeProfileCardProps) => {
  const { bondInfo, nodeDescription } = props;

  return (
    <ExplorerCard label="Nym Node" sx={{ height: "100%" }}>
      <Stack gap={1}>
        <Box display={"flex"} justifyContent={"flex-start"}>
          <RandomAvatar name={bondInfo.node.identity_key} size={80} square />
        </Box>
        <Typography
          variant="h3"
          mt={3}
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {"Moniker"}
        </Typography>
        <CountryFlag
          countryCode={nodeDescription.auxiliary_details.location || ""}
        />
        <Typography variant="body4" sx={{ color: "pine.950" }}>
          Team of professional validators with best digital solutions. Please
          visit our Telegram🔹https://t.me/CryptoSailorsAnn🔹
        </Typography>
        <Box mt={3}>
          <Button variant="contained" size="small">
            Stake on node
          </Button>
        </Box>
      </Stack>
    </ExplorerCard>
  );
};

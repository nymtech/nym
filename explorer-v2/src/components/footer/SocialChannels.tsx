"use client";
import { Box } from "@mui/material";
import socialChannels from "../../app/i18n/locales/en/social-channels.json";
import { Link } from "./MuiLink";

import { SocialIcon } from "./SocialIcon";

export const SocialChannels = () => {
  return (
    <Box
      sx={{
        display: "flex",
        gap: "30px",
      }}
    >
      {socialChannels.socialChannels.map(
        (channel: { name: string; link: string }) => {
          return (
            <Link
              key={channel.name}
              href={channel?.link}
              sx={{
                width: "32px",
                height: "32px",
                color: "background.main",
                backgroundColor: "secondary.main",
                "&:hover": {
                  backgroundColor: "background.default",
                },
                borderRadius: "50%",
              }}
            >
              <SocialIcon channel={channel?.name} />
            </Link>
          );
        },
      )}
    </Box>
  );
};

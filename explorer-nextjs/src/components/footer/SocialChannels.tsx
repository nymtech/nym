"use client";
import { Link } from "@/components/muiLink";
import { Box } from "@mui/material";
import socialChannels from "../../app/i18n/locales/en/social-channels.json";

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
                backgroundColor: "light.main",
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

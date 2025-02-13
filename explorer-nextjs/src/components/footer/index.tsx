// API

import { SocialChannels } from "@/components/footer/SocialChannels";
// Components
import { Wrapper } from "@/components/wrapper";

import { getFooter } from "@/app/features/footer/api/getFooter";
// MUI Components
import { Box, Typography } from "@mui/material";
import { NewsletterSignUp } from "./NewsLetterSignUp";
import { FooterLinks } from "./footer-links";

export async function Footer() {
  const locale = "en";
  const footerData = await getFooter(locale);
  const legalContent1 = footerData?.attributes?.legalContent1 || false;
  const legalContent2 = footerData?.attributes?.legalContent2 || false;
  const footerLinkBlocks = footerData?.attributes?.linkBlocks || [];

  return (
    <>
      <Box
        component={"footer"}
        sx={{
          py: 10,
          backgroundColor: "medium.main",
        }}
      >
        <Wrapper>
          <Box
            sx={{
              borderBottom: "1px solid",
              borderColor: "background.default",
              pb: 7.5,
              mb: 7.5,
              display: { md: "flex" },
              flexDirection: { xs: "column", md: "row" },
              justifyContent: { xs: "center", md: "space-between" },
              alignItems: "center",
              gap: 2,
            }}
          >
            <Box
              sx={{
                display: "flex",
                flexDirection: { xs: "column", md: "row" },
                alignItems: "center",
                gap: 2,
                flexGrow: 1,
                justifyContent: { xs: "center", md: "flex-start" },
                pb: { xs: 2, md: 0 },
              }}
            >
              <Typography variant="h2">Nym Newsletter</Typography>
              <NewsletterSignUp />
            </Box>
            <Box
              sx={{
                display: "flex",
                flexDirection: { xs: "column", md: "row" },
                justifyContent: { xs: "center", md: "space-between" },
                alignItems: "center",
                gap: 2,
              }}
            >
              <Box
                sx={{
                  display: "flex",
                  justifyContent: { xs: "center", md: "flex-start" },
                }}
              >
                <SocialChannels />
              </Box>
            </Box>
          </Box>

          <FooterLinks linkBlocks={footerLinkBlocks} />

          <Box
            sx={{
              py: 5,
              display: "flex",
              flexDirection: { xs: "column", md: "row" },
              gap: { xs: 2, md: 0 },
              justifyContent: "space-between",
              textAlign: { xs: "center", md: "left" },
            }}
          >
            {legalContent1 ? (
              <Typography variant="subtitle3">{legalContent1}</Typography>
            ) : null}
            {legalContent2 ? (
              <Typography variant="subtitle3">{legalContent2}</Typography>
            ) : null}
          </Box>
        </Wrapper>
      </Box>
    </>
  );
}

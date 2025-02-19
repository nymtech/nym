// API

import { SocialChannels } from "./SocialChannels";
// Components
import { Wrapper } from "./Wrapper";

// MUI Components
import { Box, Typography } from "@mui/material";
import { getFooter } from "../../app/features/footer/api/getFooter";
import { Link } from "./MuiLink";
// import { NewsletterSignUp } from "./NewsLetterSignUp";
import { FooterLinks } from "./footer-links";

const links = [
  { id: 1, title: "Imprint", url: "https://nym.com/imprint" },

  {
    id: 2,
    title: "nym.com Privacy statement",
    url: "https://nym.com/nym-com-privacy-statement",
  },
  { id: 3, title: "NymVPN Terms of use", url: "https://nym.com/vpn-terms" },
  {
    id: 4,
    title: "NymVPN referrals Terms",
    url: "https://nym.com/referrals-terms-and-conditions",
  },
  {
    id: 5,
    title: "NymVPN apps Privacy statement",
    url: "https://nym.com/vpn-privacy-statement",
  },
  {
    id: 6,
    title: "Nym Operators and Validators Terms",
    url: "https://nym.com/operators-validators-terms",
  },
];

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
            {/* <Box
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
            </Box> */}
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

          {/* Hardcoded links */}

          <Box
            sx={{
              display: "flex",
              flexDirection: { xs: "column", md: "row" },
              gap: { xs: 3, md: 5 },
              mt: 9,
              mb: 6,
            }}
          >
            {links.map((link) => {
              return (
                <Box
                  sx={{
                    listStyle: "none",
                  }}
                  key={link.id}
                >
                  <Link
                    href={link.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    sx={{
                      textDecoration: "none",
                      "&:hover": {
                        textDecoration: "underline",
                      },
                    }}
                  >
                    <Typography variant="body5">{link.title}</Typography>
                  </Link>
                </Box>
              );
            })}
          </Box>

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

import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";

export default function OnboardingPage() {
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">Stake page</Typography>
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}

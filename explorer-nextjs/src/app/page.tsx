import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";

export default function Home() {
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">
              🚀 EXPLORER 2.0, Let&apos;s go! 🚀
            </Typography>
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}

import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";

export default function ExplorerPage() {
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">Explorer page</Typography>
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}

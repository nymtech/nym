import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";

export default function ExplorerPage() {
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">Explorer page</Typography>
            <ExplorerButtonGroup
              options={[
                {
                  label: "Node",
                  link: "/explorer",
                  isSelected: true,
                },
                {
                  label: "Account",
                  link: "/stake",
                  isSelected: false,
                },
              ]}
            />
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}
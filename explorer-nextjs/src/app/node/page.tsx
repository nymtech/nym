import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";

export default function NodePage() {
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">Node page</Typography>
            <ExplorerButtonGroup
              options={[
                {
                  label: "Node",
                  link: "/node",
                  isSelected: true,
                },
                {
                  label: "Account",
                  link: "/account",
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

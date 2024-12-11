import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Wrapper } from "@/components/wrapper";
import { Box, Typography } from "@mui/material";

export default function AccountPage() {
  return (
    <div>
      <main>
        <Box sx={{ p: 5 }}>
          <Wrapper>
            <Typography fontWeight="light">Account page</Typography>
            <ExplorerButtonGroup
              options={[
                {
                  label: "Node",
                  link: "/node",
                  isSelected: false,
                },
                {
                  label: "Account",
                  link: "/account",
                  isSelected: true,
                },
              ]}
            />
          </Wrapper>
        </Box>
      </main>
    </div>
  );
}

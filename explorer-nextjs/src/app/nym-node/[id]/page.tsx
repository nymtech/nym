import ExplorerCard from "@/components/cards/ExplorerCard";
import { ContentLayout } from "@/components/contentLayout/ContentLayout";
import SectionHeading from "@/components/headings/SectionHeading";
import ExplorerListItem from "@/components/list/ListItem";
import { StarRating } from "@/components/starRating";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { Box, Grid2, Stack } from "@mui/material";

export default function NymNode() {
  return (
    <ContentLayout component="main">
      <Grid2 container columnSpacing={5} rowSpacing={5}>
        <Grid2 size={6}>
          <SectionHeading title="Nym Node Details" />
        </Grid2>
        <Grid2 size={6} justifyContent="flex-end">
          <Box sx={{ display: "flex", justifyContent: "end" }}>
            <ExplorerButtonGroup
              options={[
                { label: "Nym Node", isSelected: true, link: "/nym-node/1" },
                {
                  label: "Account",
                  isSelected: false,
                  link: "/account/1",
                },
              ]}
            />
          </Box>
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard label="Action" sx={{ height: "100%" }}>
            <div />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard label="Basic info">
            <Stack gap={1}>
              <ExplorerListItem
                divider
                label="NYM Address"
                value="0x1234567890"
              />
              <ExplorerListItem
                divider
                label="Identity Key"
                value="0x1234567890"
              />
              <ExplorerListItem
                row
                divider
                label="Node bonded"
                value="24/11/2024"
              />
              <ExplorerListItem row divider label="Nr. of stakes" value="56" />
              <ExplorerListItem row label="Self bonded" value="10,000 NYM" />
            </Stack>
          </ExplorerCard>
        </Grid2>
        <Grid2 size={4}>
          <ExplorerCard
            label="Node Rewards (Last Epoch/Hour)"
            sx={{ height: "100%" }}
          >
            <ExplorerListItem row divider label="Role" value="Gateway" />
            <ExplorerListItem
              row
              divider
              label="Quality of service"
              value={<StarRating value={5} />}
            />
            <ExplorerListItem
              row
              divider
              label="Config score"
              value={<StarRating value={4} />}
            />
            <ExplorerListItem
              row
              divider
              label="Probe score"
              value={<StarRating value={5} />}
            />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={6}>
          <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
            <ExplorerListItem
              row
              divider
              label="Total rew."
              value="10,000 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Operator rew."
              value="10,000 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Staker rew."
              value="10,000 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Profit margin rew."
              value="40 NYM"
            />
            <ExplorerListItem
              row
              divider
              label="Operating cost."
              value="40 NYM"
            />
          </ExplorerCard>
        </Grid2>
        <Grid2 size={6}>
          <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
            <ExplorerListItem row divider label="Node ID." value="209" />
            <ExplorerListItem row divider label="Host" value="45.10.145.123" />
            <ExplorerListItem
              row
              divider
              label="Staker rew."
              value="10,000 NYM"
            />
            <ExplorerListItem row divider label="Version" value="1.1.1.1" />
            <ExplorerListItem
              row
              divider
              label="Active set Prob."
              value="High"
            />
          </ExplorerCard>
        </Grid2>
      </Grid2>
    </ContentLayout>
  );
}
